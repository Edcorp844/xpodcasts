use adw::prelude::*;
use gettextrs::gettext;
use gtk::gdk::Texture;
use gtk::gio;
use podcasts_data::Episode;
use relm4::{prelude::*, Component, Controller};

use crate::{
    components::play_button::{EpisodePlayingState, PlayButton, PlayButtonInitData},
    util::{
        cover_image::{fetch_cached_image, ImageSize},
        gradient_extractor::GradientColorExtractor,
    },
};

#[derive(Debug)]
pub struct UnfinishedEpisodeCard {
    episode: Episode,
    texture: Option<Texture>,
    play_button_controller: Controller<PlayButton>,
}

#[derive(Debug, Clone)]
pub enum UnfinishedEpisodeCardInput {
    ImageDownloaded(Option<Texture>),
    MarkPlayed,
    RequestDelete,
    ConfirmDelete,
}

#[derive(Debug, Clone)]
pub enum UnfinishedEpisodeCardOutput {
    SetPlayNext(podcasts_data::EpisodeId),
    AddToPlaylist(podcasts_data::EpisodeId),
    GotoEpisode(podcasts_data::EpisodeId),
    RequestDownload(podcasts_data::EpisodeId),
    RequestDeleteEpisode(podcasts_data::EpisodeId),
}

#[derive(Debug, Clone)]
pub enum UnfinishedEpisodeCardCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for UnfinishedEpisodeCard {
    type Init = Episode;
    type Input = UnfinishedEpisodeCardInput;
    type Output = UnfinishedEpisodeCardOutput;
    type CommandOutput = UnfinishedEpisodeCardCmdInput;
    type ParentWidget = gtk::Box;

