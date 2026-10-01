use adw::prelude::*;
use gettextrs::gettext;
use gtk::gio;
use podcasts_data::{Episode, EpisodeId, dbqueries};
use relm4::factory::FactoryComponent;
use relm4::{FactorySender, RelmWidgetExt};

use crate::util::cover_image::{ImageSize, fetch_cached_image};
use crate::util::episode_description_parser;

#[derive(Debug)]
pub struct PlayListEpisodeListItem {
    episode: Episode,
    texture: Option<adw::gdk::Texture>,
    is_playing: bool,
}

#[derive(Debug)]
pub enum PlayListEpisodeListItemInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
}

#[derive(Debug)]
pub enum PlayListEpisodeListItemOutput {
    SetPlayNext(EpisodeId),
    SetPlayNow(EpisodeId),
    RemoveFromPlayList(EpisodeId),
    NotifyError(String),
}

#[derive(Debug)]
pub enum PlayListEpisodeListItemCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for PlayListEpisodeListItem {
    type Init = (Episode, bool);
    type Input = PlayListEpisodeListItemInput;
    type Output = PlayListEpisodeListItemOutput;
    type CommandOutput = PlayListEpisodeListItemCmdInput;
    type ParentWidget = gtk::ListBox;

    view! {
         gtk::Box {
            set_halign: gtk::Align::Fill,
            set_orientation: gtk::Orientation::Horizontal,
            set_margin_all: 16,

           gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start,
                set_spacing: 16,

                gtk::Overlay {
                    set_height_request: 54,
                    set_width_request: 54,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        add_css_class: "frame",
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        inline_css: "background-color: mix(var(--window-bg-color), var(--card-fg-color), 0.1); border-radius: 8px; box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32); ",

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
                    },

                    add_overlay = &gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        #[watch]
                        set_visible: self.is_playing,
                        add_css_class: "eq-bars",
                        inline_css: "background-color: alpha(black, 0.25); border-radius: 8px; padding: 6px;",

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 3,

                            gtk::Box { add_css_class: "eq-bar", add_css_class: "eq-bar-1" },
                            gtk::Box { add_css_class: "eq-bar", add_css_class: "eq-bar-2" },
                            gtk::Box { add_css_class: "eq-bar", add_css_class: "eq-bar-3" },
                            gtk::Box { add_css_class: "eq-bar", add_css_class: "eq-bar-4" },
                        }
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,
                    set_hexpand: true,
                    set_width_request: 400,

                    gtk::Label {
                        set_label: self.episode.title(),
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true,
                        set_lines: 1,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
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
                        set_lines: 1,
                        set_xalign: 0.0,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_css_classes: &vec!["dimmed", "body"]
                    },

                },

           },

           gtk::Box {
                set_hexpand: true,
            },

            gtk::Box {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,

                gtk::MenuButton {
                    set_icon_name: "view-more-symbolic",
                    set_tooltip_text: Some(&gettext("Play List Options")),
                    set_css_classes: &vec!["circular", "flat"],
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    #[wrap(Some)]
                        #[name = "menu_button"]
                        set_popover = &gtk::PopoverMenu::from_model(Some(&{
                            let menu = gtk::gio::Menu::new();

                            let play_section = gtk::gio::Menu::new();

                            let play_next_item = gtk::gio::MenuItem::new(Some(&gettext("Play Next")), if !self.is_playing{Some("playlist.play-next")} else{ None});
                            let play_now_item = gtk::gio::MenuItem::new(Some(&gettext("Play Now")), if !self.is_playing{Some("playlist.play-now")} else{ None});
                            play_section.append_item(&play_now_item);
                            play_section.append_item(&play_next_item);

                            menu.append_section(None, &play_section);

                            let manage_section = gtk::gio::Menu::new();
                            let remove_item = gtk::gio::MenuItem::new(Some(&gettext("Remove")), Some("playlist.remove"));
                            manage_section.append_item(&remove_item);
                            menu.append_section(None, &manage_section);

                            menu
                        })) {}
                },
            }
        },

    }

    fn init_model(
        (episode, is_playing): Self::Init,
        _index: &Self::Index,
        sender: relm4::prelude::FactorySender<Self>,
    ) -> Self {
        match dbqueries::get_episode_from_id(episode.id()) {
            Ok(ep) => {
                if let Some(image_url_ref) = ep.image_uri() {
                    let image_url = image_url_ref.to_string();

                    sender.oneshot_command(async move {
                        let downloaded_texture =
                            fetch_cached_image(&image_url, ImageSize::default()).await;

                        PlayListEpisodeListItemCmdInput::DownloadImage(downloaded_texture)
                    });
                }
            }
            Err(e) => {
                let _ = sender.output(PlayListEpisodeListItemOutput::NotifyError(e.to_string()));
            }
        }
        Self {
            episode,
            texture: None,
            is_playing,
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
            let _ = sender_clone.output(PlayListEpisodeListItemOutput::SetPlayNext(id.clone()));
        });

        let sender_clone = sender.clone();
        let play_now_action = gio::SimpleAction::new("play-now", None);
        play_now_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(PlayListEpisodeListItemOutput::SetPlayNow(id.clone()));
        });

        let sender_clone = sender.clone();
        let id = self.episode.id();
        let remove_action = gio::SimpleAction::new("remove", None);
        remove_action.connect_activate(move |_, _| {
            let _ = sender_clone.output(PlayListEpisodeListItemOutput::RemoveFromPlayList(
                id.clone(),
            ));
        });

        action_group.add_action(&play_next_action);
        action_group.add_action(&play_now_action);
        action_group.add_action(&remove_action);

        widgets
            .menu_button
            .insert_action_group("playlist", Some(&action_group));

        widgets
    }

    fn update(&mut self, message: Self::Input, _sender: relm4::prelude::FactorySender<Self>) {
        match message {
            PlayListEpisodeListItemInput::ImageDownloaded(texture) => {
                self.texture = texture;
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            PlayListEpisodeListItemCmdInput::DownloadImage(opt_texture) => {
                sender.input(PlayListEpisodeListItemInput::ImageDownloaded(opt_texture));
            }
        }
    }
}
