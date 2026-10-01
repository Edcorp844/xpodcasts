use adw::prelude::*;
use gettextrs::gettext;
use relm4::ComponentSender;

use crate::{
    app::{AppModel, AppModelInput, AppModelWidgets},
    app_navigation_ext::NavigationPage,
};

impl AppModel {
    pub(crate) fn show_search_page(widgets: &AppModelWidgets, sender: &ComponentSender<Self>) {
        let pages = &widgets.pages;
        pages.unselect_all();

        let library = &widgets.library;
        library.unselect_all();

        sender.input(AppModelInput::SelectPage(NavigationPage::Search));
    }
    pub(crate) fn render_sidebar_list(widgets: &AppModelWidgets, sender: &ComponentSender<Self>) {
        // Data configurations
        let (home_binding, new_binding) = (gettext("Home"), gettext("New"));
        let pages_list_items = [
            ("user-home-symbolic", home_binding.as_str()),
            ("view-grid-symbolic", new_binding.as_str()),
        ];

        let (recents_binding, shows_binding, downloads_binding) = (
            gettext("Recently updated"),
            gettext("Shows"),
            gettext("Downloaded"),
        );
        let library_list_items = [
            ("emoji-recent-symbolic", recents_binding.as_str()),
            ("display-projector-symbolic", shows_binding.as_str()),
            ("folder-download-symbolic", downloads_binding.as_str()),
        ];

        // 1. Clean dynamic population using a unified helper function
        for (icon, label) in &pages_list_items {
            let row = Self::create_sidebar_row(icon, label);
            widgets.pages.append(&row);
        }

        for (icon, label) in &library_list_items {
            let row = Self::create_sidebar_row(icon, label);
            widgets.library.append(&row);
        }

        Self::setup_collapsible_section(
            &widgets.library_header,
            &widgets.library_revealer,
            &widgets.library_chevron,
        );

        let pages_weak = widgets.pages.downgrade();
        let sender_clone = sender.clone();

        let search_button_weak = widgets.search_button.downgrade();
        widgets.library.connect_row_activated(move |_, row| {
            if let Some(pages) = pages_weak.upgrade() {
                pages.unselect_all();
            }
            if let Some(btn) = search_button_weak.upgrade() {
                btn.set_active(false);
            }

            let widget_name = row.widget_name().to_string();
            let resolved_page = NavigationPage::from_name(&widget_name);
            sender_clone.input(AppModelInput::SelectPage(resolved_page));
        });

        let library_weak = widgets.library.downgrade();
        let search_button_weak = widgets.search_button.downgrade();
        let sender_clone = sender.clone();
        widgets.pages.connect_row_activated(move |_, row| {
            if let Some(library) = library_weak.upgrade() {
                library.unselect_all();
            }
            if let Some(btn) = search_button_weak.upgrade() {
                btn.set_active(false);
            }
            let widget_name = row.widget_name().to_string();
            let resolved_page = NavigationPage::from_name(&widget_name);

            sender_clone.input(AppModelInput::SelectPage(resolved_page));
        });
    }

    fn create_sidebar_row(icon_name: &str, label_text: &str) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();

        let layout_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        layout_box.set_margin_start(8);
        layout_box.set_margin_end(8);
        layout_box.set_margin_top(8);
        layout_box.set_margin_bottom(8);

        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            //.css_classes(vec!["accent"])
            .icon_size(gtk::IconSize::Inherit)
            .build();
        layout_box.append(&icon);

        let label = gtk::Label::new(Some(label_text));
        layout_box.append(&label);

        row.set_child(Some(&layout_box));
        row.set_widget_name(label_text);
        row
    }

    pub fn setup_collapsible_section(
        header: &gtk::Box,
        revealer: &gtk::Revealer,
        chevron: &gtk::Image,
    ) {
        let r = revealer.clone();
        let c = chevron.clone();
        let gesture = gtk::GestureClick::new();

        gesture.connect_released(move |_, _, _, _| {
            let is_revealing = !r.reveals_child();
            r.set_reveal_child(is_revealing);
            c.set_icon_name(Some(if is_revealing {
                "pan-down-symbolic"
            } else {
                "pan-end-symbolic"
            }));
        });
        header.add_controller(gesture);
    }
}
