use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;

mod board;
mod history;
mod move_list;
mod puzzle;

const APPLICATION_ID: &str = "io.github.tegarden.gnome-chess-puzzles";
const APPLICATION_NAME: &str = "Chess Puzzles";
const WINDOW_TITLE: &str = "Chess Puzzles";
const MOVE_PLAYBACK_DELAY: Duration = Duration::from_millis(500);

fn main() -> adw::glib::ExitCode {
    adw::glib::set_application_name(APPLICATION_NAME);

    let application = adw::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_activate(build_ui);
    application.set_accels_for_action("app.quit", &["<Control>q"]);

    let quit = adw::gio::ActionEntry::builder("quit")
        .activate(|application: &adw::Application, _, _| application.quit())
        .build();
    let about = adw::gio::ActionEntry::builder("about")
        .activate(|application: &adw::Application, _, _| show_about_dialog(application))
        .build();
    let history = adw::gio::ActionEntry::builder("history")
        .activate(|application: &adw::Application, _, _| history::show_window(application))
        .build();
    application.add_action_entries([quit, history, about]);

    application.run()
}

fn build_ui(application: &adw::Application) {
    if let Some(window) = application.active_window() {
        window.present();
        return;
    }

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title(WINDOW_TITLE)
        .default_width(900)
        .default_height(650)
        .build();

    let title = adw::WindowTitle::builder().title(WINDOW_TITLE).build();

    let primary_menu = adw::gio::Menu::new();
    primary_menu.append(Some("History"), Some("app.history"));
    primary_menu.append(Some("About Chess Puzzles"), Some("app.about"));

    let menu_button = adw::gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&primary_menu)
        .tooltip_text("Main Menu")
        .build();

    let new_puzzle_button = adw::gtk::Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text("New Puzzle")
        .build();

    let header_bar = adw::HeaderBar::builder()
        .title_widget(&title)
        .decoration_layout(":minimize,maximize,close")
        .show_end_title_buttons(true)
        .build();
    header_bar.pack_start(&new_puzzle_button);
    header_bar.pack_end(&menu_button);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    let current_puzzle_id = Rc::new(RefCell::new(None));
    match load_puzzle_view(None) {
        Ok(loaded) => {
            current_puzzle_id.replace(Some(loaded.id));
            toolbar_view.set_content(Some(&loaded.widget));
        }
        Err(error) => show_load_error(&toolbar_view, &error),
    }

    let weak_toolbar_view = toolbar_view.downgrade();
    let current_puzzle_id_for_button = Rc::clone(&current_puzzle_id);
    new_puzzle_button.connect_clicked(move |_| {
        let Some(toolbar_view) = weak_toolbar_view.upgrade() else {
            return;
        };
        let current_id = current_puzzle_id_for_button.borrow().clone();
        match load_puzzle_view(current_id.as_deref()) {
            Ok(loaded) => {
                current_puzzle_id_for_button.replace(Some(loaded.id));
                toolbar_view.set_content(Some(&loaded.widget));
            }
            Err(error) => show_load_error(&toolbar_view, &error),
        }
    });

    window.set_content(Some(&toolbar_view));

    window.present();
}

struct LoadedPuzzleView {
    id: String,
    widget: adw::gtk::Box,
}

