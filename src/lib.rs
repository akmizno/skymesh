#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::App;

pub(crate) mod camera;
pub(crate) mod import;
pub(crate) mod model;
pub(crate) mod render;
