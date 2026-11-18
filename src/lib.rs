pub mod cache;
pub mod config;
pub mod download;
#[cfg(feature = "mobile")]
pub mod ffi;
pub mod favorites;
pub mod history;
pub mod language_filter;
pub mod logging;
pub mod models;
pub mod net;
pub mod player;
pub mod providers;
pub mod proxy;
pub mod service;
pub mod text_util;
pub mod tui;
pub mod updater;

#[cfg(feature = "mobile")]
uniffi::setup_scaffolding!();
