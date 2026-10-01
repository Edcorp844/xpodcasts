use podcasts_data::{Show, ShowId};
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::util::cover_image::{ImageSize, fetch_cached_image};

#[derive(Debug)]
pub struct ShowCard {
    show: Show,
    texture: Option<adw::gdk::Texture>,
}

#[derive(Debug)]
pub enum ShowCardInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
    GotoShow,
}

#[derive(Debug)]
pub enum ShowCardOutput {
    GotoShow(ShowId),
}

#[derive(Debug)]
pub enum ShowCardCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for ShowCard {
    type Init = Show;
    type Input = ShowCardInput;
    type Output = ShowCardOutput;
    type CommandOutput = ShowCardCmdInput;
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_width_request: 160,

            // --- 1. THE IMAGE OVERLAY BLOCK ---
            gtk::Overlay {
                set_hexpand: true,
                set_vexpand: true,
                set_height_request: 160,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    add_css_class: "frame",

                    inline_css: "border-radius: 12px; box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32); padding: 2px",

                    gtk::Label {
                        set_label: &self.show.title().trim().chars().take(2).collect::<String>().to_uppercase(),
                        add_css_class: "title-1",
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
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

                    inline_css: "border-radius: 12px",
                }
            },

            // --- 2. CARD METADATA BLOCK ---
            gtk::Label {
                set_label: self.show.title().trim(),
                set_valign: gtk::Align::Start,
                set_halign: gtk::Align::Start,
                set_xalign: 0.0,

                inline_css: "font-weight: 600; font-size: 0.92rem",

                set_wrap: true,
                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                set_lines: 2,

                set_height_request: 42,
                set_width_request: 150,

            },

            add_controller = gtk::GestureClick {
                connect_released[sender] => move |_, _, _, _| {
                    sender.input(ShowCardInput::GotoShow);
                }
            }
        }
    }

    fn init_model(show: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let clone = show.clone();

        if let Some(image_url_ref) = clone.image_uri() {
            let image_url = image_url_ref.to_string();

            sender.oneshot_command(async move {
                let downloaded_texture = fetch_cached_image(&image_url, ImageSize::default()).await;

                ShowCardCmdInput::DownloadImage(downloaded_texture)
            });
        };

        Self {
            show,
            texture: None,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            ShowCardInput::ImageDownloaded(fetched_texture) => {
                self.texture = fetched_texture;
            }
            ShowCardInput::GotoShow => {
                let _ = sender.output(ShowCardOutput::GotoShow(self.show.id()));
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            ShowCardCmdInput::DownloadImage(opt_texture) => {
                sender.input(ShowCardInput::ImageDownloaded(opt_texture));
            }
        }
    }
}
