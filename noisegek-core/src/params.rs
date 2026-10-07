//! Parámetros del plugin. Los IDs (`#[id = ...]`) son ESTABLES: cambiarlos
//! rompe proyectos y presets guardados. Rangos y defaults = JSFX v0.5.

use crate::labels;
use nih_plug::prelude::*;
use nih_plug::wrapper::state::{ParamValue, PluginState};
use nih_plug_egui::EguiState;
use noisegek_dsp::{Pattern, NUM_LANES, NUM_STEPS};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Tamaño inicial de la ventana: lienzo de diseño 1000x632 a escala 1. Cabe en
/// una pantalla de 1366x768 con el marco del host; se agranda desde la esquina.
pub const EDITOR_W: u32 = 1000;
pub const EDITOR_H: u32 = 632;
/// GEN: lienzo 1000x686 (una fila más: VOICE) a escala 0.96.
pub const GEN_EDITOR_W: u32 = 960;
pub const GEN_EDITOR_H: u32 = 659;

/// Patrón persistente (se guarda con el proyecto y los presets).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct PatternData {
    pub steps: [[f32; NUM_STEPS]; NUM_LANES],
    pub euc_k: [u8; NUM_LANES],
}

impl Default for PatternData {
    fn default() -> Self {
        Pattern::default().into()
    }
}
impl From<Pattern> for PatternData {
    fn from(p: Pattern) -> Self {
        Self { steps: p.steps, euc_k: p.euc_k }
    }
}
impl From<PatternData> for Pattern {
    fn from(p: PatternData) -> Self {
        Pattern { steps: p.steps, euc_k: p.euc_k }
    }
}

/// Selector entero con nombres (el host muestra el nombre, no el número).
fn choice(name: &str, default: i32, names: &'static [&'static str]) -> IntParam {
    IntParam::new(name, default, IntRange::Linear { min: 0, max: names.len() as i32 - 1 })
        .with_value_to_string(Arc::new(move |v| names[v.clamp(0, names.len() as i32 - 1) as usize].to_string()))
        .with_string_to_value(Arc::new(move |s| {
            names.iter().position(|n| n.eq_ignore_ascii_case(s.trim())).map(|i| i as i32)
        }))
}

fn percent(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_unit(" %")
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}

fn pct100(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_step_size(0.1)
        .with_unit(" %")
        .with_value_to_string(Arc::new(|v| format!("{v:.0}")))
}

fn db(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_step_size(0.1)
        .with_unit(" dB")
        .with_value_to_string(Arc::new(|v| format!("{v:.1}")))
}

fn ms(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-2.0) })
        .with_unit(" ms")
        .with_value_to_string(Arc::new(|v| if v >= 100.0 { format!("{v:.0}") } else { format!("{v:.1}") }))
}

#[derive(Params)]
pub struct SlotParams {
    #[id = "module"]
    pub module: IntParam,
    #[id = "a"]
    pub a: FloatParam,
    #[id = "b"]
    pub b: FloatParam,
}

