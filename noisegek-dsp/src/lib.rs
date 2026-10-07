//! NOISEGEK · núcleo DSP · por @ahrsml
//!
//! Librería pura (sin dependencias). El procesamiento no asigna memoria en el
//! heap: la regeneración de las tablas del waveshaper al cambiar su seed usa
//! un búfer en la pila. El plugin
//! (crate `noisegek`) solo traduce parámetros/transporte y llama a [`Engine`].

pub mod engine;
pub mod filter;
pub mod modules;
pub mod noise;
pub mod rng;
pub mod seq;

pub use engine::{EnvParams, EnvTap, Engine, FeedbackParams, Params, Snapshot, Transport, MAX_SR, NUM_SLOTS};
pub use filter::FilterMode;
pub use modules::Module;
pub use noise::NoiseType;
pub use seq::{Clock, Direction, LaneParams, LaneSource, Pattern, NUM_DEST, NUM_LANES, NUM_STEPS};