    fn init_model(episode: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        if let Some(image_url_ref) = episode.image_uri() {
            let image_url = image_url_ref.to_string();

            sender.oneshot_command(async move {
                let downloaded_texture =
                    fetch_cached_image(&image_url, ImageSize::from_dimesion(150)).await;
                UnfinishedEpisodeCardCmdInput::DownloadImage(downloaded_texture)
            });
        }

        let progress = match episode.duration() {
            Some(duration) if duration > 0 => {
                (episode.play_position() as f64 / duration as f64) * 100.0
            }
            _ => 0.0,
        };

        let pos_secs = episode.play_position() / 1000; // if stored as ms

        let progress_label = if pos_secs > 0 {
            let hours = pos_secs / 3600;
            let minutes = (pos_secs % 3600) / 60;
            let seconds = pos_secs % 60;

            if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else if minutes > 0 {
                format!("{}m", minutes)
            } else {
                format!("{}s", seconds)
            }
        } else {
            "0s".to_string()
        };

        let play_button_init_data = PlayButtonInitData {
            label: progress_label,
            state: EpisodePlayingState::Stopped,
            progress,
        };

        let play_button_controller = PlayButton::builder().launch(play_button_init_data).detach();

        UnfinishedEpisodeCard {
            episode,
            texture: None,
            play_button_controller,
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            UnfinishedEpisodeCardCmdInput::DownloadImage(opt_texture) => {
                sender.input(UnfinishedEpisodeCardInput::ImageDownloaded(opt_texture));
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: FactorySender<Self>,
    ) {
        match message {
            UnfinishedEpisodeCardInput::ImageDownloaded(texture) => {
                self.texture = texture;

                widgets.picture.set_paintable(
                    self.texture
                        .as_ref()
                        .map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                );
                widgets.picture.set_visible(self.texture.is_some());
                widgets.fallback_label.set_visible(self.texture.is_none());

                if let Some(texture) = &self.texture {
                    let css = format!(
                        "border-radius: 24px; \
                         box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32); \
                         border: 1px solid alpha(@borders, 0.8); {}",
                        GradientColorExtractor::extract_css_gradient_from_texture(texture)
                    );
                    widgets.card.inline_css(&css);
                }
            }
            UnfinishedEpisodeCardInput::MarkPlayed => {
                // no-op for now; hook into Save/EpisodeModel if you want
            }
            UnfinishedEpisodeCardInput::RequestDelete => {
                let root_window = relm4::main_adw_application()
                    .active_window()
                    .and_downcast::<gtk::Window>();

                let dialog = adw::AlertDialog::builder()
                    .heading("Delete Downloaded Episode?")
                    .body("This will remove the downloaded file from your device.")
                    .default_response("cancel")
                    .build();

                dialog.add_response("cancel", "Cancel");
                dialog.add_response("delete", "Delete");
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                let sender_clone = sender.clone();
                dialog.choose(
                    root_window.as_ref(),
                    None::<&gtk::gio::Cancellable>,
                    move |response| {
                        if response == "delete" {
                            sender_clone.input(UnfinishedEpisodeCardInput::ConfirmDelete);
                        }
                    },
                );
            }
            UnfinishedEpisodeCardInput::ConfirmDelete => {
                let _ = sender.output(UnfinishedEpisodeCardOutput::RequestDeleteEpisode(
                    self.episode.id(),
                ));
            }
        }
    }

    fn init_widgets(
        &mut self,
        _index: &Self::Index,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        // --- Register per-row actions, same pattern as EpisodeListItem ---
        let action_group = gio::SimpleActionGroup::new();

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let play_next_action = gio::SimpleAction::new("play-next", None);
        play_next_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(UnfinishedEpisodeCardOutput::SetPlayNext(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let add_to_playlist_action = gio::SimpleAction::new("add-to-playlist", None);
        add_to_playlist_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(UnfinishedEpisodeCardOutput::AddToPlaylist(id.clone()));
        });

        let sender_clone = sender.clone();
        let mark_played_action = gio::SimpleAction::new("mark-played", None);
        mark_played_action.connect_activate(move |_, _| {
            sender_clone.input(UnfinishedEpisodeCardInput::MarkPlayed);
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let goto_episode_action = gio::SimpleAction::new("goto-episode", None);
        goto_episode_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(UnfinishedEpisodeCardOutput::GotoEpisode(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let download_action = gio::SimpleAction::new("download", None);
        download_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(UnfinishedEpisodeCardOutput::RequestDownload(id.clone()));
        });

        let sender_clone = sender.clone();
        let delete_download_action = gio::SimpleAction::new("delete-download", None);
        delete_download_action.connect_activate(move |_, _| {
            sender_clone.input(UnfinishedEpisodeCardInput::RequestDelete);
        });

        action_group.add_action(&play_next_action);
        action_group.add_action(&add_to_playlist_action);
        action_group.add_action(&mark_played_action);
        action_group.add_action(&goto_episode_action);
        action_group.add_action(&download_action);
        action_group.add_action(&delete_download_action);

        widgets
            .menu_button
            .insert_action_group("episode", Some(&action_group));

        widgets
    }

    view! {
        #[name="card"]
        gtk::Overlay {
            set_height_request: 350,
            set_width_request: 225,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Start,
            inline_css: "border-radius: 24px; border: 1px solid alpha(@borders, 0.8);",

            // ---- Bottom layer: card content (gradient bg + labels) ----
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Start,

                // Spacer reserving the image area
                gtk::Box {
                    set_height_request: 170,
                    set_vexpand: false,
                    set_hexpand: true,
                },

                // ---- Title ----
                gtk::Label {
                    #[watch]
                    set_label: self.episode.title(),
                    set_valign: gtk::Align::Start,
                    set_halign: gtk::Align::Start,
                    set_xalign: 0.0,
                    set_margin_all: 16,
                    add_css_class: "caption",
                    inline_css: "color: white",
                    set_wrap: true,
                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_lines: 3,
                    set_height_request: 70,
                },

                // ---- Play button + menu button row ----
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_margin_horizontal: 16,
                    set_margin_bottom: 16,
                    set_halign: gtk::Align::Fill,

                    self.play_button_controller.widget() {
                        set_halign: gtk::Align::Start,
                        set_hexpand: true,
                        inline_css: "color: white",
                    },

                    gtk::MenuButton {
                        set_icon_name: "view-more-symbolic",
                        set_tooltip_text: Some(&gettext("Episode Options")),
                        set_css_classes: &["circular"],
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,

                        #[wrap(Some)]
                        #[name = "menu_button"]
                        set_popover = &gtk::PopoverMenu::from_model(Some(&{
                            let menu = gtk::gio::Menu::new();

                            let play_section = gtk::gio::Menu::new();
                            let play_next_item = gtk::gio::MenuItem::new(
                                Some(&gettext("Play Next")),
                                Some("episode.play-next"),
                            );
                            let mark_played_item = gtk::gio::MenuItem::new(
                                Some(&gettext("Mark as Played")),
                                Some("episode.mark-played"),
                            );
                            play_section.append_item(&play_next_item);
                            play_section.append_item(&mark_played_item);
                            menu.append_section(None, &play_section);

                            let playlist_section = gtk::gio::Menu::new();
                            let add_to_playlist_item = gtk::gio::MenuItem::new(
                                Some(&gettext("Add to Playlist")),
                                Some("episode.add-to-playlist"),
                            );
                            playlist_section.append_item(&add_to_playlist_item);
                            menu.append_section(None, &playlist_section);

                            let go_to_section = gtk::gio::Menu::new();
                            let goto_episode_item = gtk::gio::MenuItem::new(
                                Some(&gettext("Go to Episode")),
                                Some("episode.goto-episode"),
                            );
                            go_to_section.append_item(&goto_episode_item);
                            menu.append_section(None, &go_to_section);

                            let download_section = gtk::gio::Menu::new();
                            let download_episode_item = gtk::gio::MenuItem::new(
                                Some(&gettext("Download")),
                                Some("episode.download"),
                            );
                            download_section.append_item(&download_episode_item);
                            menu.append_section(None, &download_section);

                            menu
                        })) {}
                    }
                }
            },

            // ---- Top layer: 150x150 picture, top-centered ----
            add_overlay = &gtk::Box {
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_width_request: 150,
                set_height_request: 150,
                set_margin_top: 20,
                set_margin_start: 37,
                set_margin_end: 37,
                set_margin_bottom: 20,
                inline_css: "background-color: mix(var(--window-bg-color), var(--card-fg-color), 0.1); border-radius: 16px; box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32);",

                #[name="picture"]
                gtk::Picture {
                    set_visible: false,
                    set_content_fit: gtk::ContentFit::Contain,
                    set_can_shrink: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    inline_css: "border-radius: 16px;",
                },

                #[name="fallback_label"]
                gtk::Label {
                    set_visible: true,
                    #[watch]
                    set_label: &self.episode.title().trim().chars().take(2).collect::<String>().to_uppercase(),
                    set_css_classes: &["title-large", "dimmed"],
                    set_width_request: 100,
                    set_height_request: 100,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    inline_css: "opacity: 0.25; font-weight: 800;",
                }
            }
        }
    }
}