impl SlotParams {
    fn new(i: usize) -> Self {
        let n = i + 1;
        Self {
            module: choice(&format!("Slot {n} Module"), if i == 0 { 2 } else { 0 }, &labels::MODULES),
            a: FloatParam::new(format!("Slot {n} Param A"), 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{v:.3}"))),
            b: FloatParam::new(format!("Slot {n} Param B"), 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{v:.3}"))),
        }
    }
}

#[derive(Params)]
pub struct ModLaneParams {
    #[id = "dest"]
    pub dest: IntParam,
    #[id = "depth"]
    pub depth: FloatParam,
    #[id = "rate"]
    pub rate: IntParam,
    #[id = "steps"]
    pub steps: IntParam,
    #[id = "dir"]
    pub dir: IntParam,
    #[id = "glide"]
    pub glide: FloatParam,
    #[id = "prob"]
    pub prob: FloatParam,
    #[id = "mutate"]
    pub mutate: FloatParam,
}

impl ModLaneParams {
    fn new(i: usize) -> Self {
        let n = i + 1;
        let (rate, steps, dir, glide) = match i {
            0 => (9, 16, 0, 0.0),
            1 => (7, 12, 0, 30.0),
            _ => (3, 7, 2, 100.0),
        };
        Self {
            dest: choice(&format!("Mod {n} Destination"), 0, &labels::DESTS_STATIC),
            depth: pct100(&format!("Mod {n} Depth"), 50.0, -100.0, 100.0)
                .with_value_to_string(Arc::new(|v| format!("{v:+.0}"))),
            rate: choice(&format!("Mod {n} Rate"), rate, &noisegek_dsp::seq::RATE_NAMES),
            steps: IntParam::new(format!("Mod {n} Steps"), steps, IntRange::Linear { min: 1, max: 16 }),
            dir: choice(&format!("Mod {n} Direction"), dir, &labels::DIRS),
            glide: pct100(&format!("Mod {n} Glide"), glide, 0.0, 100.0),
            prob: pct100(&format!("Mod {n} Probability"), 100.0, 0.0, 100.0),
            mutate: pct100(&format!("Mod {n} Mutate"), 0.0, 0.0, 100.0),
        }
    }
}

/// Fuente de un carril MOD (0.8). Arreglo aparte para que sus IDs
/// (`src_1`..`src_3`) queden al final de la lista de parámetros.
#[derive(Params)]
pub struct ModSrcParams {
    #[id = "src"]
    pub src: IntParam,
}

impl ModSrcParams {
    fn new(i: usize) -> Self {
        Self { src: choice(&format!("Mod {} Source", i + 1), 0, &labels::LANE_SOURCES) }
    }
}

fn hz(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Logarithmic { min, max })
        .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
        .with_string_to_value(formatters::s2v_f32_hz_then_khz())
}

#[derive(Params)]
pub struct NgParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[persist = "pattern"]
    pub pattern: Arc<RwLock<PatternData>>,

    // --- Cadena ---
    #[nested(array, group = "Slot")]
    pub slots: [SlotParams; 4],
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "out"]
    pub out: FloatParam,
    #[id = "tseed"]
    pub table_seed: IntParam,

    // --- Source ---
    #[id = "ntype"]
    pub noise_type: IntParam,
    #[id = "nlevel"]
    pub noise_level: FloatParam,
    #[id = "nchar"]
    pub noise_char: FloatParam,

    // --- Filter ---
    #[id = "fmode"]
    pub filter_mode: IntParam,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "reso"]
    pub reso: FloatParam,
    #[id = "fpos"]
    pub filter_pos: IntParam,

    // --- Seq global ---
    #[id = "clock"]
    pub clock: IntParam,
    #[id = "swing"]
    pub swing: FloatParam,
    #[id = "sseed"]
    pub seq_seed: IntParam,

    // --- Gate ---
    #[id = "gdepth"]
    pub gate_depth: FloatParam,
    #[id = "glen"]
    pub gate_len: FloatParam,
    #[id = "gatk"]
    pub gate_attack: FloatParam,
    #[id = "grel"]
    pub gate_release: FloatParam,
    #[id = "gpos"]
    pub gate_pos: IntParam,
    #[id = "grate"]
    pub gate_rate: IntParam,
    #[id = "gsteps"]
    pub gate_steps: IntParam,
    #[id = "gdir"]
    pub gate_dir: IntParam,
    #[id = "gprob"]
    pub gate_prob: FloatParam,
    #[id = "gmut"]
    pub gate_mutate: FloatParam,

    // --- Mod 1..3 ---
    #[nested(array, group = "Mod")]
    pub mods: [ModLaneParams; 3],

    // --- 0.8 (al final; los IDs anteriores no cambian) ---
    #[id = "fbamt"]
    pub fb_amount: FloatParam,
    #[id = "fbtime"]
    pub fb_time: FloatParam,
    #[id = "fbtone"]
    pub fb_tone: FloatParam,
    #[id = "fbtap"]
    pub fb_tap: IntParam,
    #[id = "scream"]
    pub scream: BoolParam,
    #[id = "eatk"]
    pub env_attack: FloatParam,
    #[id = "erel"]
    pub env_release: FloatParam,
    #[id = "egain"]
    pub env_gain: FloatParam,
    #[id = "etap"]
    pub env_tap: IntParam,
    #[id = "spread"]
    pub spread: FloatParam,
    #[nested(array, group = "Mod Source")]
    pub mod_src: [ModSrcParams; 3],
}

