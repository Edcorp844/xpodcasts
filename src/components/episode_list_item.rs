use adw::prelude::*;
use gettextrs::gettext;
use gst_play::PlayState;
use gtk::gio;
use podcasts_data::{Episode, EpisodeId, EpisodeModel, Save, dbqueries};
use relm4::{
    Component, ComponentController, Controller, FactorySender, RelmWidgetExt,
    factory::{DynamicIndex, FactoryComponent},
};

use crate::{
    components::{
        circular_progress::{CircularProgress, CircularProgressInput},
        play_button::{
            self, EpisodePlayingState, PlayButton, PlayButtonInitData, PlayButtonInput,
            PlayButtonOutput,
        },
    },
    util::{
        cover_image::{ImageSize, fetch_cached_image},
        episode_description_parser,
    },
};

#[derive(Debug)]
pub struct EpisodeListItem {
    episode: Episode,
    texture: Option<adw::gdk::Texture>,
    play_button: Controller<PlayButton>,
    downloaded: bool,
    downloading: bool,
    progress_indicator: Controller<CircularProgress>,
}

#[derive(Debug, Clone)]
pub enum EpisodeListItemInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
    MarkPlayed,
    TogglePlay,
    DownloadStarted,
    PlayBackProgress(f64, u64),
    DownloadProgress(f64),
    CancleDownload,
    DownloadCancled,
    RequestDownload,
    DownloadFinished,
    RequestDelete,
    ChangePlayBackState(PlayState),
    ChangeEpisodeTo(EpisodeId),
    EpisodeDeleted,
    ConfirmDelete,
}

#[derive(Debug)]
pub enum EpisodeListItemOutput {
    TogglePlay(EpisodeId),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    GotoEpisode(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    NotifyError(String),
}

#[derive(Debug)]
pub enum EpisodeListItemCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for EpisodeListItem {
    type Init = Episode;
    type Input = EpisodeListItemInput;
    type Output = EpisodeListItemOutput;
    type CommandOutput = EpisodeListItemCmdInput;
    type ParentWidget = gtk::ListBox;

    fn init_model(episode: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let clone = episode.clone();

        if let Some(image_url_ref) = clone.image_uri() {
            let image_url = image_url_ref.to_string();

            sender.oneshot_command(async move {
                let downloaded_texture = fetch_cached_image(&image_url, ImageSize::default()).await;

                EpisodeListItemCmdInput::DownloadImage(downloaded_texture)
            });
        }

        let duration_str = match episode.duration() {
            Some(seconds) if seconds > 0 => {
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;

                if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                }
            }
            _ => "0m".to_string(),
        };

        let play_button = PlayButton::builder()
            .launch(PlayButtonInitData {
                label: duration_str,
                state: play_button::EpisodePlayingState::Stopped,
                progress: 0.0,
            })
            .forward(sender.input_sender(), |msg| match msg {
                PlayButtonOutput::Clicked => EpisodeListItemInput::TogglePlay,
            });

        let downloaded = {
            if let Ok(episode_widget) = dbqueries::get_episode_widget_from_id(episode.id()) {
                episode_widget.local_uri().is_some()
            } else {
                false
            }
        };

        let progress_indicator = CircularProgress::builder().launch(0.0).detach();

        Self {
            episode,
            texture: None,
            downloaded,
            downloading: false,
            progress_indicator,
            play_button,
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

        // --- Register per-row actions ---
        let action_group = gio::SimpleActionGroup::new();

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let play_next_action = gio::SimpleAction::new("play-next", None);
        play_next_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeListItemOutput::SetPlayNext(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let add_to_playlist_action = gio::SimpleAction::new("add-to-playlist", None);
        add_to_playlist_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeListItemOutput::AddToPlaylist(id.clone()));
        });

