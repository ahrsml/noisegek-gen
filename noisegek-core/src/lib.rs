//! NOISEGEK · por @ahrsml
//!
//! Dos plugins con el mismo motor:
//!   * [`Noisegek`]    efecto: procesa el audio de la pista (+ ruido interno).
//!   * [`NoisegekGen`] instrumento: genera ruido al recibir notas MIDI
//!                     (piano roll), con envolvente y seguimiento de teclado.
//! Los crates `noisegek` y `noisegek-gen` solo los exportan como VST3/CLAP
//! (archivos separados: algunos hosts, como FL Studio, no verifican archivos
//! con varios plugins dentro).

use nih_plug::prelude::*;
use nih_plug::wrapper::state::PluginState;
use noisegek_dsp::{Engine, Snapshot, Transport};
use std::sync::{Arc, Mutex};

mod editor;
mod labels;
mod params;

pub use params::{migrate_state, GenParams, NgParams, VoiceUiParams};

/// Estado que el audio comparte con la UI (solo lectura para la UI).
#[derive(Clone, Copy, Debug, Default)]
pub struct UiSnap {
    pub s: Snapshot,
    pub tempo: f32,
}

#[derive(Default)]
pub struct Shared {
    pub snap: Mutex<UiSnap>,
}

fn host_transport(tr: &nih_plug::prelude::Transport) -> Transport {
    Transport {
        playing: tr.playing,
        pos_qn: tr.pos_beats(),
        tempo: tr.tempo.unwrap_or(120.0),
        ts_num: tr.time_sig_numerator.unwrap_or(4) as f64,
        ts_den: tr.time_sig_denominator.unwrap_or(4) as f64,
    }
}

/// Copia el patrón de la UI al motor (inicio de bloque) sin bloquear.
fn sync_pattern_in(params: &NgParams, engine: &mut Engine) {
    if let Ok(p) = params.pattern.try_read() {
        engine.pattern = (*p).into();
    }
}

/// Devuelve al estado persistente lo que la mutación cambió, y publica el
/// estado en vivo para la UI.
fn finish_block(params: &NgParams, engine: &mut Engine, shared: &Shared, tempo: f64) {
    if engine.pattern_mutated {
        if let Ok(mut w) = params.pattern.try_write() {
            *w = engine.pattern.into();
            engine.pattern_mutated = false;
        }
    }
    if params.editor_state.is_open() {
        if let Ok(mut s) = shared.snap.try_lock() {
            *s = UiSnap { s: engine.snapshot(), tempo: tempo as f32 };
        }
    }
}

// =============================================================================
// EFECTO
// =============================================================================

pub struct Noisegek {
    params: Arc<NgParams>,
    shared: Arc<Shared>,
    engine: Engine,
}

impl Default for Noisegek {
    fn default() -> Self {
        Self {
            params: Arc::new(NgParams::default()),
            shared: Arc::new(Shared::default()),
            engine: Engine::new(48000.0),
        }
    }
}

impl Plugin for Noisegek {
    const NAME: &'static str = "NOISEGEK";
    const VENDOR: &'static str = "@ahrsml";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine = Engine::new(buffer_config.sample_rate as f64);
        if let Ok(p) = self.params.pattern.read() {
            self.engine.pattern = (*p).into();
        }
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn filter_state(state: &mut PluginState) {
        migrate_state(state, false);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        sync_pattern_in(&self.params, &mut self.engine);
        let transport = host_transport(context.transport());
        let p = self.params.to_engine();
        self.engine.begin_block(&p, &transport, buffer.samples());

        let chans = buffer.as_slice();
        if chans.len() >= 2 {
            let (l, r) = chans.split_at_mut(1);
            for (a, b) in l[0].iter_mut().zip(r[0].iter_mut()) {
                let (ya, yb) = self.engine.process_frame(*a, *b);
                *a = ya;
                *b = yb;
            }
        } else if let Some(m) = chans.first_mut() {
            for x in m.iter_mut() {
                let (y, _) = self.engine.process_frame(*x, *x);
                *x = y;
            }
        }

        finish_block(&self.params, &mut self.engine, &self.shared, transport.tempo);
        // El ruido interno suena aunque la entrada sea silencio.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Noisegek {
    const CLAP_ID: &'static str = "com.ahrsml.noisegek";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Noise/ambient FX + secuenciador de 4 carriles");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
        ClapFeature::Filter,
    ];
}

impl Vst3Plugin for Noisegek {
    const VST3_CLASS_ID: [u8; 16] = *b"NOISEGEKahrsml01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion, Vst3SubCategory::Filter];
}

// =============================================================================
// INSTRUMENTO (GEN)
// =============================================================================

pub struct NoisegekGen {
    params: Arc<GenParams>,
    shared: Arc<Shared>,
    engine: Engine,
}

impl Default for NoisegekGen {
    fn default() -> Self {
        Self {
            params: Arc::new(GenParams::default()),
            shared: Arc::new(Shared::default()),
            engine: Engine::new(48000.0),
        }
    }
}

impl Plugin for NoisegekGen {
    const NAME: &'static str = "NOISEGEK GEN";
    const VENDOR: &'static str = "@ahrsml";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Instrumento: sin entrada de audio, salida estéreo, notas MIDI.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine = Engine::new(buffer_config.sample_rate as f64);
        if let Ok(p) = self.params.core.pattern.read() {
            self.engine.pattern = (*p).into();
        }
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn filter_state(state: &mut PluginState) {
        migrate_state(state, true);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let core = &self.params.core;
        sync_pattern_in(core, &mut self.engine);
        let transport = host_transport(context.transport());
        let p = self.params.to_engine();
        let n = buffer.samples();
        self.engine.begin_block(&p, &transport, n);

        // Notas con precisión de muestra.
        let mut next = context.next_event();
        let chans = buffer.as_slice();
        for i in 0..n {
            while let Some(ev) = next {
                if ev.timing() as usize > i {
                    break;
                }
                match ev {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        if velocity > 0.0 {
                            self.engine.note_on(note, velocity);
                        } else {
                            self.engine.note_off(note);
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => self.engine.note_off(note),
                    NoteEvent::Choke { note, .. } => self.engine.note_off(note),
                    _ => {}
                }
                next = context.next_event();
            }
            let (l, r) = self.engine.process_frame(0.0, 0.0);
            if let Some(c) = chans.get_mut(0) {
                c[i] = l;
            }
            if let Some(c) = chans.get_mut(1) {
                c[i] = r;
            }
        }

        finish_block(core, &mut self.engine, &self.shared, transport.tempo);
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for NoisegekGen {
    const CLAP_ID: &'static str = "com.ahrsml.noisegek-gen";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Generador de ruido tocable por MIDI + secuenciador de 4 carriles");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for NoisegekGen {
    const VST3_CLASS_ID: [u8; 16] = *b"NOISEGEKGENahrsm";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