fn load_puzzle_view(excluded_puzzle_id: Option<&str>) -> Result<LoadedPuzzleView, String> {
    let mut selection = history::puzzle_selection_state().map_err(|error| error.to_string())?;
    if let Some(puzzle_id) = excluded_puzzle_id {
        selection.completed_puzzle_ids.insert(puzzle_id.to_owned());
    }
    let puzzle::Puzzle {
        id,
        rating,
        rating_deviation,
        initial_fen,
        setup_move,
        solution,
    } = puzzle::load_for_player(selection.player_rating, &selection.completed_puzzle_ids)
        .map_err(|error| error.to_string())?;

    let board_initial_position = initial_fen.clone();
    let session = puzzle::PuzzleSession::new(initial_fen, setup_move, solution)
        .map_err(|error| format!("could not start puzzle {id}: {error}"))?;
    let user_color = session.position().side_to_move();
    let move_list = move_list::MoveList::new(user_color, session.total_plies());
    let setup_notated_move = session.setup_move().clone();
    let board = board::Board::new(board_initial_position, user_color);
    board.set_input_enabled(false);
    let session = Rc::new(RefCell::new(session));

    let heading = adw::gtk::Label::builder()
        .label(format!("Puzzle {id} (rating {rating})"))
        .xalign(0.0)
        .wrap(true)
        .build();
    heading.add_css_class("title-3");

    let side_to_move = match user_color {
        puzzle::Color::White => "White to move",
        puzzle::Color::Black => "Black to move",
    };
    let side_to_move = adw::gtk::Label::builder()
        .label(side_to_move)
        .xalign(0.0)
        .build();

    let feedback_content = adw::gtk::Box::new(adw::gtk::Orientation::Vertical, 6);
    feedback_content.set_margin_top(18);
    feedback_content.set_margin_bottom(18);
    feedback_content.set_margin_start(18);
    feedback_content.set_margin_end(18);
    feedback_content.append(&heading);
    feedback_content.append(&side_to_move);
    feedback_content.append(move_list.widget());

    let feedback_spacer = adw::gtk::Box::new(adw::gtk::Orientation::Vertical, 0);
    feedback_spacer.set_vexpand(true);
    feedback_content.append(&feedback_spacer);

    let progress_text = adw::gtk::Label::builder()
        .label("")
        .hexpand(true)
        .xalign(0.0)
        .wrap(true)
        .build();

    let retry_button = adw::gtk::Button::builder()
        .label("Retry")
        .sensitive(false)
        .hexpand(true)
        .build();
    let show_answer_button = adw::gtk::Button::builder()
        .label("Show Answer")
        .sensitive(false)
        .hexpand(true)
        .build();

    let progress_buttons = adw::gtk::Box::new(adw::gtk::Orientation::Horizontal, 6);
    progress_buttons.set_homogeneous(true);
    progress_buttons.append(&retry_button);
    progress_buttons.append(&show_answer_button);

    let progress_area = adw::gtk::Box::new(adw::gtk::Orientation::Vertical, 12);
    progress_area.append(&progress_text);
    progress_area.append(&progress_buttons);
    feedback_content.append(&progress_area);

    let weak_board = board.downgrade();
    let session_for_move = Rc::clone(&session);
    let progress_text_for_move = progress_text.clone();
    let retry_button_for_move = retry_button.clone();
    let show_answer_button_for_move = show_answer_button.clone();
    let move_list_for_move = move_list.clone();
    let puzzle_id_for_move = id.clone();
    board.connect_user_move(move |user_move| {
        let Some(board) = weak_board.upgrade() else {
            return;
        };
        let outcome = match session_for_move.borrow_mut().play_user_move(user_move) {
            Ok(outcome) => outcome,
            Err(error) => {
                eprintln!("could not record user move: {error}");
                board.set_input_enabled(false);
                return;
            }
        };

        let waiting_for_opponent = match outcome {
            puzzle::MoveOutcome::Incorrect { user_move } => {
                move_list_for_move.show_incorrect_move(&user_move);
                board.set_input_enabled(false);
                false
            }
            puzzle::MoveOutcome::Correct {
                user_move,
                opponent_move: Some(opponent_move),
            } => {
                move_list_for_move.show_move(&user_move);
                let opponent_position = session_for_move.borrow().position().clone();
                board.set_input_enabled(false);
                let weak_board = board.downgrade();
                let show_answer_button_weak = show_answer_button_for_move.downgrade();
                let move_list = move_list_for_move.clone();
                adw::glib::timeout_add_local_once(MOVE_PLAYBACK_DELAY, move || {
                    let Some(board) = weak_board.upgrade() else {
                        return;
                    };
                    board.set_position(opponent_position);
                    board.highlight_move(opponent_move.chess_move);
                    move_list.show_move(&opponent_move);
                    board.set_input_enabled(true);
                    if let Some(show_answer_button) = show_answer_button_weak.upgrade() {
                        show_answer_button.set_sensitive(true);
                    }
                });
                true
            }
            puzzle::MoveOutcome::Correct {
                user_move,
                opponent_move: None,
            } => {
                move_list_for_move.show_move(&user_move);
                board.set_input_enabled(false);
                false
            }
        };

        let rating_update = session_for_move
            .borrow()
            .progress()
            .terminal_state()
            .and_then(|result| {
                match history::record(&puzzle_id_for_move, rating, rating_deviation, result) {
                    Ok(update) => Some(update),
                    Err(error) => {
                        eprintln!("could not record completed puzzle: {error}");
                        None
                    }
                }
            });
        update_progress_controls(
            session_for_move.borrow().progress(),
            rating_update,
            &board,
            &progress_text_for_move,
            &retry_button_for_move,
            &show_answer_button_for_move,
        );
        if waiting_for_opponent {
            show_answer_button_for_move.set_sensitive(false);
        }
    });

    let weak_board = board.downgrade();
    let session_for_retry = Rc::clone(&session);
    let progress_text_for_retry = progress_text.clone();
    let show_answer_button_weak = show_answer_button.downgrade();
    let move_list_for_retry = move_list.clone();
    retry_button.connect_clicked(move |retry_button| {
        let Some(board) = weak_board.upgrade() else {
            return;
        };
        if !session_for_retry.borrow_mut().retry() {
            return;
        }

        let session = session_for_retry.borrow();
        board.set_position(session.position().clone());
        board.highlight_move(session.last_opponent_move());
        move_list_for_retry.clear_incorrect_move();
        board.set_input_enabled(true);
        if let Some(show_answer_button) = show_answer_button_weak.upgrade() {
            update_progress_controls(
                session.progress(),
                None,
                &board,
                &progress_text_for_retry,
                retry_button,
                &show_answer_button,
            );
        }
    });

    let weak_board = board.downgrade();
    let session_for_answer = Rc::clone(&session);
    let progress_text_for_answer = progress_text.clone();
    let retry_button_weak = retry_button.downgrade();
    let move_list_for_answer = move_list.clone();
    let puzzle_id_for_answer = id.clone();
    show_answer_button.connect_clicked(move |show_answer_button| {
        let Some(board) = weak_board.upgrade() else {
            return;
        };
        let answer_steps = match session_for_answer.borrow_mut().show_answer() {
            Ok(answer_steps) => answer_steps,
            Err(error) => {
                eprintln!("could not show puzzle answer: {error}");
                return;
            }
        };

        board.set_input_enabled(false);
        play_answer_steps(&board, &move_list_for_answer, answer_steps);
        let rating_update = match history::record(
            &puzzle_id_for_answer,
            rating,
            rating_deviation,
            puzzle::TerminalState::Failed,
        ) {
            Ok(update) => Some(update),
            Err(error) => {
                eprintln!("could not record completed puzzle: {error}");
                None
            }
        };
        if let Some(retry_button) = retry_button_weak.upgrade() {
            update_progress_controls(
                session_for_answer.borrow().progress(),
                rating_update,
                &board,
                &progress_text_for_answer,
                &retry_button,
                show_answer_button,
            );
        }
    });

    let feedback_panel = adw::gtk::Frame::builder()
        .child(&feedback_content)
        .width_request(280)
        .vexpand(true)
        .build();
    feedback_panel.add_css_class("card");
    feedback_panel.set_margin_top(24);
    feedback_panel.set_margin_bottom(24);
    feedback_panel.set_margin_end(24);

    let puzzle_view = adw::gtk::Box::new(adw::gtk::Orientation::Horizontal, 0);
    puzzle_view.append(&board);
    puzzle_view.append(&feedback_panel);

    let setup_position = session.borrow().position().clone();
    let weak_board = board.downgrade();
    let show_answer_button_weak = show_answer_button.downgrade();
    let move_list_for_setup = move_list.clone();
    adw::glib::timeout_add_local_once(MOVE_PLAYBACK_DELAY, move || {
        let Some(board) = weak_board.upgrade() else {
            return;
        };
        board.set_position(setup_position);
        board.highlight_move(setup_move);
        move_list_for_setup.show_move(&setup_notated_move);
        board.set_input_enabled(true);
        if let Some(show_answer_button) = show_answer_button_weak.upgrade() {
            show_answer_button.set_sensitive(true);
        }
    });

    Ok(LoadedPuzzleView {
        id,
        widget: puzzle_view,
    })
}

