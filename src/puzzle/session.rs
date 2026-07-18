use std::fmt;

use super::{ChessMove, Position};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalState {
    Succeeded,
    SucceededAfterRetry,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    InProgress,
    Incorrect,
    Complete(TerminalState),
}

impl Progress {
    pub fn feedback_text(self) -> &'static str {
        match self {
            Self::Incorrect => "That move is not correct.",
            Self::Complete(TerminalState::Succeeded | TerminalState::SucceededAfterRetry) => {
                "You solved the puzzle."
            }
            Self::InProgress | Self::Complete(TerminalState::Failed) => "",
        }
    }

    pub fn retry_enabled(self) -> bool {
        self == Self::Incorrect
    }

    pub fn show_answer_enabled(self) -> bool {
        matches!(self, Self::InProgress | Self::Incorrect)
    }

    pub fn terminal_state(self) -> Option<TerminalState> {
        match self {
            Self::Complete(terminal_state) => Some(terminal_state),
            Self::InProgress | Self::Incorrect => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MoveOutcome {
    Incorrect {
        user_move: NotatedMove,
    },
    Correct {
        user_move: NotatedMove,
        opponent_move: Option<NotatedMove>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotatedMove {
    pub chess_move: ChessMove,
    pub color: super::Color,
    pub algebraic: String,
    pub ply: usize,
}

#[derive(Clone, Debug)]
pub struct AnswerStep {
    pub played_move: NotatedMove,
    pub position: Position,
}

pub struct PuzzleSession {
    position: Position,
    solution: Vec<ChessMove>,
    next_move: usize,
    made_mistake: bool,
    progress: Progress,
    retry_position: Option<Position>,
    last_opponent_move: ChessMove,
    setup_move: NotatedMove,
}

impl PuzzleSession {
    pub fn new(
        mut initial_position: Position,
        setup_move: ChessMove,
        solution: Vec<ChessMove>,
    ) -> Result<Self, SessionError> {
        if solution.is_empty() || solution.len() % 2 == 0 {
            return Err(SessionError(
                "a puzzle solution must contain an odd number of moves".into(),
            ));
        }
        let setup_move = NotatedMove {
            chess_move: setup_move,
            color: initial_position.side_to_move(),
            algebraic: initial_position
                .algebraic(setup_move)
                .map_err(|error| SessionError(format!("could not format setup move: {error}")))?,
            ply: 0,
        };
        initial_position
            .apply_move(setup_move.chess_move)
            .map_err(|error| SessionError(format!("could not apply setup move: {error}")))?;

        Ok(Self {
            position: initial_position,
            solution,
            next_move: 0,
            made_mistake: false,
            progress: Progress::InProgress,
            retry_position: None,
            last_opponent_move: setup_move.chess_move,
            setup_move,
        })
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn progress(&self) -> Progress {
        self.progress
    }

    pub fn last_opponent_move(&self) -> ChessMove {
        self.last_opponent_move
    }

    pub fn setup_move(&self) -> &NotatedMove {
        &self.setup_move
    }

    pub fn total_plies(&self) -> usize {
        self.solution.len() + 1
    }

    pub fn play_user_move(&mut self, user_move: ChessMove) -> Result<MoveOutcome, SessionError> {
        if self.progress != Progress::InProgress {
            return Err(SessionError(
                "the puzzle is not accepting a user move".into(),
            ));
        }

        let position_before_move = self.position.clone();
        let user_move = NotatedMove {
            chess_move: user_move,
            color: self.position.side_to_move(),
            algebraic: self
                .position
                .algebraic(user_move)
                .map_err(|error| SessionError(format!("could not format user move: {error}")))?,
            ply: self.next_move + 1,
        };
        self.position
            .apply_move(user_move.chess_move)
            .map_err(|error| SessionError(format!("could not apply user move: {error}")))?;

        if self.solution.get(self.next_move) != Some(&user_move.chess_move) {
            self.made_mistake = true;
            self.progress = Progress::Incorrect;
            self.retry_position = Some(position_before_move);
            return Ok(MoveOutcome::Incorrect { user_move });
        }

        self.next_move += 1;
        if self.next_move == self.solution.len() {
            self.progress = Progress::Complete(if self.made_mistake {
                TerminalState::SucceededAfterRetry
            } else {
                TerminalState::Succeeded
            });
            return Ok(MoveOutcome::Correct {
                user_move,
                opponent_move: None,
            });
        }

        let opponent_chess_move = self.solution[self.next_move];
        let opponent_move = NotatedMove {
            chess_move: opponent_chess_move,
            color: self.position.side_to_move(),
            algebraic: self
                .position
                .algebraic(opponent_chess_move)
                .map_err(|error| {
                    SessionError(format!("could not format opponent move: {error}"))
                })?,
            ply: self.next_move + 1,
        };
        self.position
            .apply_move(opponent_move.chess_move)
            .map_err(|error| SessionError(format!("could not apply opponent move: {error}")))?;
        self.next_move += 1;
        self.last_opponent_move = opponent_move.chess_move;

        Ok(MoveOutcome::Correct {
            user_move,
            opponent_move: Some(opponent_move),
        })
    }

    pub fn retry(&mut self) -> bool {
        if self.progress != Progress::Incorrect {
            return false;
        }

        let Some(position) = self.retry_position.take() else {
            return false;
        };
        self.position = position;
        self.progress = Progress::InProgress;
        true
    }

    pub fn show_answer(&mut self) -> Result<Vec<AnswerStep>, SessionError> {
        if !matches!(self.progress, Progress::InProgress | Progress::Incorrect) {
            return Err(SessionError("the puzzle is already complete".into()));
        }

        let mut answer_position = if self.progress == Progress::Incorrect {
            self.retry_position
                .clone()
                .ok_or_else(|| SessionError("the retry position is unavailable".into()))?
        } else {
            self.position.clone()
        };
        let remaining_moves = &self.solution[self.next_move..];
        if remaining_moves.is_empty() {
            return Err(SessionError(
                "the puzzle has no remaining answer moves".into(),
            ));
        }

        let mut steps = Vec::with_capacity(remaining_moves.len());
        for answer_move in remaining_moves {
            let played_move = NotatedMove {
                chess_move: *answer_move,
                color: answer_position.side_to_move(),
                algebraic: answer_position.algebraic(*answer_move).map_err(|error| {
                    SessionError(format!("could not format answer move: {error}"))
                })?,
                ply: steps.len() + self.next_move + 1,
            };
            answer_position
                .apply_move(*answer_move)
                .map_err(|error| SessionError(format!("could not apply answer move: {error}")))?;
            steps.push(AnswerStep {
                played_move,
                position: answer_position.clone(),
            });
        }

        self.position = answer_position;
        self.next_move = self.solution.len();
        self.progress = Progress::Complete(TerminalState::Failed);
        self.retry_position = None;
        Ok(steps)
    }
}

#[derive(Debug)]
pub struct SessionError(String);

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::{Color, Piece, Role};

    const STARTING_POSITION_BLACK_TO_MOVE: &str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";

    fn session(solution: &[&str]) -> PuzzleSession {
        PuzzleSession::new(
            Position::from_fen(STARTING_POSITION_BLACK_TO_MOVE).unwrap(),
            ChessMove::from_uci("e7e5").unwrap(),
            solution
                .iter()
                .map(|value| ChessMove::from_uci(value).unwrap())
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn correct_moves_apply_the_opponent_reply_and_complete_the_puzzle() {
        let mut session = session(&["g1f3", "b8c6", "d2d4"]);

        let outcome = session
            .play_user_move(ChessMove::from_uci("g1f3").unwrap())
            .unwrap();
        let MoveOutcome::Correct {
            user_move,
            opponent_move: Some(opponent_move),
        } = outcome
        else {
            panic!("expected a correct move with an opponent reply");
        };
        assert_eq!(user_move.algebraic, "Nf3");
        assert_eq!(opponent_move.algebraic, "Nc6");
        assert_eq!(session.progress(), Progress::InProgress);
        assert_eq!(
            session.position().piece_at(2, 5),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight,
            })
        );

        session
            .play_user_move(ChessMove::from_uci("d2d4").unwrap())
            .unwrap();
        assert_eq!(
            session.progress(),
            Progress::Complete(TerminalState::Succeeded)
        );
    }

    #[test]
    fn retry_restores_the_position_but_preserves_the_mistake() {
        let mut session = session(&["g1f3"]);
        let before_mistake = session.position().clone();

        let outcome = session
            .play_user_move(ChessMove::from_uci("b1c3").unwrap())
            .unwrap();
        assert!(matches!(outcome, MoveOutcome::Incorrect { .. }));
        assert_eq!(session.progress(), Progress::Incorrect);
        assert!(session.retry());
        assert_eq!(
            session.position().piece_at(1, 0),
            before_mistake.piece_at(1, 0)
        );

        session
            .play_user_move(ChessMove::from_uci("g1f3").unwrap())
            .unwrap();
        assert_eq!(
            session.progress(),
            Progress::Complete(TerminalState::SucceededAfterRetry)
        );
    }

    #[test]
    fn showing_the_answer_fails_and_disables_both_actions() {
        let mut session = session(&["g1f3", "b8c6", "d2d4"]);

        let steps = session.show_answer().unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(
            steps[0].played_move.chess_move,
            ChessMove::from_uci("g1f3").unwrap()
        );
        assert_eq!(steps[0].played_move.algebraic, "Nf3");
        assert_eq!(
            steps[1].played_move.chess_move,
            ChessMove::from_uci("b8c6").unwrap()
        );
        assert_eq!(steps[1].played_move.algebraic, "Nc6");
        assert_eq!(
            steps[2].played_move.chess_move,
            ChessMove::from_uci("d2d4").unwrap()
        );
        assert_eq!(steps[2].played_move.algebraic, "d4");
        assert_eq!(steps[0].position.piece_at(6, 0), None);
        assert_eq!(
            steps[1].position.piece_at(2, 5),
            Some(Piece {
                color: Color::Black,
                role: Role::Knight,
            })
        );

        let progress = session.progress();
        assert_eq!(progress, Progress::Complete(TerminalState::Failed));
        assert_eq!(progress.feedback_text(), "");
        assert!(!progress.retry_enabled());
        assert!(!progress.show_answer_enabled());
        assert_eq!(session.position().piece_at(6, 0), None);
        assert_eq!(
            session.position().piece_at(3, 3),
            Some(Piece {
                color: Color::White,
                role: Role::Pawn,
            })
        );
    }

    #[test]
    fn showing_the_answer_discards_an_incorrect_move_first() {
        let mut session = session(&["g1f3"]);
        session
            .play_user_move(ChessMove::from_uci("b1c3").unwrap())
            .unwrap();

        session.show_answer().unwrap();

        assert_eq!(
            session.position().piece_at(1, 0),
            Some(Piece {
                color: Color::White,
                role: Role::Knight,
            })
        );
        assert_eq!(session.position().piece_at(6, 0), None);
        assert_eq!(
            session.position().piece_at(5, 2),
            Some(Piece {
                color: Color::White,
                role: Role::Knight,
            })
        );
    }

    #[test]
    fn progress_exposes_the_incorrect_move_feedback() {
        assert_eq!(Progress::InProgress.feedback_text(), "");
        assert!(!Progress::InProgress.retry_enabled());
        assert!(Progress::InProgress.show_answer_enabled());
        assert_eq!(Progress::InProgress.terminal_state(), None);

        assert_eq!(
            Progress::Incorrect.feedback_text(),
            "That move is not correct."
        );
        assert!(Progress::Incorrect.retry_enabled());
        assert!(Progress::Incorrect.show_answer_enabled());
        assert_eq!(Progress::Incorrect.terminal_state(), None);

        for terminal_state in [TerminalState::Succeeded, TerminalState::SucceededAfterRetry] {
            let progress = Progress::Complete(terminal_state);
            assert_eq!(progress.feedback_text(), "You solved the puzzle.");
            assert!(!progress.retry_enabled());
            assert!(!progress.show_answer_enabled());
            assert_eq!(progress.terminal_state(), Some(terminal_state));
        }
    }
}
