use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
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
    pub fn file(self) -> usize {
        self.file
    }

    pub fn rank(self) -> usize {
        self.rank
    }

    fn from_uci(value: &[u8]) -> Result<Self, MoveError> {
        if value.len() != 2
            || !(b'a'..=b'h').contains(&value[0])
            || !(b'1'..=b'8').contains(&value[1])
        {
            return Err(MoveError("invalid UCI square"));
        }

        Ok(Self {
            file: (value[0] - b'a') as usize,
            rank: (value[1] - b'1') as usize,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChessMove {
    pub from: Square,
    pub to: Square,
    promotion: Option<Role>,
}

impl ChessMove {
    pub fn from_uci(value: &str) -> Result<Self, MoveError> {
        let value = value.as_bytes();
        if value.len() != 4 && value.len() != 5 {
            return Err(MoveError("UCI move must contain four or five characters"));
        }

        let promotion = if value.len() == 5 {
            Some(match value[4] {
                b'n' => Role::Knight,
                b'b' => Role::Bishop,
                b'r' => Role::Rook,
                b'q' => Role::Queen,
                _ => return Err(MoveError("invalid UCI promotion role")),
            })
        } else {
            None
        };

        Ok(Self {
            from: Square::from_uci(&value[0..2])?,
            to: Square::from_uci(&value[2..4])?,
            promotion,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    squares: [Option<Piece>; 64],
    side_to_move: Color,
}

impl Position {
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let mut fields = fen.split_whitespace();
        let placement = fields.next().ok_or(FenError("missing piece placement"))?;
        let side_to_move = match fields.next() {
            Some("w") => Color::White,
            Some("b") => Color::Black,
            Some(_) => return Err(FenError("invalid side to move")),
            None => return Err(FenError("missing side to move")),
        };

        let ranks: Vec<_> = placement.split('/').collect();
        if ranks.len() != 8 {
            return Err(FenError("piece placement must contain eight ranks"));
        }

        let mut squares = [None; 64];
        for (fen_rank, contents) in ranks.into_iter().enumerate() {
            let rank = 7 - fen_rank;
            let mut file = 0;

            for symbol in contents.chars() {
                if let Some(empty) = symbol.to_digit(10) {
                    if !(1..=8).contains(&empty) {
                        return Err(FenError("invalid empty-square count"));
                    }
                    file += empty as usize;
                } else {
                    if file >= 8 {
                        return Err(FenError("rank contains too many squares"));
                    }
                    squares[rank * 8 + file] = Some(piece_from_fen(symbol)?);
                    file += 1;
                }
            }

            if file != 8 {
                return Err(FenError("rank must describe exactly eight squares"));
            }
        }

        Ok(Self {
            squares,
            side_to_move,
        })
    }

    pub fn piece_at(&self, file: usize, rank: usize) -> Option<Piece> {
        self.squares[rank * 8 + file]
    }

    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub fn apply_move(&mut self, chess_move: ChessMove) -> Result<(), MoveError> {
        let Some(mut piece) = self.piece_at(chess_move.from.file, chess_move.from.rank) else {
            return Err(MoveError("move starts on an empty square"));
        };
        if piece.color != self.side_to_move {
            return Err(MoveError(
                "moving piece does not belong to the side to move",
            ));
        }
        if self
            .piece_at(chess_move.to.file, chess_move.to.rank)
            .is_some_and(|target| target.color == piece.color)
        {
            return Err(MoveError("move ends on a friendly piece"));
        }

        let en_passant_capture = if piece.role == Role::Pawn
            && chess_move.from.file != chess_move.to.file
            && self
                .piece_at(chess_move.to.file, chess_move.to.rank)
                .is_none()
        {
            let captured_rank = match piece.color {
                Color::White => chess_move.to.rank.checked_sub(1),
                Color::Black => chess_move.to.rank.checked_add(1).filter(|rank| *rank < 8),
            }
            .ok_or(MoveError("invalid en passant destination"))?;
            let captured = self
                .piece_at(chess_move.to.file, captured_rank)
                .ok_or(MoveError("en passant move has no captured pawn"))?;
            if captured.color == piece.color || captured.role != Role::Pawn {
                return Err(MoveError(
                    "en passant move does not capture an opposing pawn",
                ));
            }
            Some(Square {
                file: chess_move.to.file,
                rank: captured_rank,
            })
        } else {
            None
        };

        let castling_rook =
            if piece.role == Role::King && chess_move.from.file.abs_diff(chess_move.to.file) == 2 {
                let (rook_file, rook_destination) = if chess_move.to.file > chess_move.from.file {
                    (7, 5)
                } else {
                    (0, 3)
                };
                let rook = self
                    .piece_at(rook_file, chess_move.from.rank)
                    .ok_or(MoveError("castling move has no rook"))?;
                if rook.color != piece.color || rook.role != Role::Rook {
                    return Err(MoveError("castling move has an invalid rook"));
                }
                Some((
                    Square {
                        file: rook_file,
                        rank: chess_move.from.rank,
                    },
                    Square {
                        file: rook_destination,
                        rank: chess_move.from.rank,
                    },
                    rook,
                ))
            } else {
                None
            };

        if let Some(promotion) = chess_move.promotion {
            if piece.role != Role::Pawn || !matches!(chess_move.to.rank, 0 | 7) {
                return Err(MoveError("only a pawn on its final rank can promote"));
            }
            piece.role = promotion;
        } else if piece.role == Role::Pawn && matches!(chess_move.to.rank, 0 | 7) {
            return Err(MoveError("pawn move to final rank requires promotion"));
        }

        self.set_piece(chess_move.from, None);
        if let Some(captured) = en_passant_capture {
            self.set_piece(captured, None);
        }
        if let Some((rook_from, rook_to, rook)) = castling_rook {
            self.set_piece(rook_from, None);
            self.set_piece(rook_to, Some(rook));
        }
        self.set_piece(chess_move.to, Some(piece));
        self.side_to_move = self.side_to_move.opposite();
        Ok(())
    }

    fn set_piece(&mut self, square: Square, piece: Option<Piece>) {
        self.squares[square.rank * 8 + square.file] = piece;
    }
}

#[derive(Debug)]
pub struct FenError(&'static str);

impl fmt::Display for FenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug)]
pub struct MoveError(&'static str);

impl fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

fn piece_from_fen(symbol: char) -> Result<Piece, FenError> {
    let color = if symbol.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    let role = match symbol.to_ascii_lowercase() {
        'p' => Role::Pawn,
        'n' => Role::Knight,
        'b' => Role::Bishop,
        'r' => Role::Rook,
        'q' => Role::Queen,
        'k' => Role::King,
        _ => return Err(FenError("invalid piece symbol")),
    };

    Ok(Piece { color, role })
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
    fn parses_promotions() {
        let chess_move = ChessMove::from_uci("a7a8q").unwrap();
        assert_eq!(chess_move.promotion, Some(Role::Queen));
    }
}