impl Default for NgParams {
    fn default() -> Self {
        Self::new(false)
    }
}

impl NgParams {
    /// `gen = true`: valores por defecto del instrumento (ruido encendido,
    /// ventana más alta). Los IDs son los mismos en ambos.
    pub fn new(gen: bool) -> Self {
        Self {
            editor_state: if gen {
                EguiState::from_size(GEN_EDITOR_W, GEN_EDITOR_H)
            } else {
                EguiState::from_size(EDITOR_W, EDITOR_H)
            },
            pattern: Arc::new(RwLock::new(PatternData::default())),

            slots: [SlotParams::new(0), SlotParams::new(1), SlotParams::new(2), SlotParams::new(3)],
            drive: db("Input Drive", 0.0, -12.0, 36.0),
            mix: percent("Mix Dry/Wet", 1.0),
            out: db("Output Gain", 0.0, -24.0, 12.0),
            table_seed: IntParam::new("Table Seed", 1234, IntRange::Linear { min: 0, max: 9999 }),

            noise_type: choice("Noise Type", if gen { 2 } else { 0 }, &labels::NOISES),
            // Sin unidad fija: "off" en -60, "x.x dB" en el resto (y se puede escribir igual).
            noise_level: FloatParam::new("Noise Level", if gen { -6.0 } else { -18.0 }, FloatRange::Linear { min: -60.0, max: 0.0 })
                .with_step_size(0.1)
                .with_value_to_string(Arc::new(|v| if v <= -60.0 { "off".into() } else { format!("{v:.1} dB") }))
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim().to_ascii_lowercase();
                    if s.starts_with("off") || s.contains("inf") {
                        Some(-60.0)
                    } else {
                        s.trim_end_matches("db").trim().parse::<f32>().ok()
                    }
                })),
            noise_char: percent("Noise Character", 0.5),

            filter_mode: choice("Filter Mode", 0, &labels::FILTER_MODES),
            cutoff: FloatParam::new(
                "Filter Cutoff",
                2000.0,
                FloatRange::Skewed { min: 20.0, max: 20000.0, factor: FloatRange::skew_factor(-2.0) },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            reso: percent("Filter Resonance", 0.2),
            filter_pos: choice("Filter Position", 0, &labels::FILTER_POS),

            clock: choice("Seq Clock", 0, &labels::CLOCKS),
            swing: pct100("Seq Swing", 50.0, 50.0, 75.0),
            seq_seed: IntParam::new("Seq Seed", 0, IntRange::Linear { min: 0, max: 9999 }),

            gate_depth: percent("Gate Depth", 0.0),
            gate_len: FloatParam::new("Gate Length", 0.5, FloatRange::Linear { min: 0.01, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            gate_attack: ms("Gate Attack", 1.0, 0.1, 100.0),
            gate_release: ms("Gate Release", 40.0, 1.0, 2000.0),
            gate_pos: choice("Gate Position", 0, &labels::GATE_POS),
            gate_rate: choice("Gate Rate", 9, &noisegek_dsp::seq::RATE_NAMES),
            gate_steps: IntParam::new("Gate Steps", 16, IntRange::Linear { min: 1, max: 16 }),
            gate_dir: choice("Gate Direction", 0, &labels::DIRS),
            gate_prob: pct100("Gate Probability", 100.0, 0.0, 100.0),
            gate_mutate: pct100("Gate Mutate", 0.0, 0.0, 100.0),

            mods: [ModLaneParams::new(0), ModLaneParams::new(1), ModLaneParams::new(2)],

            fb_amount: FloatParam::new("Feedback Amount", 0.0, FloatRange::Linear { min: 0.0, max: 1.5 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            fb_time: FloatParam::new("Feedback Time", 5.0, FloatRange::Logarithmic { min: 0.05, max: 50.0 })
                .with_unit(" ms")
                .with_value_to_string(Arc::new(|v| if v >= 10.0 { format!("{v:.1}") } else { format!("{v:.2}") })),
            fb_tone: hz("Feedback Tone", 6000.0, 50.0, 20000.0),
            fb_tap: choice("Feedback Tap", 0, &labels::FB_TAPS),
            scream: BoolParam::new("Filter Scream", false),
            env_attack: ms("Env Attack", 5.0, 0.1, 100.0),
            env_release: ms("Env Release", 150.0, 1.0, 2000.0),
            env_gain: db("Env Gain", 0.0, -24.0, 24.0),
            // En el instrumento no hay entrada: su toma por defecto es PRE-FILTER.
            env_tap: choice("Env Tap", if gen { 1 } else { 0 }, if gen { &labels::ENV_TAPS_GEN } else { &labels::ENV_TAPS }),
            spread: percent("Stereo Spread", 0.0),
            mod_src: [ModSrcParams::new(0), ModSrcParams::new(1), ModSrcParams::new(2)],
        }
    }

    /// Traduce los parámetros a la estructura del motor DSP.
    pub fn to_engine(&self) -> noisegek_dsp::Params {
        use noisegek_dsp::{Clock, Direction, EnvTap, FilterMode, LaneParams, LaneSource, Module, NoiseType};
        let mut p = noisegek_dsp::Params::default();
        for (i, s) in self.slots.iter().enumerate() {
            p.slot_module[i] = Module::from_index(s.module.value() as usize);
            p.slot_a[i] = s.a.value() as f64;
            p.slot_b[i] = s.b.value() as f64;
        }
        p.drive_db = self.drive.value() as f64;
        p.mix = self.mix.value() as f64;
        p.out_db = self.out.value() as f64;
        p.table_seed = self.table_seed.value() as u32;
        p.noise_type = NoiseType::from_index(self.noise_type.value() as usize);
        p.noise_level_db = self.noise_level.value() as f64;
        p.noise_char = self.noise_char.value() as f64;
        p.filter_mode = FilterMode::from_index(self.filter_mode.value() as usize);
        p.cutoff_hz = self.cutoff.value() as f64;
        p.reso = self.reso.value() as f64;
        p.filter_pre = self.filter_pos.value() == 1;
        p.clock = match self.clock.value() {
            1 => Clock::HostOnly,
            2 => Clock::Free,
            _ => Clock::Auto,
        };
        p.swing = self.swing.value() as f64;
        p.seq_seed = self.seq_seed.value() as u32;
        p.gate_depth = self.gate_depth.value() as f64;
        p.gate_len = self.gate_len.value() as f64;
        p.gate_attack_ms = self.gate_attack.value() as f64;
        p.gate_release_ms = self.gate_release.value() as f64;
        p.gate_post = self.gate_pos.value() == 1;
        p.lanes[0] = LaneParams {
            rate: self.gate_rate.value() as u8,
            steps: self.gate_steps.value() as u8,
            dir: Direction::from_index(self.gate_dir.value() as usize),
            glide: 0.0,
            prob: self.gate_prob.value() as f64,
            mutate: self.gate_mutate.value() as f64,
            dest: 0,
            depth: 0.0,
            src: LaneSource::Seq,
        };
        for (i, m) in self.mods.iter().enumerate() {
            p.lanes[i + 1] = LaneParams {
                rate: m.rate.value() as u8,
                steps: m.steps.value() as u8,
                dir: Direction::from_index(m.dir.value() as usize),
                glide: m.glide.value() as f64 / 100.0,
                prob: m.prob.value() as f64,
                mutate: m.mutate.value() as f64,
                dest: m.dest.value() as u8,
                depth: m.depth.value() as f64 / 100.0,
                src: LaneSource::from_index(self.mod_src[i].src.value() as usize),
            };
        }
        p.fb.amount = self.fb_amount.value() as f64;
        p.fb.time_ms = self.fb_time.value() as f64;
        p.fb.tone_hz = self.fb_tone.value() as f64;
        p.fb.post_filter = self.fb_tap.value() == 1;
        p.scream = self.scream.value();
        p.env.attack_ms = self.env_attack.value() as f64;
        p.env.release_ms = self.env_release.value() as f64;
        p.env.gain_db = self.env_gain.value() as f64;
        p.env.tap = if self.env_tap.value() == 1 { EnvTap::PreFilter } else { EnvTap::Input };
        p.spread = self.spread.value() as f64;
        p
    }
}

// =============================================================================
// Instrumento (GEN): parámetros de la voz
// =============================================================================

#[derive(Params)]
pub struct VoiceUiParams {
    #[id = "v_atk"]
    pub attack: FloatParam,
    #[id = "v_dec"]
    pub decay: FloatParam,
    #[id = "v_sus"]
    pub sustain: FloatParam,
    #[id = "v_rel"]
    pub release: FloatParam,
    #[id = "v_vel"]
    pub velocity: FloatParam,
    #[id = "v_track"]
    pub track: FloatParam,
    #[id = "v_pitch"]
    pub pitched: BoolParam,
    #[id = "v_retrig"]
    pub retrig: BoolParam,
    #[id = "v_envpos"]
    pub env_pos: IntParam,
}

impl Default for VoiceUiParams {
    fn default() -> Self {
        Self {
            attack: ms("Attack", 2.0, 0.1, 2000.0),
            decay: ms("Decay", 250.0, 1.0, 5000.0),
            sustain: percent("Sustain", 0.7),
            release: ms("Release", 300.0, 1.0, 8000.0),
            velocity: percent("Velocity Sensitivity", 0.7),
            track: percent("Key Track", 0.0),
            pitched: BoolParam::new("Pitched", false),
            retrig: BoolParam::new("Retrigger Sequencer", true),
            env_pos: choice("Envelope Position", 1, &labels::GATE_POS),
        }
    }
}

#[derive(Params)]
pub struct GenParams {
    #[nested(group = "NOISEGEK")]
    pub core: NgParams,
    #[nested(group = "Voice")]
    pub voice: VoiceUiParams,
}

impl Default for GenParams {
    fn default() -> Self {
        Self { core: NgParams::new(true), voice: VoiceUiParams::default() }
    }
}

impl GenParams {
    pub fn to_engine(&self) -> noisegek_dsp::Params {
        let mut p = self.core.to_engine();
        // El instrumento no tiene entrada de audio: su índice 0 es la voz.
        if self.core.env_tap.value() == 0 {
            p.env.tap = noisegek_dsp::EnvTap::Voice;
        }
        let v = &self.voice;
        p.voice = noisegek_dsp::engine::VoiceParams {
            enabled: true,
            attack_ms: v.attack.value() as f64,
            decay_ms: v.decay.value() as f64,
            sustain: v.sustain.value() as f64,
            release_ms: v.release.value() as f64,
            vel_sens: v.velocity.value() as f64,
            key_track: v.track.value() as f64,
            pitched: v.pitched.value(),
            retrig: v.retrig.value(),
            env_post: v.env_pos.value() == 1,
        };
        p
    }
}

// =============================================================================
// Estado guardado: versión y migración
// =============================================================================

/// Parámetros que no existían antes de la 0.8, con su valor por defecto.
pub fn params_added_in_0_8(gen: bool) -> Vec<(&'static str, ParamValue)> {
    use ParamValue::*;
    vec![
        ("fbamt", F32(0.0)),
        ("fbtime", F32(5.0)),
        ("fbtone", F32(6000.0)),
        ("fbtap", I32(0)),
        ("scream", Bool(false)),
        ("eatk", F32(5.0)),
        ("erel", F32(150.0)),
        ("egain", F32(0.0)),
        ("etap", I32(if gen { 1 } else { 0 })),
        ("spread", F32(0.0)),
        ("src_1", I32(0)),
        ("src_2", I32(0)),
        ("src_3", I32(0)),
    ]
}

/// (mayor, menor, parche) de una versión "a.b.c"; un estado sin versión es
/// anterior a todas.
fn version_of(v: &str) -> (u32, u32, u32) {
    let mut it = v.split('.').map(|x| x.trim().parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

/// Migración explícita (se llama desde `Plugin::filter_state`). Un estado de
/// una versión anterior a la 0.8 no trae los parámetros nuevos; sin esto, al
/// cargarlo conservarían el valor que tuviera la instancia y el preset no
/// sonaría como se guardó. Se insertan con su valor por defecto, que
/// reproduce la salida de la 0.7.
///
/// Desde la 0.8.1 el lienzo es más compacto: un estado anterior pierde el
/// tamaño de ventana guardado (`editor-state`) para abrir con el tamaño inicial
/// nuevo, que cabe en pantallas de 768 px de alto. El patrón no se toca.
pub fn migrate_state(state: &mut PluginState, gen: bool) {
    let v = version_of(&state.version);
    if v < (0, 8, 0) {
        for (id, v) in params_added_in_0_8(gen) {
            state.params.entry(id.to_string()).or_insert(v);
        }
    }
    if v < (0, 8, 1) {
        state.fields.remove("editor-state");
    }
}

/// Lo que la UI necesita de cada plugin: el núcleo y, si es el GEN, la voz.
pub trait UiParams: Send + Sync + 'static {
    fn core(&self) -> &NgParams;
    fn voice(&self) -> Option<&VoiceUiParams>;
}

impl UiParams for NgParams {
    fn core(&self) -> &NgParams {
        self
    }
    fn voice(&self) -> Option<&VoiceUiParams> {
        None
    }
}

impl UiParams for GenParams {
    fn core(&self) -> &NgParams {
        &self.core
    }
    fn voice(&self) -> Option<&VoiceUiParams> {
        Some(&self.voice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// IDs de la 0.7 (README de distribución de la 0.7).
    fn ids_0_7(gen: bool) -> BTreeSet<String> {
        let mut v: Vec<String> = ["drive", "mix", "out", "tseed", "ntype", "nlevel", "nchar", "fmode", "cutoff", "reso",
            "fpos", "clock", "swing", "sseed", "gdepth", "glen", "gatk", "grel", "gpos", "grate", "gsteps", "gdir",
            "gprob", "gmut"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        for n in 1..=4 {
            for p in ["module", "a", "b"] {
                v.push(format!("{p}_{n}"));
            }
        }
        for n in 1..=3 {
            for p in ["dest", "depth", "rate", "steps", "dir", "glide", "prob", "mutate"] {
                v.push(format!("{p}_{n}"));
            }
        }
        if gen {
            for p in ["v_atk", "v_dec", "v_sus", "v_rel", "v_vel", "v_track", "v_pitch", "v_retrig", "v_envpos"] {
                v.push(p.to_string());
            }
        }
        v.into_iter().collect()
    }

    fn param_ids<P: Params>(p: &P) -> Vec<String> {
        p.param_map().into_iter().map(|(id, _, _)| id).collect()
    }

    /// Los IDs de la 0.7 siguen existiendo y los nuevos son exactamente los de
    /// `params_added_in_0_8`, al final de la lista.
    #[test]
    fn ids_are_stable_and_new_ones_go_last() {
        for gen in [false, true] {
            let ids = if gen { param_ids(&GenParams::default()) } else { param_ids(&NgParams::new(false)) };
            let old = ids_0_7(gen);
            let new: Vec<String> = params_added_in_0_8(gen).iter().map(|(id, _)| id.to_string()).collect();
            let all: BTreeSet<String> = ids.iter().cloned().collect();
            assert!(old.is_subset(&all), "faltan IDs de la 0.7");
            let expected: BTreeSet<String> = old.iter().cloned().chain(new.iter().cloned()).collect();
            assert_eq!(all, expected, "gen = {gen}");
            // Orden: en el efecto, los nuevos son los últimos de la lista.
            if !gen {
                let tail: BTreeSet<String> = ids[ids.len() - new.len()..].iter().cloned().collect();
                assert_eq!(tail, new.iter().cloned().collect::<BTreeSet<_>>());
            }
        }
    }

    /// Los valores que inserta la migración son los valores por defecto reales.
    #[test]
    fn migration_defaults_match_params() {
        for gen in [false, true] {
            let gp = GenParams::default();
            let ep = NgParams::new(false);
            let map = if gen { gp.param_map() } else { ep.param_map() };
            for (id, v) in params_added_in_0_8(gen) {
                let (_, ptr, _) = map.iter().find(|(i, _, _)| i == id).expect(id);
                let def = unsafe { ptr.preview_plain(ptr.default_normalized_value()) };
                let want = match v {
                    ParamValue::F32(x) => x,
                    ParamValue::I32(x) => x as f32,
                    ParamValue::Bool(b) => b as i32 as f32,
                    _ => unreachable!(),
                };
                assert!((def - want).abs() <= want.abs() * 1e-5, "{id}: {def} != {want}");
            }
        }
    }

    /// Un estado de la 0.6/0.7 recibe los parámetros nuevos con su valor por
    /// defecto; uno de la 0.8 no se toca.
    #[test]
    fn old_states_are_migrated() {
        for version in ["0.6.0", "0.7.0", ""] {
            let mut s = PluginState {
                version: version.to_string(),
                params: [("drive".to_string(), ParamValue::F32(6.0))].into_iter().collect(),
                fields: Default::default(),
            };
            migrate_state(&mut s, false);
            assert_eq!(s.params.len(), 1 + params_added_in_0_8(false).len());
            assert!(matches!(s.params["fbamt"], ParamValue::F32(x) if x == 0.0));
            assert!(matches!(s.params["drive"], ParamValue::F32(x) if x == 6.0));
            let mut g = s.clone();
            g.params.remove("etap");
            migrate_state(&mut g, true);
            assert!(matches!(g.params["etap"], ParamValue::I32(1)));
        }
        let mut s = PluginState {
            version: "0.8.0".into(),
            params: [("fbamt".to_string(), ParamValue::F32(0.9))].into_iter().collect(),
            fields: Default::default(),
        };
        migrate_state(&mut s, false);
        assert_eq!(s.params.len(), 1);
    }

    /// Hasta la 0.8.0 se descarta el tamaño de ventana guardado; el patrón se
    /// conserva. Desde la 0.8.1 no se toca nada.
    #[test]
    fn old_window_size_is_dropped() {
        for (version, keeps) in [("0.7.0", false), ("0.8.0", false), ("0.8.1", true), ("0.9.0", true)] {
            let mut s = PluginState {
                version: version.into(),
                params: Default::default(),
                fields: [("editor-state".to_string(), "{}".to_string()), ("pattern".to_string(), "p".to_string())]
                    .into_iter()
                    .collect(),
            };
            migrate_state(&mut s, false);
            assert_eq!(s.fields.contains_key("editor-state"), keeps, "{version}");
            assert_eq!(s.fields["pattern"], "p");
        }
    }
}
