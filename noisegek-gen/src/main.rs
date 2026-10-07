//! NOISEGEK GEN standalone (desarrollo): abre el instrumento sin DAW.
//!   cargo run -p noisegek-gen --features standalone --release -- --backend dummy
use nih_plug::prelude::*;
use noisegek_core::NoisegekGen;

fn main() {
    nih_export_standalone::<NoisegekGen>();
}