fn play_answer_steps(
    board: &board::Board,
    move_list: &move_list::MoveList,
    steps: Vec<puzzle::AnswerStep>,
) {
    for (index, step) in steps.into_iter().enumerate() {
        let weak_board = board.downgrade();
        let move_list = move_list.clone();
        let delay =
            Duration::from_millis(MOVE_PLAYBACK_DELAY.as_millis() as u64 * (index as u64 + 1));
        adw::glib::timeout_add_local_once(delay, move || {
            let Some(board) = weak_board.upgrade() else {
                return;
            };
            board.set_position(step.position);
            board.highlight_move(step.played_move.chess_move);
            move_list.show_move(&step.played_move);
        });
    }
}

fn show_load_error(toolbar_view: &adw::ToolbarView, error: &str) {
    let error_page = adw::StatusPage::builder()
        .icon_name("dialog-error-symbolic")
        .title("Unable to Load Puzzle")
        .description(error)
        .build();
    toolbar_view.set_content(Some(&error_page));
}

fn update_progress_controls(
    progress: puzzle::Progress,
    rating_update: Option<history::RatingUpdate>,
    board: &board::Board,
    progress_text: &adw::gtk::Label,
    retry_button: &adw::gtk::Button,
    show_answer_button: &adw::gtk::Button,
) {
    let border_state = match progress.terminal_state() {
        None => board::BorderState::InProgress,
        Some(puzzle::TerminalState::Succeeded) => board::BorderState::Succeeded,
        Some(puzzle::TerminalState::SucceededAfterRetry) => board::BorderState::SucceededAfterRetry,
        Some(puzzle::TerminalState::Failed) => board::BorderState::Failed,
    };
    board.set_border_state(border_state);
    progress_text.set_label(&progress_feedback_text(progress, rating_update));
    retry_button.set_sensitive(progress.retry_enabled());
    show_answer_button.set_sensitive(progress.show_answer_enabled());
}