        let sender_clone = sender.clone();
        let mark_played_action = gio::SimpleAction::new("mark-played", None);
        mark_played_action.connect_activate(move |_, _| {
            sender_clone.input(EpisodeListItemInput::MarkPlayed);
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let goto_episode_action = gio::SimpleAction::new("goto-episode", None);
        goto_episode_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeListItemOutput::GotoEpisode(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let download_action = gio::SimpleAction::new("download", None);
        download_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(EpisodeListItemOutput::RequestDownload(id));
        });

        let sender_clone = sender.clone();
        let delete_download_action = gio::SimpleAction::new("delete-download", None);
        delete_download_action.connect_activate(move |_, _| {
            sender_clone.input(EpisodeListItemInput::RequestDelete);
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

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            EpisodeListItemInput::ImageDownloaded(fetched_texture) => {
                self.texture = fetched_texture;
            }
            EpisodeListItemInput::TogglePlay => {
                let _ = sender.output(EpisodeListItemOutput::TogglePlay(self.episode.id()));
            }
            EpisodeListItemInput::CancleDownload => {
                let _ = sender.output(EpisodeListItemOutput::CancleDownload(self.episode.id()));
            }
            EpisodeListItemInput::DownloadCancled => todo!(),
            EpisodeListItemInput::RequestDownload => {
                let _ = sender.output(EpisodeListItemOutput::RequestDownload(self.episode.id()));
            }
            EpisodeListItemInput::DownloadStarted => {
                self.downloading = true;
            }
            EpisodeListItemInput::DownloadProgress(fraction) => {
                self.downloading = true;
                let _ = self
                    .progress_indicator
                    .sender()
                    .send(CircularProgressInput::SetFraction(fraction));
            }
            EpisodeListItemInput::DownloadFinished => {
                self.downloading = false;
                self.downloaded = true;
            }
            EpisodeListItemInput::ChangePlayBackState(state) => match state {
                PlayState::Stopped => {
                    self.play_button.emit(PlayButtonInput::UpdatePlayingState(
                        EpisodePlayingState::Stopped,
                    ));

                    let duration_str = match self.episode.duration() {
                        Some(seconds) if seconds > 0 => {
                            let hours = seconds / 3600;
                            let minutes = (seconds % 3600) / 60;

                            if hours > 0 {
                                format!("{}h {}m", hours, minutes)
                            } else {
                                format!("{}m", minutes)
                            }
                        }
                        _ => "0m".to_string(),
                    };

                    self.play_button
                        .emit(PlayButtonInput::SetLabel(duration_str));
                }
                PlayState::Buffering => {
                    self.play_button.emit(PlayButtonInput::UpdatePlayingState(
                        EpisodePlayingState::Playing,
                    ));
                }
                PlayState::Paused => {
                    self.play_button.emit(PlayButtonInput::UpdatePlayingState(
                        EpisodePlayingState::Paused,
                    ));
                }
                PlayState::Playing => {
                    self.play_button.emit(PlayButtonInput::UpdatePlayingState(
                        EpisodePlayingState::Playing,
                    ));
                }
                _ => {}
            },
            EpisodeListItemInput::PlayBackProgress(fraction, rem) => {
                self.play_button
                    .emit(PlayButtonInput::UpdateProgress(fraction));
                let duration_str = if rem > 0 {
                    let hours = rem / 3600;
                    let minutes = (rem % 3600) / 60;
                    let seconds = rem % 60;

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

                self.play_button
                    .emit(PlayButtonInput::SetLabel(duration_str));
            }
            EpisodeListItemInput::ChangeEpisodeTo(episode_id) => {
                if episode_id != self.episode.id() {
                    sender.input(EpisodeListItemInput::ChangePlayBackState(
                        PlayState::Stopped,
                    ));
                }
            }
            EpisodeListItemInput::MarkPlayed => {
                self.episode
                    .set_played(Some(chrono::Utc::now().naive_utc()));
                match self.episode.save() {
                    Ok(_) => {}
                    Err(error) => {
                        let _ =
                            sender.output(EpisodeListItemOutput::NotifyError(error.to_string()));
                    }
                }
            }
            EpisodeListItemInput::RequestDelete => {
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
                            sender_clone.input(EpisodeListItemInput::ConfirmDelete);
                        }
                    },
                );
            }
            EpisodeListItemInput::ConfirmDelete => {
                let _ = sender.output(EpisodeListItemOutput::RequestDeleteEpisode(
                    self.episode.id(),
                ));
            }

            EpisodeListItemInput::EpisodeDeleted => {
                self.downloaded = false;
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            EpisodeListItemCmdInput::DownloadImage(opt_texture) => {
                sender.input(EpisodeListItemInput::ImageDownloaded(opt_texture));
            }
        }
    }

