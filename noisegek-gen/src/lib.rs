//! NOISEGEK GEN (instrumento) · VST3/CLAP · por @ahrsml. Todo el código vive
//! en `noisegek-core`; este crate solo exporta el plugin.
use nih_plug::prelude::*;
pub use noisegek_core::NoisegekGen;

nih_export_clap!(NoisegekGen);
nih_export_vst3!(NoisegekGen);
