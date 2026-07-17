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
            let Some(geometry) = BoardGeometry::new(widget.width(), widget.height()) else {
                return;
            };

            let frame = gdk::RGBA::new(0.18, 0.20, 0.21, 1.0);
            let light_square = gdk::RGBA::new(0.93, 0.93, 0.92, 1.0);
            let dark_square = gdk::RGBA::new(0.73, 0.75, 0.72, 1.0);
            let highlighted_square = gdk::RGBA::new(0.96, 0.76, 0.18, 1.0);

            snapshot.append_color(
                &frame,
                &graphene::Rect::new(
                    geometry.board_x,
                    geometry.board_y,
                    geometry.board_size,
                    geometry.board_size,
                ),
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
                            geometry.squares_x + file as f32 * geometry.square_size,
                            geometry.squares_y + row as f32 * geometry.square_size,
                            geometry.square_size,
                            geometry.square_size,
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
                        geometry.squares_x,
                        geometry.squares_y,
                        geometry.square_size,
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

        let click = gtk::GestureClick::new();
        let weak_board = board.downgrade();
        click.connect_pressed(move |_, _, x, y| {
            if let Some(board) = weak_board.upgrade() {
                board.handle_click(x as f32, y as f32);
            }
        });
        board.add_controller(click);

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

    fn handle_click(&self, x: f32, y: f32) {
        let Some(perspective) = self.imp().perspective.get() else {
            return;
        };
        let Some(geometry) = BoardGeometry::new(self.width(), self.height()) else {
            return;
        };
        let Some(square) = geometry.square_at(x, y, perspective) else {
            return;
        };

        let origin = self.imp().highlights.borrow().selection;
        let action = {
            let position = self.imp().position.borrow();
            let Some(position) = position.as_ref() else {
                return;
            };
            user_action(position, perspective, origin, square)
        };

        match action {
            UserAction::Select(square) => self.highlight_origin(square),
            UserAction::Move(chess_move) => {
                let moved = self
                    .imp()
                    .position
                    .borrow_mut()
                    .as_mut()
                    .is_some_and(|position| position.apply_move(chess_move).is_ok());
                if moved {
                    self.highlight_move(chess_move);
                }
            }
            UserAction::Ignore => {}
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UserAction {
    Select(Square),
    Move(ChessMove),
    Ignore,
}

fn user_action(
    position: &Position,
    user_color: Color,
    selected: Option<Square>,
    clicked: Square,
) -> UserAction {
    if position.side_to_move() != user_color {
        return UserAction::Ignore;
    }

    if position
        .piece_at(clicked.file(), clicked.rank())
        .is_some_and(|piece| piece.color == user_color)
    {
        return UserAction::Select(clicked);
    }

    selected
        .and_then(|from| position.legal_move(from, clicked))
        .map_or(UserAction::Ignore, UserAction::Move)
}

#[derive(Clone, Copy)]
struct BoardGeometry {
    board_x: f32,
    board_y: f32,
    board_size: f32,
    squares_x: f32,
    squares_y: f32,
    square_size: f32,
}

impl BoardGeometry {
    fn new(width: i32, height: i32) -> Option<Self> {
        let available = width.min(height) as f32 - 2.0 * PADDING;
        let board_size = available.clamp(0.0, MAXIMUM_SIZE);
        if board_size == 0.0 {
            return None;
        }

        // The reference design uses a frame half as wide as a square.
        let square_size = board_size / 9.0;
        let frame_size = square_size / 2.0;
        let board_x = (width as f32 - board_size) / 2.0;
        let board_y = (height as f32 - board_size) / 2.0;
        Some(Self {
            board_x,
            board_y,
            board_size,
            squares_x: board_x + frame_size,
            squares_y: board_y + frame_size,
            square_size,
        })
    }

    fn square_at(self, x: f32, y: f32, perspective: Color) -> Option<Square> {
        let display_file = ((x - self.squares_x) / self.square_size).floor() as isize;
        let display_row = ((y - self.squares_y) / self.square_size).floor() as isize;
        if !(0..8).contains(&display_file) || !(0..8).contains(&display_row) {
            return None;
        }

        let (file, rank) = match perspective {
            Color::White => (display_file as usize, 7 - display_row as usize),
            Color::Black => (7 - display_file as usize, display_row as usize),
        };
        Square::from_coords(file, rank)
    }
}

#[derive(Default)]
struct Highlights {
    origin: Option<Square>,
    destination: Option<Square>,
    selection: Option<Square>,
}

impl Highlights {
    fn select_origin(&mut self, square: Square) {
        self.selection = Some(square);
    }

    fn show_move(&mut self, from: Square, to: Square) {
        self.origin = Some(from);
        self.destination = Some(to);
        self.selection = None;
    }

    fn clear(&mut self) {
        self.origin = None;
        self.destination = None;
        self.selection = None;
    }

    fn contains_display(&self, file: usize, row: usize, perspective: Color) -> bool {
        [self.origin, self.destination, self.selection]
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

        assert_eq!(highlights.selection, Some(second.from));
        assert_eq!(highlights.origin, None);
        assert_eq!(highlights.destination, None);
    }

    #[test]
    fn selecting_a_piece_preserves_the_opponents_move_highlights() {
        let opponent_move = ChessMove::from_uci("g8f6").unwrap();
        let selection = ChessMove::from_uci("e2e4").unwrap().from;
        let mut highlights = Highlights::default();

        highlights.show_move(opponent_move.from, opponent_move.to);
        highlights.select_origin(selection);

        assert_eq!(highlights.origin, Some(opponent_move.from));
        assert_eq!(highlights.destination, Some(opponent_move.to));
        assert_eq!(highlights.selection, Some(selection));
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
        assert_eq!(highlights.selection, None);
    }

    #[test]
    fn click_coordinates_follow_the_board_perspective() {
        let geometry = BoardGeometry::new(640, 640).unwrap();
        let x = geometry.squares_x + geometry.square_size / 2.0;
        let y = geometry.squares_y + 7.0 * geometry.square_size + geometry.square_size / 2.0;

        assert_eq!(
            geometry.square_at(x, y, Color::White),
            Square::from_coords(0, 0)
        );
        assert_eq!(
            geometry.square_at(x, y, Color::Black),
            Square::from_coords(7, 7)
        );
    }

    #[test]
    fn user_can_select_change_and_complete_a_legal_move() {
        let mut position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let e2e4 = ChessMove::from_uci("e2e4").unwrap();
        let d2d4 = ChessMove::from_uci("d2d4").unwrap();

        assert_eq!(
            user_action(&position, Color::White, None, e2e4.from),
            UserAction::Select(e2e4.from)
        );
        assert_eq!(
            user_action(&position, Color::White, Some(e2e4.from), d2d4.from),
            UserAction::Select(d2d4.from)
        );
        assert_eq!(
            user_action(&position, Color::White, Some(e2e4.from), e2e4.to),
            UserAction::Move(e2e4)
        );

        position.apply_move(e2e4).unwrap();
        assert_eq!(
            user_action(&position, Color::White, Some(e2e4.to), d2d4.from),
            UserAction::Ignore
        );
    }

    #[test]
    fn invalid_destination_keeps_the_current_selection() {
        let position =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let origin = ChessMove::from_uci("e2e4").unwrap().from;
        let invalid_destination = ChessMove::from_uci("e2e5").unwrap().to;

        assert_eq!(
            user_action(&position, Color::White, Some(origin), invalid_destination),
            UserAction::Ignore
        );
    }
}
