use std::cell::{Cell, OnceCell, RefCell};

use adw::gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene};

use crate::puzzle::{ChessMove, Color, Piece, Position, Role, Square};

const BLACK_BISHOP: &[u8] = include_bytes!("../../data/pieces/fancy/blackBishop.png");
const BLACK_KING: &[u8] = include_bytes!("../../data/pieces/fancy/blackKing.png");
const BLACK_KNIGHT: &[u8] = include_bytes!("../../data/pieces/fancy/blackKnight.png");
const BLACK_PAWN: &[u8] = include_bytes!("../../data/pieces/fancy/blackPawn.png");
const BLACK_QUEEN: &[u8] = include_bytes!("../../data/pieces/fancy/blackQueen.png");
const BLACK_ROOK: &[u8] = include_bytes!("../../data/pieces/fancy/blackRook.png");
const WHITE_BISHOP: &[u8] = include_bytes!("../../data/pieces/fancy/whiteBishop.png");
const WHITE_KING: &[u8] = include_bytes!("../../data/pieces/fancy/whiteKing.png");
const WHITE_KNIGHT: &[u8] = include_bytes!("../../data/pieces/fancy/whiteKnight.png");
const WHITE_PAWN: &[u8] = include_bytes!("../../data/pieces/fancy/whitePawn.png");
const WHITE_QUEEN: &[u8] = include_bytes!("../../data/pieces/fancy/whiteQueen.png");
const WHITE_ROOK: &[u8] = include_bytes!("../../data/pieces/fancy/whiteRook.png");

