use gtk::prelude::GtkApplicationExt;

use crate::util::size_class::{HorizontalSizeClass, SizeClass};

#[macro_use]
extern crate log;

pub mod app;
pub mod app_navigation_ext;
//pub mod app_render_ext;
pub mod chapter_parser;
pub mod components;
pub mod config;
pub mod pages;
pub mod settings;
pub mod util;
pub mod workers;

pub static RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> =
    std::sync::LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

pub fn get_horizontal_size_class() -> Option<SizeClass> {
    let app = relm4::main_application();
    if let Some(window) = app.active_window() {
        Some(window.horizontal_size_class())
    } else {
        None
    }
}
