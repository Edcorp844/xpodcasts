use adw::prelude::*;
use gettextrs::gettext;
use gtk::gdk::Texture;
use gtk::gio;
use podcasts_data::Episode;
use relm4::{prelude::*, Component, Controller};

use crate::{
    components::play_button::{EpisodePlayingState, PlayButton, PlayButtonInitData},
    util::cover_image::{fetch_cached_image, ImageSize},
};

#[derive(Debug)]
pub struct EpisodeSectionTile {
    episode: Episode,
    texture: Option<Texture>,
    play_button_controller: Controller<PlayButton>,
}

#[derive(Debug, Clone)]
pub enum EpisodeSectionTileInput {
    ImageDownloaded(Option<Texture>),
    MarkPlayed,
    RequestDelete,
    ConfirmDelete,
}

#[derive(Debug, Clone)]
pub enum EpisodeSectionTileOutput {
    SetPlayNext(podcasts_data::EpisodeId),
    AddToPlaylist(podcasts_data::EpisodeId),
    GotoEpisode(podcasts_data::EpisodeId),
    RequestDownload(podcasts_data::EpisodeId),
    RequestDeleteEpisode(podcasts_data::EpisodeId),
}

#[derive(Debug, Clone)]
pub enum EpisodeSectionTileCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for EpisodeSectionTile {
    type Init = Episode;
    type Input = EpisodeSectionTileInput;
    type Output = EpisodeSectionTileOutput;
    type CommandOutput = EpisodeSectionTileCmdInput;
    type ParentWidget = gtk::Box;

    fn init_model(episode: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        if let Some(image_url_ref) = episode.image_uri() {
            let image_url = image_url_ref.to_string();

            sender.oneshot_command(async move {
                let downloaded_texture =
                    fetch_cached_image(&image_url, ImageSize::new(Some(100), Some(100))).await;
                EpisodeSectionTileCmdInput::DownloadImage(downloaded_texture)
            });
        }

        let progress_label = if let Some(duration) = episode.duration() {
            let duration_secs = duration;

            if duration_secs > 0 {
                let hours = duration_secs / 3600;
                let minutes = (duration_secs % 3600) / 60;
                let seconds = duration_secs % 60;

                if hours > 0 {
                    format!("{}hr {}min", hours, minutes)
                } else if minutes > 0 {
                    format!("{}min", minutes)
                } else {
                    format!("{}sec", seconds)
                }
            } else {
                "0sec".to_string()
            }
        } else {
            "0s".to_string()
        };

        let play_button_init_data = PlayButtonInitData {
            label: progress_label,
            state: EpisodePlayingState::Stopped,
            progress: 0.0,
        };

        let play_button_controller = PlayButton::builder().launch(play_button_init_data).detach();

        EpisodeSectionTile {
            episode,
            texture: None,
            play_button_controller,
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            EpisodeSectionTileCmdInput::DownloadImage(opt_texture) => {
                sender.input(EpisodeSectionTileInput::ImageDownloaded(opt_texture));
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
            EpisodeSectionTileInput::ImageDownloaded(texture) => {
                self.texture = texture;

                widgets.picture.set_paintable(
                    self.texture
                        .as_ref()
                        .map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                );
                widgets.picture.set_visible(self.texture.is_some());
            }
            EpisodeSectionTileInput::RequestDelete => {
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
                            sender_clone.input(EpisodeSectionTileInput::ConfirmDelete);
                        }
                    },
                );
            }
            EpisodeSectionTileInput::ConfirmDelete => {
                let _ = sender.output(EpisodeSectionTileOutput::RequestDeleteEpisode(
                    self.episode.id(),
                ));
            }
            EpisodeSectionTileInput::MarkPlayed => {}
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

        let action_group = gio::SimpleActionGroup::new();

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let play_next_action = gio::SimpleAction::new("play-next", None);
        play_next_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeSectionTileOutput::SetPlayNext(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let add_to_playlist_action = gio::SimpleAction::new("add-to-playlist", None);
        add_to_playlist_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeSectionTileOutput::AddToPlaylist(id.clone()));
        });

        let sender_clone = sender.clone();
        let mark_played_action = gio::SimpleAction::new("mark-played", None);
        mark_played_action.connect_activate(move |_, _| {
            sender_clone.input(EpisodeSectionTileInput::MarkPlayed);
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let goto_episode_action = gio::SimpleAction::new("goto-episode", None);
        goto_episode_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeSectionTileOutput::GotoEpisode(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let download_action = gio::SimpleAction::new("download", None);
        download_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeSectionTileOutput::RequestDownload(id.clone()));
        });

        let sender_clone = sender.clone();
        let delete_download_action = gio::SimpleAction::new("delete-download", None);
        delete_download_action.connect_activate(move |_, _| {
            sender_clone.input(EpisodeSectionTileInput::RequestDelete);
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
        gtk::Box {
            set_width_request: 450,
            set_height_request: 140,
            set_hexpand: false,
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 16,
            set_valign: gtk::Align::Start,

            gtk::Overlay {
                set_height_request: 100,
                set_width_request: 100,
                set_halign: gtk::Align::Start,
                set_valign: gtk::Align::Start,
                set_hexpand: false,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    add_css_class: "frame",
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    inline_css: "background-color: mix(var(--window-bg-color), var(--card-fg-color), 0.1); border-radius: 8px; box-shadow: 0 6px 14px rgba(0, 0, 0, 0.32);",

                    gtk::Label {
                        #[watch]
                        set_label: &self.episode.title().trim().chars().take(2).collect::<String>().to_uppercase(),
                        set_css_classes: &vec!["title-large", "dimmed"],
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        inline_css: "opacity: 0.25; font-weight: 800;",
                    }
                },

                #[name = "picture"]
                add_overlay = &gtk::Picture {
                    #[watch]
                    set_paintable: self.texture.as_ref().map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                    #[watch]
                    set_visible: self.texture.is_some(),
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    set_content_fit: gtk::ContentFit::Cover,
                    set_can_shrink: true,
                    inline_css: "border-radius: 8px;",
                }
            },


            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_halign: gtk::Align::Fill,
                set_valign: gtk::Align::Start,
                set_hexpand: true,


                gtk::Label {
                    set_label: self.episode.title(),
                    add_css_class: "caption-heading",
                    set_halign: gtk::Align::Fill,
                    set_xalign: 0.0,
                    set_wrap: true,
                    set_wrap_mode: gtk::pango::WrapMode::WordChar,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_lines: 2,
                    set_height_request: 40,
                },


                gtk::Box {
                    set_vexpand: true,
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::End,

                    self.play_button_controller.widget(),

                    gtk::Box {
                        set_hexpand: true,
                    },

                    gtk::MenuButton {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,
                        set_icon_name: "view-more-symbolic",
                        set_css_classes: &["circular"],

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
                    },
                },

                gtk::Separator{},
            },
        }
    }
}
