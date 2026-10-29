#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod http;
mod model;
mod storage;
mod theme;
mod ui;
mod util;

fn main() -> eframe::Result<()> {
    app::run()
}
