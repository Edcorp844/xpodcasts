#[macro_use]
extern crate log;

pub mod action;
pub mod app;
pub mod app_navigation_ext;
pub mod app_render_ext;
pub mod chapter_parser;
pub mod components;
pub mod config;
pub mod pages;
pub mod settings;
pub mod util;
pub mod workers;

pub static RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> =
    std::sync::LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