fn progress_feedback_text(
    progress: puzzle::Progress,
    rating_update: Option<history::RatingUpdate>,
) -> String {
    match (progress, rating_update) {
        (puzzle::Progress::Complete(terminal_state), Some(rating_update)) => {
            let result = match terminal_state {
                puzzle::TerminalState::Succeeded => "You solved the puzzle.",
                puzzle::TerminalState::SucceededAfterRetry => "You solved the puzzle with retries.",
                puzzle::TerminalState::Failed => "You did not solve the puzzle.",
            };
            format!(
                "{result}\nYour rating is now {} ({:+}).",
                rating_update.rating, rating_update.change
            )
        }
        _ => progress.feedback_text().to_owned(),
    }
}

fn show_about_dialog(application: &adw::Application) {
    let Some(window) = application.active_window() else {
        return;
    };

    let dialog = adw::AboutDialog::builder()
        .application_name(WINDOW_TITLE)
        .version(env!("CARGO_PKG_VERSION"))
        .comments("Practice chess with tactical puzzles")
        .license_type(adw::gtk::License::Gpl30)
        .website("https://github.com/tegarden/gnome-chess-puzzles")
        .issue_url("https://github.com/tegarden/gnome-chess-puzzles/issues")
        .build();

    dialog.present(Some(&window));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_feedback_includes_rating_and_signed_change() {
        let cases = [
            (
                puzzle::TerminalState::Succeeded,
                history::RatingUpdate {
                    rating: 503,
                    change: 3,
                },
                "You solved the puzzle.\nYour rating is now 503 (+3).",
            ),
            (
                puzzle::TerminalState::SucceededAfterRetry,
                history::RatingUpdate {
                    rating: 498,
                    change: -5,
                },
                "You solved the puzzle with retries.\nYour rating is now 498 (-5).",
            ),
            (
                puzzle::TerminalState::Failed,
                history::RatingUpdate {
                    rating: 498,
                    change: -5,
                },
                "You did not solve the puzzle.\nYour rating is now 498 (-5).",
            ),
        ];

        for (terminal_state, rating_update, expected) in cases {
            assert_eq!(
                progress_feedback_text(
                    puzzle::Progress::Complete(terminal_state),
                    Some(rating_update)
                ),
                expected
            );
        }
    }

    #[test]
    fn in_progress_feedback_is_unchanged() {
        assert_eq!(
            progress_feedback_text(puzzle::Progress::InProgress, None),
            ""
        );
        assert_eq!(
            progress_feedback_text(puzzle::Progress::Incorrect, None),
            "That move is not correct."
        );
    }
}
