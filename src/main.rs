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
    match load_puzzle_view() {
        Ok(puzzle_view) => {
            toolbar_view.set_content(Some(&puzzle_view));
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

fn load_puzzle_view() -> Result<adw::gtk::Box, String> {
    let puzzle::Puzzle {
        id,
        rating,
        mut initial_fen,
        setup_move,
    } = puzzle::load_placeholder().map_err(|error| error.to_string())?;

    initial_fen
        .apply_move(setup_move)
        .map_err(|error| format!("could not apply setup move for puzzle {id}: {error}"))?;
    let user_color = initial_fen.side_to_move();
    let board = board::Board::new(initial_fen, user_color);
    board.highlight_move(setup_move);

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

    Ok(puzzle_view)
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
