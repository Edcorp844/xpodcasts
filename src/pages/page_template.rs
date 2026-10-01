use adw::prelude::*;
use podcasts_data::discovery::FoundPodcast;
use relm4::{Component, prelude::*};

pub struct PodcastPage {
    // Add your page state fields here (e.g., selected podcast)
}

#[relm4::component(pub)]
impl Component for PodcastPage {
    type Init = ();
    type Input = ();
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::NavigationPage {
            // Set the title displayed in the navigation stack
            set_title: "Podcast Details",

           #[wrap(Some)]
            set_child = &adw::ToolbarView {

                 add_top_bar=&adw::HeaderBar {},

               #[wrap(Some)]
                set_content=&gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 12,
                    set_spacing: 6,

                    gtk::Label {
                        set_label: "Welcome to the Podcast Page",
                        add_css_class: "title-1",
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PodcastPage {};

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
