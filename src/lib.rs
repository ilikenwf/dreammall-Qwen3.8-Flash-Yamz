//! dreammall: pure math/audio/texture modules + the bevy app layer.

// lets the moved-in modules keep their `dreammall::` absolute paths
extern crate self as dreammall;

pub mod app;
pub mod game;
pub mod shared;
pub mod synth;
pub mod texturegen;
pub mod world;
