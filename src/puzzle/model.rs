use std::fmt;

use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::uci::UciMove;
use shakmaty::{
    CastlingMode, Chess, Color as ShakmatyColor, File, Position as ShakmatyPosition, Rank,
    Role as ShakmatyRole, Square as ShakmatySquare,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Piece {
    pub color: Color,
    pub role: Role,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Square {
    file: usize,
    rank: usize,
}

impl Square {
    pub(crate) fn from_coords(file: usize, rank: usize) -> Option<Self> {
        (file < 8 && rank < 8).then_some(Self { file, rank })
    }

    pub fn file(self) -> usize {
        self.file
    }

    pub fn rank(self) -> usize {
        self.rank
    }

    fn from_shakmaty(square: ShakmatySquare) -> Self {
        Self {
            file: square.file().to_u32() as usize,
            rank: square.rank().to_u32() as usize,
        }
    }

    fn to_shakmaty(self) -> ShakmatySquare {
        ShakmatySquare::from_coords(File::new(self.file as u32), Rank::new(self.rank as u32))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChessMove {
    pub from: Square,
    pub to: Square,
    uci: UciMove,
}

impl ChessMove {
    pub fn from_uci(value: &str) -> Result<Self, MoveError> {
        let uci: UciMove = value
            .parse()
            .map_err(|error| MoveError(format!("invalid UCI move: {error}")))?;
        Self::from_uci_move(uci)
    }

    fn from_uci_move(uci: UciMove) -> Result<Self, MoveError> {
        let from = uci
            .from()
            .map(Square::from_shakmaty)
            .ok_or_else(|| MoveError("move must have an origin square".into()))?;
        let to = uci
            .to()
            .map(Square::from_shakmaty)
            .ok_or_else(|| MoveError("move must have a destination square".into()))?;
        Ok(Self { from, to, uci })
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    inner: Chess,
}

impl Position {
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let fen: Fen = fen
            .parse()
            .map_err(|error| FenError(format!("invalid FEN: {error}")))?;
        let inner = fen
            .into_position(CastlingMode::Standard)
            .map_err(|error| FenError(format!("invalid chess position: {error}")))?;
        Ok(Self { inner })
    }

    pub fn piece_at(&self, file: usize, rank: usize) -> Option<Piece> {
        self.inner
            .board()
            .piece_at(Square { file, rank }.to_shakmaty())
            .map(piece_from_shakmaty)
    }

    pub fn side_to_move(&self) -> Color {
        color_from_shakmaty(self.inner.turn())
    }

    pub fn legal_move(&self, from: Square, to: Square) -> Option<ChessMove> {
        let mut fallback = None;
        for chess_move in self.inner.legal_moves() {
            let uci = UciMove::from_standard(chess_move);
            if uci.from() == Some(from.to_shakmaty()) && uci.to() == Some(to.to_shakmaty()) {
                let parsed = ChessMove::from_uci_move(uci).ok()?;
                if uci.promotion() == Some(ShakmatyRole::Queen) {
                    return Some(parsed);
                }
                fallback = Some(parsed);
            }
        }
        fallback
    }

    pub fn apply_move(&mut self, chess_move: ChessMove) -> Result<(), MoveError> {
        let legal_move = chess_move
            .uci
            .to_move(&self.inner)
            .map_err(|error| MoveError(format!("illegal move: {error}")))?;
        self.inner.play_unchecked(legal_move);
        Ok(())
    }

    pub fn algebraic(&self, chess_move: ChessMove) -> Result<String, MoveError> {
        let legal_move = chess_move
            .uci
            .to_move(&self.inner)
            .map_err(|error| MoveError(format!("illegal move: {error}")))?;
        Ok(SanPlus::from_move(self.inner.clone(), legal_move).to_string())
    }
}

#[derive(Debug)]
pub struct FenError(String);

impl fmt::Display for FenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug)]
pub struct MoveError(String);

impl fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn color_from_shakmaty(color: ShakmatyColor) -> Color {
    match color {
        ShakmatyColor::White => Color::White,
        ShakmatyColor::Black => Color::Black,
    }
}

fn role_from_shakmaty(role: ShakmatyRole) -> Role {
    match role {
        ShakmatyRole::Pawn => Role::Pawn,
        ShakmatyRole::Knight => Role::Knight,
        ShakmatyRole::Bishop => Role::Bishop,
        ShakmatyRole::Rook => Role::Rook,
        ShakmatyRole::Queen => Role::Queen,
        ShakmatyRole::King => Role::King,
    }
}

fn piece_from_shakmaty(piece: shakmaty::Piece) -> Piece {
    Piece {
        color: color_from_shakmaty(piece.color),
        role: role_from_shakmaty(piece.role),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_starting_position() {
        let position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        assert_eq!(
            position.piece_at(4, 0),
            Some(Piece {
                color: Color::White,
                role: Role::King,
            })
        );
        assert_eq!(
            position.piece_at(4, 7),
            Some(Piece {
                color: Color::Black,
                role: Role::King,
            })
        );
        assert_eq!(position.side_to_move(), Color::White);
    }

    #[test]
    fn rejects_incomplete_rank() {
        assert!(Position::from_fen("7/8/8/8/8/8/8/8 w - - 0 1").is_err());
    }

    #[test]
    fn applies_the_placeholder_setup_move() {
        let mut position =
            Position::from_fen("2kr1b1r/p1p2pp1/2pqb3/7p/3N2n1/2NPB3/PPP2PPP/R2Q1RK1 w - - 2 13")
                .unwrap();
        let setup_move = ChessMove::from_uci("d4e6").unwrap();

        position.apply_move(setup_move).unwrap();

        assert_eq!(position.piece_at(3, 3), None);
        assert_eq!(
            position.piece_at(4, 5),
            Some(Piece {
                color: Color::White,
                role: Role::Knight,
            })
        );
        assert_eq!(position.side_to_move(), Color::Black);
    }

    #[test]
    fn finds_legal_moves_and_rejects_illegal_destinations() {
        let position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let legal = ChessMove::from_uci("e2e4").unwrap();
        let illegal = ChessMove::from_uci("e2e5").unwrap();

        assert_eq!(position.legal_move(legal.from, legal.to), Some(legal));
        assert_eq!(position.legal_move(illegal.from, illegal.to), None);
    }

    #[test]
    fn formats_moves_in_standard_algebraic_notation() {
        let position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

        assert_eq!(
            position
                .algebraic(ChessMove::from_uci("g1f3").unwrap())
                .unwrap(),
            "Nf3"
        );
    }
}
