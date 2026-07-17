use adw::prelude::*;

mod board;
mod puzzle;

const APPLICATION_ID: &str = "io.github.tegarden.GnomeChessPuzzles";
const APPLICATION_NAME: &str = "Gnome Chess Puzzles";
const WINDOW_TITLE: &str = "Chess Puzzles";

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
    application.add_action_entries([quit, about]);

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
    primary_menu.append(Some("About Chess Puzzles"), Some("app.about"));

    let menu_button = adw::gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&primary_menu)
        .tooltip_text("Main Menu")
        .build();

    let header_bar = adw::HeaderBar::builder()
        .title_widget(&title)
        .decoration_layout(":minimize,maximize,close")
        .show_end_title_buttons(true)
        .build();
    header_bar.pack_end(&menu_button);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    match load_board() {
        Ok(board) => {
            toolbar_view.set_content(Some(&board));
        }
        Err(error) => {
            let error_page = adw::StatusPage::builder()
                .icon_name("dialog-error-symbolic")
                .title("Unable to Load Puzzle")
                .description(error.to_string())
                .build();
            toolbar_view.set_content(Some(&error_page));
        }
    }
    window.set_content(Some(&toolbar_view));

    window.present();
}

fn load_board() -> Result<board::Board, String> {
    let puzzle::Puzzle {
        id,
        mut initial_fen,
        setup_move,
    } = puzzle::load_placeholder().map_err(|error| error.to_string())?;

    initial_fen
        .apply_move(setup_move)
        .map_err(|error| format!("could not apply setup move for puzzle {id}: {error}"))?;
    let user_color = initial_fen.side_to_move();
    let board = board::Board::new(initial_fen, user_color);
    board.highlight_move(setup_move);
    Ok(board)
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
