use std::cell::Cell;
use std::rc::Rc;

use adw::gtk;
use gtk::prelude::*;

use crate::puzzle::{Color, NotatedMove};

#[derive(Clone)]
pub struct MoveList {
    grid: gtk::Grid,
    move_numbers: Rc<Vec<gtk::Label>>,
    white_moves: Rc<Vec<gtk::Label>>,
    black_moves: Rc<Vec<gtk::Label>>,
    setup_color: Color,
    incorrect_ply: Rc<Cell<Option<usize>>>,
}

impl MoveList {
    pub fn new(user_color: Color, total_plies: usize) -> Self {
        let setup_color = opposite(user_color);
        let rows = row_for_ply(total_plies - 1, setup_color) + 1;
        let grid = gtk::Grid::builder()
            .column_spacing(12)
            .row_spacing(6)
            .margin_top(30)
            .hexpand(true)
            .build();

        let header = gtk::Label::builder()
            .label("Moves:")
            .xalign(0.5)
            .hexpand(true)
            .build();
        grid.attach(&header, 0, 0, 3, 1);

        let mut move_numbers = Vec::with_capacity(rows);
        let mut white_moves = Vec::with_capacity(rows);
        let mut black_moves = Vec::with_capacity(rows);
        for row in 0..rows {
            let number = gtk::Label::builder().xalign(1.0).build();
            let white = move_label();
            let black = move_label();
            grid.attach(&number, 0, row as i32 + 1, 1, 1);
            grid.attach(&white, 1, row as i32 + 1, 1, 1);
            grid.attach(&black, 2, row as i32 + 1, 1, 1);
            move_numbers.push(number);
            white_moves.push(white);
            black_moves.push(black);
        }

        if setup_color == Color::Black {
            move_numbers[0].set_label("1.");
            white_moves[0].set_label("...");
        }

        Self {
            grid,
            move_numbers: Rc::new(move_numbers),
            white_moves: Rc::new(white_moves),
            black_moves: Rc::new(black_moves),
            setup_color,
            incorrect_ply: Rc::new(Cell::new(None)),
        }
    }

    pub fn widget(&self) -> &gtk::Grid {
        &self.grid
    }

    pub fn show_move(&self, played_move: &NotatedMove) {
        let row = row_for_ply(played_move.ply, self.setup_color);
        self.label_for(played_move)
            .set_label(&played_move.algebraic);
        if played_move.color == Color::White {
            self.move_numbers[row].set_label(&format!("{}.", row + 1));
        }
        if self.incorrect_ply.get() == Some(played_move.ply) {
            self.incorrect_ply.set(None);
        }
    }

    pub fn show_incorrect_move(&self, played_move: &NotatedMove) {
        self.show_move(played_move);
        self.incorrect_ply.set(Some(played_move.ply));
    }

    pub fn clear_incorrect_move(&self) {
        let Some(ply) = self.incorrect_ply.take() else {
            return;
        };
        let row = row_for_ply(ply, self.setup_color);
        let labels = if move_color(ply, self.setup_color) == Color::White {
            &self.white_moves
        } else {
            &self.black_moves
        };
        labels[row].set_label("");
        if move_color(ply, self.setup_color) == Color::White {
            self.move_numbers[row].set_label("");
        }
    }

    fn label_for(&self, played_move: &NotatedMove) -> &gtk::Label {
        let row = row_for_ply(played_move.ply, self.setup_color);
        match played_move.color {
            Color::White => &self.white_moves[row],
            Color::Black => &self.black_moves[row],
        }
    }
}

fn move_label() -> gtk::Label {
    gtk::Label::builder().xalign(0.0).hexpand(true).build()
}

fn opposite(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

fn move_color(ply: usize, setup_color: Color) -> Color {
    if ply % 2 == 0 {
        setup_color
    } else {
        opposite(setup_color)
    }
}

fn row_for_ply(ply: usize, setup_color: Color) -> usize {
    match setup_color {
        Color::White => ply / 2,
        Color::Black => (ply + 1) / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_setup_moves_share_rows_with_following_black_moves() {
        assert_eq!(row_for_ply(0, Color::White), 0);
        assert_eq!(row_for_ply(1, Color::White), 0);
        assert_eq!(row_for_ply(2, Color::White), 1);
    }

    #[test]
    fn black_setup_move_gets_its_own_first_row() {
        assert_eq!(row_for_ply(0, Color::Black), 0);
        assert_eq!(row_for_ply(1, Color::Black), 1);
        assert_eq!(row_for_ply(2, Color::Black), 1);
    }
}
