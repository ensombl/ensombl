//! Ensombl's engine, free of any UI shell. The Tauri app and the headless
//! server both wire it up through [`app::Core`].

pub mod app;
pub mod application;
pub mod domain;
pub mod infrastructure;
