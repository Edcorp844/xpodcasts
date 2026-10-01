use adw::prelude::*;
use gettextrs::{gettext, ngettext};
use relm4::prelude::*;

use crate::{config, settings::GenaralSettings, util::external_controls::ExternalControlsMode};

pub struct AppMenu {}

impl AppMenu {
    pub fn register() {
        let app = relm4::main_application();

        let refresh_action = gtk::gio::SimpleAction::new("refresh", None);
        refresh_action.connect_activate(move |_, _| {
            println!("Refresh requested");
        });
        app.add_action(&refresh_action);
        app.set_accels_for_action("app.refresh", &["<Primary>r"]);

        let refresh_episodes_action = gtk::gio::SimpleAction::new("refresh_episodes", None);
        refresh_episodes_action.connect_activate(move |_, _| {
            println!("Refresh episodes requested");
        });
        app.add_action(&refresh_episodes_action);
        app.set_accels_for_action("app.refresh_episodes", &["<Primary><Shift>r"]);

        let preferences_action = gtk::gio::SimpleAction::new("preferences", None);
        preferences_action.connect_activate(move |_, _| {
            Self::show_preferences_window();
        });

        app.add_action(&preferences_action);
        app.set_accels_for_action("app.preferences", &["<Primary>comma"]);

        let shortcuts_action = gtk::gio::SimpleAction::new("shortcuts", None);
        shortcuts_action.connect_activate(move |_, _| {
            Self::show_shortcuts_window();
        });
        app.add_action(&shortcuts_action);
        app.set_accels_for_action("app.shortcuts", &["<Primary>question"]);

        let about_action = gtk::gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            Self::show_about_window();
        });
        app.add_action(&about_action);

        let quit_action = gtk::gio::SimpleAction::new("quit", None);
        let app_clone = app.clone();
        quit_action.connect_activate(move |_, _| {
            app_clone.quit();
        });
        app.add_action(&quit_action);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }

    pub(crate) fn show_about_window() {
        let gnome_podcasts_developers = vec![
            "Alexandre Franke",
            "Carlos Soriano",
            "Constantin Nickel",
            "Daniel García Moreno",
            "Felix Häcker",
            "Gabriele Musco",
            "Ivan Augusto",
            "James Wykeham-Martin",
            "Jordan Petridis",
            "Jordan Williams",
            "Julian Hofer",
            "Julian Sparber",
            "Matthew Martin",
            "Piotr Drąg",
            "Rowan Lewis",
            "Zander Brown",
        ];

        if let Some(active_window) = relm4::main_application().active_window() {
            let about = adw::AboutDialog::builder()
                .application_name("XPodcasts")
                .application_icon(config::APPLICATION_ID)
                .comments(gettext("Podcast Client for the GNOME Desktop.").as_str())
                .version(config::VERSION)
                .developer_name("Edson Frost")
                .website("https://github.com/Edcorp844/gnome_podcasts.git")
                .issue_url("https://github.com/Edcorp844/gnome_podcasts/issues")
                .copyright("© 2026 Edson Frost")
                .license_type(gtk::License::Gpl30)
                .developers(vec!["Frost Edson"])
                .artists(vec!["Frost Edson"])
                .build();

            about.add_acknowledgement_section(
                Some("GNOME Podcasts Data Library"),
                &gnome_podcasts_developers,
            );

            about.present(Some(&active_window));
        }
    }

    pub(crate) fn show_shortcuts_window() {
        if let Some(active_window) = relm4::main_application().active_window() {
            let shortcuts_window = adw::ShortcutsDialog::builder()
                .title("Keyboard Shortcuts")
                .width_request(600)
                .height_request(500)
                .build();

            // --- SECTION: Window ---
            let window_section = adw::ShortcutsSection::new(Some("Window"));

            let refresh = adw::ShortcutsItem::new("Refresh", "<Primary>r");
            refresh.set_subtitle("Refresh all views content");

            let refresh_episodes = adw::ShortcutsItem::new("Refresh Episodes", "<Primary><Shift>r");
            refresh_episodes.set_subtitle("Refresh the database and new episodes");
            window_section.add(refresh);
            window_section.add(refresh_episodes);

            let quit = adw::ShortcutsItem::new("Quit", "<Primary>q");
            quit.set_subtitle("Close the application");
            window_section.add(quit);

            // --- SECTION: Application ---
            let application_section = adw::ShortcutsSection::new(Some("Application"));

            let prefs = adw::ShortcutsItem::new("Preferences", "<Primary>comma");
            prefs.set_subtitle("Configure application preferences");
            application_section.add(prefs);

            let shorts = adw::ShortcutsItem::new("Shortcuts", "<Primary>question");
            shorts.set_subtitle("Shows shortcuts window");
            application_section.add(shorts);

            // --- SECTION: Navigation ---
            let nav_section = adw::ShortcutsSection::new(Some("Navigation"));

            let search = adw::ShortcutsItem::new("Search", "<Primary>f");
            search.set_subtitle("Search Podcasts, Episodes, Shows");
            nav_section.add(search);

            // Add sections to the dialog
            shortcuts_window.add(window_section);
            shortcuts_window.add(application_section);
            shortcuts_window.add(nav_section);

            shortcuts_window.present(Some(&active_window));
        }
    }

    pub(crate) fn show_preferences_window() {
        let general_settings = GenaralSettings::new();
        if let Some(active_window) = relm4::main_application().active_window() {
            let preferences_window = adw::PreferencesDialog::builder()
                .search_enabled(true)
                .build();

            let general_page = adw::PreferencesPage::builder()
                .title("General")
                .icon_name("preferences-system-symbolic")
                .build();

            let library_group = adw::PreferencesGroup::builder().title("Library").build();

            let auto_sync_row = adw::SwitchRow::builder()
                .title("Auto sync")
                .subtitle("Automatically update followed shows")
                .build();

            general_settings
                .settings
                .bind("library-auto-sync", &auto_sync_row, "active")
                .build();

            library_group.add(&auto_sync_row);
            general_page.add(&library_group);

            let search_platforms_group = adw::PreferencesGroup::builder()
                .title("Search platforms")
                .description("Search queries will be sent to these platforms.")
                .build();

            for id in podcasts_data::discovery::ALL_PLATFORM_IDS {
                let is_active = general_settings
                    .get_search_platforms()
                    .iter()
                    .any(|p| p == id);

                let display_name = match id {
                    "fyyd.de" => gettext("Fyyd"),
                    "itunes.apple.com" => gettext("Apple Podcasts"),
                    other => other.to_string(),
                };

                let search_platform_row = adw::SwitchRow::builder()
                    .title(display_name)
                    .active(is_active)
                    .build();

                search_platforms_group.add(&search_platform_row);

                let settings = general_settings.clone();
                let id = id.to_string(); // still the raw id, used for storage — not translated

                search_platform_row.connect_active_notify(move |row| {
                    settings.toggle_search_platform(&id, row.is_active());
                });
            }
            general_page.add(&search_platforms_group);

            let player_page = adw::PreferencesPage::builder()
                .title("Player")
                .icon_name("prefs-tweaks-symbolic")
                .build();

            let play_back_group = adw::PreferencesGroup::builder()
                .title("Playback")
                .description("Configure standard playback behavior.")
                .build();

            let auto_play_row = adw::SwitchRow::builder()
                .title("Continuous Playback")
                .subtitle("Continue playing after an episode ends.")
                .build();

            general_settings
                .settings
                .bind("continuous-playback", &auto_play_row, "active")
                .build();
            play_back_group.add(&auto_play_row);

            player_page.add(&play_back_group);

            let skip_buttons_group = adw::PreferencesGroup::builder()
                .title("Skip Buttons")
                .description("Set the number of seconds to skip when you tap the skip buttons.")
                .build();

            let skip_model = gtk::StringList::new(&[]);
            let mut values = Vec::new();

            for i in 1..=8 {
                let number = if i <= 3 { i * 5 } else { (i - 2) * 15 };
                let template = ngettext("{} second", "{} seconds", number as u32);
                let label = template.replace("{}", &number.to_string());
                skip_model.append(&label);
                values.push(number);
            }

            // --- Forward ---
            let current_value = general_settings.settings.int("skip-foward-seconds");
            let selected_index =
                values.iter().position(|&v| v == current_value).unwrap_or(0) as u32;

            let skip_forward_row = adw::ComboRow::builder()
                .title(gettext("Forward"))
                .model(&skip_model)
                .selected(selected_index)
                .build();

            skip_buttons_group.add(&skip_forward_row);

            let settings = general_settings.clone();
            let forward_values = values.clone();
            skip_forward_row.connect_selected_notify(move |row| {
                let idx = row.selected() as usize;
                if let Some(&value) = forward_values.get(idx) {
                    let _ = settings.settings.set_int("skip-foward-seconds", value);
                }
            });

            // --- Backward ---
            let current_value = general_settings.settings.int("skip-backward-seconds");
            let selected_index =
                values.iter().position(|&v| v == current_value).unwrap_or(0) as u32;

            let skip_backward_row = adw::ComboRow::builder()
                .title(gettext("Backward"))
                .model(&skip_model)
                .selected(selected_index)
                .build();

            skip_buttons_group.add(&skip_backward_row);

            let settings = general_settings.clone();
            let backward_values = values;
            skip_backward_row.connect_selected_notify(move |row| {
                let idx = row.selected() as usize;
                if let Some(&value) = backward_values.get(idx) {
                    let _ = settings.settings.set_int("skip-backward-seconds", value);
                }
            });

            player_page.add(&skip_buttons_group);

            let external_controls_group = adw::PreferencesGroup::builder()
                .title("External Controls")
                .description("Set what external controls like headphones should do")
                .build();

            let labels: Vec<String> = ExternalControlsMode::ALL
                .iter()
                .map(|m| m.display_name())
                .collect();
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            let external_controls_model = gtk::StringList::new(&label_refs);

            let current_mode = general_settings.get_external_controls_mode();

            let external_controls_row = adw::ComboRow::builder()
                .title(gettext("External Controls"))
                .model(&external_controls_model)
                .selected(current_mode.index())
                .build();

            external_controls_group.add(&external_controls_row);

            let settings = general_settings.clone();
            external_controls_row.connect_selected_notify(move |row| {
                let mode = ExternalControlsMode::from_index(row.selected());
                settings.set_external_controls_mode(mode);
            });
            player_page.add(&external_controls_group);

            preferences_window.add(&general_page);
            preferences_window.add(&player_page);

            preferences_window.present(Some(&active_window));
        }
    }
}