    view! {
        gtk::Box {
            set_halign: gtk::Align::Fill,
            set_orientation: gtk::Orientation::Horizontal,
            set_margin_all: 16,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start, // Locks the content layout tightly to the left
                set_spacing: 16,

                 gtk::Overlay {
                    set_height_request: 150,
                    set_width_request: 150,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        inline_css: "background-color: mix(var(--window-bg-color), var(--card-fg-color), 0.1); border-radius: 16px; box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32); border: 1px solid alpha(@borders, 0.8)",

                        gtk::Label {
                            #[watch]
                            set_label: &self.episode.title().trim().chars().take(2).collect::<String>().to_uppercase(),
                            set_css_classes: &vec!["title-large", "dimmed"],
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            inline_css: "opacity: 0.25; font-weight: 800",
                        }
                    },

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
                        inline_css: "border-radius: 16px;",
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,

                     gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Start,
                        set_spacing: 16,

                        gtk::Label {
                            set_label: &self.episode.epoch().format("%e %b").to_string(),
                            add_css_class: "caption",
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_wrap: true
                        },

                         gtk::Label {
                            #[watch]
                            set_visible:  self.episode.played().is_some(),
                            set_label: gettext("Played").as_str(),
                            set_css_classes: &vec!["caption", "warning"],
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_wrap: true
                        },
                    },

                    gtk::Label {
                        set_label: self.episode.title(),
                        add_css_class: "heading",
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true
                    },

                    gtk::Label {
                        #[watch]
                        set_use_markup: true,
                        #[watch]
                        set_markup: &{
                            let raw_markup = if let Some(desc) = self.episode.description() {
                                let markup = episode_description_parser::html2pango_markup(desc);

                                if markup.is_empty() && !desc.is_empty() {
                                    html2text::config::plain()
                                        .string_from_read(desc.as_bytes(), desc.len())
                                        .unwrap_or_else(|_| desc.to_string())
                                } else {
                                    markup
                                }
                            } else {
                                "".to_string()
                            };
                            raw_markup.replace('\n', " ").replace('\r', " ")
                        },
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_lines: 3,
                        set_xalign: 0.0,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_css_classes: &vec!["dimmed", "body"]
                    },

                    gtk::Separator {
                        set_vexpand: true,
                        add_css_class: "spacer",
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Start,
                        set_spacing: 16,

                        self.play_button.widget(),

                        gtk::Box {
                            #[watch]
                            set_visible: self.downloading,

                            self.progress_indicator.widget() {
                                set_size_request: (34, 34),
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                            }
                        },

                        gtk::Box{
                            #[watch]
                            set_visible: self.downloaded,
                            set_css_classes: &vec!["success"],
                            inline_css: "border-radius: 50px; padding: 4px; border: 2px solid var(--success-color);",
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            gtk::Image{
                                set_icon_name: Some("folder-download-symbolic")
                            }
                        }
                    }
                },
            },

            gtk::Separator {
                set_hexpand: true,
                add_css_class: "spacer",
                set_halign: gtk::Align::Fill,
            },

            gtk::Box {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,
                set_orientation: gtk::Orientation::Horizontal,

                    gtk::MenuButton {
                        set_icon_name: "view-more-symbolic",
                        set_tooltip_text: Some(&gettext("Episode Options")),
                        set_css_classes: &vec!["circular"],
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,

                        #[wrap(Some)]
                        #[name = "menu_button"]
                        set_popover = &gtk::PopoverMenu::from_model(Some(&{
                            let menu = gtk::gio::Menu::new();
                            let play_section = gtk::gio::Menu::new();
                            let play_next_item = gtk::gio::MenuItem::new(Some(&gettext("Play Next")), Some("episode.play-next"));
                            let mark_played_item = gtk::gio::MenuItem::new(Some(&gettext("Mark as Played")), Some("episode.mark-played"));
                            play_section.append_item(&play_next_item);
                            play_section.append_item(&mark_played_item);
                            menu.append_section(None, &play_section);


                            let playlist_section = gtk::gio::Menu::new();
                            let add_to_playlist_item = gtk::gio::MenuItem::new(Some(&gettext("Add to Playlist")), Some("episode.add-to-playlist"));
                            playlist_section.append_item(&add_to_playlist_item);
                            menu.append_section(None, &playlist_section);

                            let go_to_section = gtk::gio::Menu::new();
                            let goto_episode_item = gtk::gio::MenuItem::new(Some(&gettext("Go to Episode")), Some("episode.goto-episode"));
                            go_to_section.append_item(&goto_episode_item);
                            menu.append_section(None, &go_to_section);

                            let download_section = gtk::gio::Menu::new();
                            let download_episode_item = if self.downloaded {
                                gtk::gio::MenuItem::new(Some(&gettext("Delete Download")), Some("episode.delete-download"))

                            } else {
                                gtk::gio::MenuItem::new(Some(&gettext("Download")), Some("episode.download"))
                            };
                            download_section.append_item(&download_episode_item);
                            menu.append_section(None, &download_section);

                            menu
                        })) {}
                }
            },
        }
    }
}
