//! This module contains the internal implementation of a translation layer
//! between Android and egui, which makes up a majority of this library. Nothing
//! in here is actually exposed to app developers, but is run automatically
//! behind the scenes.

pub(crate) mod bindings;
pub(crate) mod logging;
pub(crate) mod runner;

mod graphics;
mod ime;
mod input;
mod keycodes;
mod state;