const MINIMUM_SIZE: i32 = 256;
const NATURAL_SIZE: i32 = 640;
const MAXIMUM_SIZE: f32 = 720.0;
const PADDING: f32 = 24.0;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Board {
        pub(super) position: RefCell<Option<Position>>,
        pub(super) perspective: Cell<Option<Color>>,
        pub(super) highlights: RefCell<Highlights>,
        pub(super) textures: PieceTextures,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Board {
        const NAME: &'static str = "GcpBoard";
        type Type = super::Board;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Board {}

    impl WidgetImpl for Board {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (MINIMUM_SIZE, NATURAL_SIZE, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let available = widget.width().min(widget.height()) as f32 - 2.0 * PADDING;
            let board_size = available.clamp(0.0, MAXIMUM_SIZE);

            if board_size == 0.0 {
                return;
            }

            // The reference design uses a frame half as wide as a square.
            let square_size = board_size / 9.0;
            let frame_size = square_size / 2.0;
            let board_x = (widget.width() as f32 - board_size) / 2.0;
            let board_y = (widget.height() as f32 - board_size) / 2.0;
            let squares_x = board_x + frame_size;
            let squares_y = board_y + frame_size;

            let frame = gdk::RGBA::new(0.18, 0.20, 0.21, 1.0);
            let light_square = gdk::RGBA::new(0.93, 0.93, 0.92, 1.0);
            let dark_square = gdk::RGBA::new(0.73, 0.75, 0.72, 1.0);
            let highlighted_square = gdk::RGBA::new(0.96, 0.76, 0.18, 1.0);

            snapshot.append_color(
                &frame,
                &graphene::Rect::new(board_x, board_y, board_size, board_size),
            );

            let perspective = self.perspective.get().unwrap_or(Color::White);
            let highlights = self.highlights.borrow();
            for row in 0..8 {
                for file in 0..8 {
                    let color = if highlights.contains_display(file, row, perspective) {
                        &highlighted_square
                    } else if (row + file) % 2 == 0 {
                        &light_square
                    } else {
                        &dark_square
                    };
                    snapshot.append_color(
                        color,
                        &graphene::Rect::new(
                            squares_x + file as f32 * square_size,
                            squares_y + row as f32 * square_size,
                            square_size,
                            square_size,
                        ),
                    );
                }
            }

            let position = self.position.borrow();
            let Some(position) = position.as_ref() else {
                return;
            };
            for rank in 0..8 {
                for file in 0..8 {
                    let Some(piece) = position.piece_at(file, rank) else {
                        continue;
                    };
                    let (display_file, display_row) = display_coordinates(file, rank, perspective);
                    append_piece(
                        snapshot,
                        self.textures.texture(piece),
                        squares_x,
                        squares_y,
                        square_size,
                        display_file,
                        display_row,
                    );
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
        @extends gtk::Widget,
        @implements gtk::Buildable, gtk::ConstraintTarget;
}

impl Board {
    pub fn new(position: Position, perspective: Color) -> Self {
        let board: Self = glib::Object::builder()
            .property("hexpand", true)
            .property("vexpand", true)
            .build();
        board.set_position(position);
        board.imp().perspective.set(Some(perspective));
        board
    }

    pub fn set_position(&self, position: Position) {
        self.imp().position.replace(Some(position));
        self.queue_draw();
    }

    pub fn highlight_origin(&self, square: Square) {
        self.imp().highlights.borrow_mut().select_origin(square);
        self.queue_draw();
    }

    pub fn highlight_move(&self, chess_move: ChessMove) {
        self.imp()
            .highlights
            .borrow_mut()
            .show_move(chess_move.from, chess_move.to);
        self.queue_draw();
    }

    pub fn clear_highlights(&self) {
        self.imp().highlights.borrow_mut().clear();
        self.queue_draw();
    }
}

#[derive(Default)]
struct Highlights {
    origin: Option<Square>,
    destination: Option<Square>,
}

impl Highlights {
    fn select_origin(&mut self, square: Square) {
        self.origin = Some(square);
        self.destination = None;
    }

    fn show_move(&mut self, from: Square, to: Square) {
        self.origin = Some(from);
        self.destination = Some(to);
    }

    fn clear(&mut self) {
        self.origin = None;
        self.destination = None;
    }

    fn contains_display(&self, file: usize, row: usize, perspective: Color) -> bool {
        [self.origin, self.destination]
            .into_iter()
            .flatten()
            .any(|square| {
                display_coordinates(square.file(), square.rank(), perspective) == (file, row)
            })
    }
}

#[derive(Default)]
struct PieceTextures {
    black_bishop: OnceCell<gdk::Texture>,
    black_king: OnceCell<gdk::Texture>,
    black_knight: OnceCell<gdk::Texture>,
    black_pawn: OnceCell<gdk::Texture>,
    black_queen: OnceCell<gdk::Texture>,
    black_rook: OnceCell<gdk::Texture>,
    white_bishop: OnceCell<gdk::Texture>,
    white_king: OnceCell<gdk::Texture>,
    white_knight: OnceCell<gdk::Texture>,
    white_pawn: OnceCell<gdk::Texture>,
    white_queen: OnceCell<gdk::Texture>,
    white_rook: OnceCell<gdk::Texture>,
}

impl PieceTextures {
    fn texture(&self, piece: Piece) -> &gdk::Texture {
        let (cell, png) = match (piece.color, piece.role) {
            (Color::Black, Role::Bishop) => (&self.black_bishop, BLACK_BISHOP),
            (Color::Black, Role::King) => (&self.black_king, BLACK_KING),
            (Color::Black, Role::Knight) => (&self.black_knight, BLACK_KNIGHT),
            (Color::Black, Role::Pawn) => (&self.black_pawn, BLACK_PAWN),
            (Color::Black, Role::Queen) => (&self.black_queen, BLACK_QUEEN),
            (Color::Black, Role::Rook) => (&self.black_rook, BLACK_ROOK),
            (Color::White, Role::Bishop) => (&self.white_bishop, WHITE_BISHOP),
            (Color::White, Role::King) => (&self.white_king, WHITE_KING),
            (Color::White, Role::Knight) => (&self.white_knight, WHITE_KNIGHT),
            (Color::White, Role::Pawn) => (&self.white_pawn, WHITE_PAWN),
            (Color::White, Role::Queen) => (&self.white_queen, WHITE_QUEEN),
            (Color::White, Role::Rook) => (&self.white_rook, WHITE_ROOK),
        };
        cell.get_or_init(|| texture_from_png(png))
    }
}

fn texture_from_png(png: &'static [u8]) -> gdk::Texture {
    gdk::Texture::from_bytes(&glib::Bytes::from_static(png))
        .expect("embedded chess piece must be a valid PNG image")
}

fn display_coordinates(file: usize, rank: usize, user_color: Color) -> (usize, usize) {
    match user_color {
        Color::White => (file, 7 - rank),
        Color::Black => (7 - file, rank),
    }
}

fn append_piece(
    snapshot: &gtk::Snapshot,
    texture: &gdk::Texture,
    board_x: f32,
    board_y: f32,
    square_size: f32,
    file: usize,
    row: usize,
) {
    snapshot.append_texture(
        texture,
        &graphene::Rect::new(
            board_x + file as f32 * square_size,
            board_y + row as f32 * square_size,
            square_size,
            square_size,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_has_rank_one_at_bottom() {
        assert_eq!(display_coordinates(0, 0, Color::White), (0, 7));
        assert_eq!(display_coordinates(7, 7, Color::White), (7, 0));
    }

    #[test]
    fn black_has_rank_eight_at_bottom() {
        assert_eq!(display_coordinates(0, 0, Color::Black), (7, 0));
        assert_eq!(display_coordinates(7, 7, Color::Black), (0, 7));
    }

    #[test]
    fn selecting_another_origin_removes_the_old_highlight() {
        let first = ChessMove::from_uci("e2e4").unwrap();
        let second = ChessMove::from_uci("d2d4").unwrap();
        let mut highlights = Highlights::default();

        highlights.select_origin(first.from);
        highlights.select_origin(second.from);

        assert_eq!(highlights.origin, Some(second.from));
        assert_eq!(highlights.destination, None);
    }

    #[test]
    fn completed_move_replaces_previous_move_highlights() {
        let first = ChessMove::from_uci("e2e4").unwrap();
        let second = ChessMove::from_uci("g8f6").unwrap();
        let mut highlights = Highlights::default();

        highlights.show_move(first.from, first.to);
        highlights.show_move(second.from, second.to);

        assert_eq!(highlights.origin, Some(second.from));
        assert_eq!(highlights.destination, Some(second.to));
    }
}
