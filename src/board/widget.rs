use std::cell::OnceCell;

use adw::gtk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene};

const WHITE_KING: &[u8] = include_bytes!("../../data/pieces/fancy/whiteKing.png");
const BLACK_KING: &[u8] = include_bytes!("../../data/pieces/fancy/blackKing.png");

const MINIMUM_SIZE: i32 = 256;
const NATURAL_SIZE: i32 = 640;
const MAXIMUM_SIZE: f32 = 720.0;
const PADDING: f32 = 24.0;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Board {
        white_king: OnceCell<gdk::Texture>,
        black_king: OnceCell<gdk::Texture>,
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

            snapshot.append_color(
                &frame,
                &graphene::Rect::new(board_x, board_y, board_size, board_size),
            );

            for row in 0..8 {
                for file in 0..8 {
                    let color = if (row + file) % 2 == 0 {
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

            let black_king = self.black_king.get_or_init(|| texture_from_png(BLACK_KING));
            let white_king = self.white_king.get_or_init(|| texture_from_png(WHITE_KING));

            // Black starts on e8; White starts on e1.
            append_piece(
                snapshot,
                black_king,
                squares_x,
                squares_y,
                square_size,
                4,
                0,
            );
            append_piece(
                snapshot,
                white_king,
                squares_x,
                squares_y,
                square_size,
                4,
                7,
            );
        }
    }
}

glib::wrapper! {
    pub struct Board(ObjectSubclass<imp::Board>)
        @extends gtk::Widget,
        @implements gtk::Buildable, gtk::ConstraintTarget;
}

impl Board {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("hexpand", true)
            .property("vexpand", true)
            .build()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

fn texture_from_png(png: &'static [u8]) -> gdk::Texture {
    gdk::Texture::from_bytes(&glib::Bytes::from_static(png))
        .expect("embedded chess piece must be a valid PNG image")
}

fn append_piece(
    snapshot: &gtk::Snapshot,
    texture: &gdk::Texture,
    board_x: f32,
    board_y: f32,
    square_size: f32,
    file: u8,
    row: u8,
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
