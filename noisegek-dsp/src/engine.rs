//! Motor completo de NOISEGEK. Traducción directa del JSFX v0.5.
//!
//! Flujo por canal:
//!   in + ruido -> [gate pre] -> [filtro pre] -> drive -> slot1..4
//!      -> [filtro post] -> mix dry/wet -> [gate post] -> paneo -> out gain
//!      -> DC blocker (~10 Hz) -> limitador duro -0.3 dBFS
//! Realimentación (0.8): la salida de los slots (o del filtro post) vuelve a
//! la entrada del slot 1 por delay -> tono -> DC blocker -> ganancia -> tanh.

use crate::filter::{FilterMode, Svf, SvfCoefs};
use crate::modules::{self, Module, SlotState, Tables};
use crate::noise::{NoiseCoefs, NoiseSource, NoiseType};
use crate::rng::{hash01, XorShift};
use crate::seq::{dest, rate_spq, Clock, Direction, LaneParams, LaneSource, Pattern, NUM_DEST, NUM_LANES, NUM_STEPS};

pub const NUM_SLOTS: usize = 4;
const SMOOTH_MS: f64 = 4.0;
const FADE_MS: f64 = 10.0;
const DC_HZ: f64 = 10.0;
const LIMIT_DB: f64 = -0.3;
const CTRL_N: u32 = 16;
/// Umbral de reposicionamiento: con el reloj sincronizado, si la posición del
/// host se aparta más de esto (en muestras) de la prevista, es un salto.
pub const JUMP_SAMPLES: f64 = 64.0;

/// Frecuencia de muestreo máxima para la que se reserva el delay del bucle.
pub const MAX_SR: f64 = 384_000.0;
/// FB TIME máximo (ms) por el factor de SPREAD en el canal R (1 + 0.3).
const FB_MAX_MS: f64 = 50.0 * 1.3;
/// Tamaño del delay del bucle (potencia de 2 > MAX_SR · 65 ms + 2).
const FB_LEN: usize = 32_768;
const FB_MASK: usize = FB_LEN - 1;
const _: () = assert!(FB_LEN as f64 > MAX_SR * FB_MAX_MS * 0.001 + 2.0);
/// Valor absoluto a partir del cual el bucle se considera desbocado (NaN/inf).
const FB_GUARD: f64 = 1e30;
/// Por debajo de esto, FB AMOUNT suavizado con objetivo 0 se da por 0.
const FB_OFF_EPS: f64 = 1e-6;

/// Realimentación global (0.8). `amount = 0` deja el bucle desconectado.
#[derive(Clone, Copy, Debug)]
pub struct FeedbackParams {
    /// 0..1.5 (sobre 1 el bucle es inestable y lo contiene el tanh)
    pub amount: f64,
    /// 0.05..50 ms
    pub time_ms: f64,
    /// corte del filtro de un polo dentro del bucle (Hz)
    pub tone_hz: f64,
    /// toma después del filtro principal (true) o a la salida de los slots
    pub post_filter: bool,
}

/// Punto de toma del seguidor de envolvente (0.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnvTap {
    /// Entrada seca del efecto (no existe en el instrumento).
    #[default]
    Input,
    /// Salida de los slots: automodulación.
    PreFilter,
    /// Envolvente de la voz (ADSR × velocidad). Solo en el instrumento.
    Voice,
}

/// Seguidor de envolvente (0.8): fuente ENV de los carriles MOD.
#[derive(Clone, Copy, Debug)]
pub struct EnvParams {
    pub attack_ms: f64,
    pub release_ms: f64,
    pub gain_db: f64,
    pub tap: EnvTap,
}

impl Default for EnvParams {
    fn default() -> Self {
        Self { attack_ms: 5.0, release_ms: 150.0, gain_db: 0.0, tap: EnvTap::Input }
    }
}

impl Default for FeedbackParams {
    fn default() -> Self {
        Self { amount: 0.0, time_ms: 5.0, tone_hz: 6000.0, post_filter: false }
    }
}

/// Modo instrumento (NOISEGEK GEN): notas MIDI, envolvente y seguimiento de
/// teclado. Con `enabled = false` el motor es exactamente el efecto.
#[derive(Clone, Copy, Debug)]
pub struct VoiceParams {
    pub enabled: bool,
    pub attack_ms: f64,
    pub decay_ms: f64,
    /// 0..1
    pub sustain: f64,
    pub release_ms: f64,
    /// 0..1: cuánto afecta la velocidad al volumen
    pub vel_sens: f64,
    /// 0..1: el cutoff sigue la nota (1 = una octava por octava)
    pub key_track: f64,
    /// LFSR en modo tonal y Ring Mod afinados a la nota
    pub pitched: bool,
    /// Cada nota reinicia el secuenciador
    pub retrig: bool,
    /// Envolvente al final (true) o antes de la cadena (false)
    pub env_post: bool,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            enabled: false,
            attack_ms: 2.0,
            decay_ms: 250.0,
            sustain: 0.7,
            release_ms: 300.0,
            vel_sens: 0.7,
            key_track: 0.0,
            pitched: false,
            retrig: true,
            env_post: true,
        }
    }
}

/// Valores "planos" de todos los parámetros (lo que en el JSFX son sliders).
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub slot_module: [Module; NUM_SLOTS],
    pub slot_a: [f64; NUM_SLOTS],
    pub slot_b: [f64; NUM_SLOTS],
    pub drive_db: f64,
    /// 0..1
    pub mix: f64,
    pub out_db: f64,
    pub table_seed: u32,
    pub noise_type: NoiseType,
    /// -60 = apagado
    pub noise_level_db: f64,
    pub noise_char: f64,
    pub filter_mode: FilterMode,
    pub cutoff_hz: f64,
    /// 0..1
    pub reso: f64,
    pub filter_pre: bool,
    pub clock: Clock,
    /// 50..75 (%)
    pub swing: f64,
    pub seq_seed: u32,
    /// 0..1
    pub gate_depth: f64,
    /// 0..1
    pub gate_len: f64,
    pub gate_attack_ms: f64,
    pub gate_release_ms: f64,
    pub gate_post: bool,
    pub lanes: [LaneParams; NUM_LANES],
    pub voice: VoiceParams,
    pub fb: FeedbackParams,
    /// Filtro autooscilante (0.8)
    pub scream: bool,
    pub env: EnvParams,
    /// 0..1: diferencias entre L y R (0 = idéntico a la 0.7)
    pub spread: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            slot_module: [Module::SoftClip, Module::Off, Module::Off, Module::Off],
            slot_a: [0.5; 4],
            slot_b: [0.5; 4],
            drive_db: 0.0,
            mix: 1.0,
            out_db: 0.0,
            table_seed: 1234,
            noise_type: NoiseType::Off,
            noise_level_db: -18.0,
            noise_char: 0.5,
            filter_mode: FilterMode::Off,
            cutoff_hz: 2000.0,
            reso: 0.2,
            filter_pre: false,
            clock: Clock::Auto,
            swing: 50.0,
            seq_seed: 0,
            gate_depth: 0.0,
            gate_len: 0.5,
            gate_attack_ms: 1.0,
            gate_release_ms: 40.0,
            gate_post: false,
            lanes: [
                LaneParams::new(9, 16, Direction::Forward, 0.0),
                LaneParams::new(9, 16, Direction::Forward, 0.0),
                LaneParams::new(7, 12, Direction::Forward, 0.3),
                LaneParams::new(3, 7, Direction::PingPong, 1.0),
            ],
            voice: VoiceParams::default(),
            fb: FeedbackParams::default(),
            scream: false,
            env: EnvParams::default(),
            spread: 0.0,
        }
    }
}

/// Estado del transporte del host para un bloque.
#[derive(Clone, Copy, Debug)]
pub struct Transport {
    pub playing: bool,
    /// Posición en negras al inicio del bloque (si el host la da).
    pub pos_qn: Option<f64>,
    pub tempo: f64,
    pub ts_num: f64,
    pub ts_den: f64,
}

impl Default for Transport {
    fn default() -> Self {
        Self { playing: false, pos_qn: None, tempo: 120.0, ts_num: 4.0, ts_den: 4.0 }
    }
}

/// Lo que la UI necesita mostrar en vivo.
#[derive(Clone, Copy, Debug, Default)]
pub struct Snapshot {
    pub seq_running: bool,
    pub synced: bool,
    pub lane_idx: [u8; NUM_LANES],
    pub lane_fired: [bool; NUM_LANES],
    pub lane_val: [f32; NUM_LANES],
    pub gate_env: f32,
    pub slot_active: [bool; NUM_SLOTS],
    pub slot_seq_on: [bool; NUM_SLOTS],
    pub modsum: [f32; NUM_DEST],
    /// Seguidor de envolvente, 0..1 (0.8)
    pub env_follow: f32,
    /// Envolvente del modo instrumento (0..1)
    pub voice_env: f32,
    /// Nota sonando (-1 = ninguna)
    pub note: i16,
}

#[derive(Clone, Copy, Debug)]
struct Lane {
    spq: f64,
    n: i64,
    idx: usize,
    fired: bool,
    phase: f64,
    tgt: f64,
    val: f64,
    k: f64,
}

impl Default for Lane {
    fn default() -> Self {
        Self { spq: 4.0, n: i64::MIN, idx: 0, fired: false, phase: 0.0, tgt: 0.0, val: 0.0, k: 1.0 }
    }
}

pub struct Engine {
    sr: f64,
    sm_k: f64,
    fade_step: f64,
    dc_r: f64,
    lim: f64,
    p: Params,
    tempo: f64,

    tables: Tables,
    req_mod: [Module; NUM_SLOTS],
    cur_mod: [Module; NUM_SLOTS],
    slot_fade: [f64; NUM_SLOTS],
    slot_on: [bool; NUM_SLOTS],
    p_tgt: [(f64, f64); NUM_SLOTS],
    p_sm: [(f64, f64); NUM_SLOTS],
    slot_state: [[SlotState; 2]; NUM_SLOTS],

    drive: (f64, f64),
    mix: (f64, f64),
    out: (f64, f64),
    cut: (f64, f64),
    reso: (f64, f64),
    noise_lvl: (f64, f64),
    pan: (f64, f64),

    noise: NoiseSource,
    noise_coefs: NoiseCoefs,

    // --- Realimentación ---
    fb_amt: (f64, f64),
    fb_time: (f64, f64),
    spread: (f64, f64),
    fb_on: bool,
    fb_tone_a: f64,
    fb_buf: [Vec<f64>; 2],
    fb_w: usize,
    fb_tone: [f64; 2],
    fb_dc_x1: [f64; 2],
    fb_dc_y1: [f64; 2],

    // --- Seguidor de envolvente ---
    env_e: f64,
    env_ka: f64,
    env_kr: f64,
    env_g: f64,
    svf: [Svf; 2],
    dc_x1: [f64; 2],
    dc_y1: [f64; 2],

    qn: f64,
    qn_inc: f64,
    seq_running: bool,
    was_host: bool,
    last_qn_host: f64,
    last_block_n: usize,
    /// Avance real del host (negras por muestra) entre los dos últimos bloques
    /// sincronizados sin salto; `None` justo después de una resincronización.
    host_rate: Option<f64>,
    /// Resincronizaciones desde la creación (para las pruebas).
    resyncs: u64,
    swing_b: f64,
    gate_att_k: f64,
    gate_rel_k: f64,
    gate_env: f64,
    gate_gain: f64,
    lanes: [Lane; NUM_LANES],
    modsum: [f64; NUM_DEST],
    ctrl_count: u32,
    rnd: XorShift,

    /// Patrón que suena. La mutación lo modifica: ver `pattern_mutated`.
    pub pattern: Pattern,
    /// true si la mutación cambió el patrón desde la última vez que se leyó.
    pub pattern_mutated: bool,
    init_pending: bool,

    // --- Modo instrumento ---
    notes: [u8; 16],
    n_notes: usize,
    note: Option<u8>,
    velocity: f64,
    env: f64,
    /// 0 = reposo, 1 = ataque, 2 = caída, 3 = sostenido, 4 = release
    env_stage: u8,
    env_att_inc: f64,
    env_dec_k: f64,
    env_rel_k: f64,
    /// Posición (en negras) que el secuenciador toma como "paso 0".
    seq_origin: f64,
}

#[inline]
fn db_to_gain(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Estado inicial de los slots: el xorshift del TRIGGER S/H del slot s, canal
/// c, se siembra con 12345 + 7919·(2s + c).
fn initial_slot_state() -> [[SlotState; 2]; NUM_SLOTS] {
    let mut slot_state = [[SlotState::new(1); 2]; NUM_SLOTS];
    for (s, st) in slot_state.iter_mut().enumerate() {
        for (c, ch) in st.iter_mut().enumerate() {
            *ch = SlotState::new(12345 + (s * 2 + c) as u32 * 7919);
        }
    }
    slot_state
}

impl Engine {
    /// Crea el motor. Reserva aquí (fuera del audio) las tablas y el delay del
    /// bucle de realimentación, dimensionado para `MAX_SR`.
    pub fn new(sr: f64) -> Self {
        let tables = Tables::default();
        let fb_buf = [vec![0.0; FB_LEN], vec![0.0; FB_LEN]];
        Self::build(sr, tables, fb_buf)
    }

    fn build(sr: f64, tables: Tables, fb_buf: [Vec<f64>; 2]) -> Self {
        let slot_state = initial_slot_state();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(1);
        let mut e = Self {
            sr,
            sm_k: 0.0,
            fade_step: 0.0,
            dc_r: 0.0,
            lim: db_to_gain(LIMIT_DB),
            p: Params::default(),
            tempo: 120.0,
            tables,
            req_mod: [Module::Off; NUM_SLOTS],
            cur_mod: [Module::Off; NUM_SLOTS],
            slot_fade: [1.0; NUM_SLOTS],
            slot_on: [true; NUM_SLOTS],
            p_tgt: [(0.5, 0.5); NUM_SLOTS],
            p_sm: [(0.5, 0.5); NUM_SLOTS],
            slot_state,
            drive: (1.0, 1.0),
            mix: (1.0, 1.0),
            out: (1.0, 1.0),
            cut: (2000.0, 2000.0),
            reso: (0.2, 0.2),
            noise_lvl: (0.0, 0.0),
            pan: (0.0, 0.0),
            noise: NoiseSource::default(),
            noise_coefs: NoiseCoefs::default(),
            fb_amt: (0.0, 0.0),
            fb_time: (5.0, 5.0),
            spread: (0.0, 0.0),
            fb_on: false,
            fb_tone_a: 1.0,
            fb_buf,
            fb_w: 0,
            fb_tone: [0.0; 2],
            fb_dc_x1: [0.0; 2],
            fb_dc_y1: [0.0; 2],
            env_e: 0.0,
            env_ka: 1.0,
            env_kr: 1.0,
            env_g: 1.0,
            svf: [Svf::default(); 2],
            dc_x1: [0.0; 2],
            dc_y1: [0.0; 2],
            qn: 0.0,
            qn_inc: 0.0,
            seq_running: false,
            was_host: false,
            last_qn_host: 0.0,
            last_block_n: 0,
            host_rate: None,
            resyncs: 0,
            swing_b: 1.0,
            gate_att_k: 1.0,
            gate_rel_k: 1.0,
            gate_env: 1.0,
            gate_gain: 1.0,
            lanes: [Lane::default(); NUM_LANES],
            modsum: [0.0; NUM_DEST],
            ctrl_count: 0,
            rnd: XorShift::new(seed),
            pattern: Pattern::default(),
            pattern_mutated: false,
            init_pending: true,
            notes: [0; 16],
            n_notes: 0,
            note: None,
            velocity: 1.0,
            env: 0.0,
            env_stage: 0,
            env_att_inc: 1.0,
            env_dec_k: 1.0,
            env_rel_k: 1.0,
            seq_origin: 0.0,
        };
        e.set_sample_rate(sr);
        e
    }

    pub fn set_sample_rate(&mut self, sr: f64) {
        self.sr = sr;
        self.sm_k = 1.0 - (-1.0 / (SMOOTH_MS * 0.001 * sr)).exp();
        self.fade_step = 1.0 / (FADE_MS * 0.001 * sr);
        self.dc_r = 1.0 - 2.0 * std::f64::consts::PI * DC_HZ / sr;
        self.init_pending = true;
    }

    /// Reinicia todo el estado de audio (no el patrón). No asigna memoria:
    /// reutiliza las tablas y el delay del bucle.
    pub fn reset(&mut self) {
        let pattern = self.pattern;
        let p = self.p;
        let empty = Tables { raw: Vec::new(), smooth: Vec::new(), seed: None };
        let tables = std::mem::replace(&mut self.tables, empty);
        let mut fb_buf = std::mem::take(&mut self.fb_buf);
        for b in fb_buf.iter_mut() {
            b.fill(0.0);
        }
        *self = Self::build(self.sr, tables, fb_buf);
        self.pattern = pattern;
        self.p = p;
    }

    /// Llamar al inicio de cada bloque, antes de `process_frame`.
    pub fn begin_block(&mut self, p: &Params, t: &Transport, nframes: usize) {
        self.p = *p;
        self.tempo = t.tempo.max(1.0);

        for (l, lane) in self.lanes.iter_mut().enumerate() {
            lane.spq = rate_spq(p.lanes[l].rate, t.ts_num, t.ts_den);
        }
        let ncoef_char = p.noise_char;
        self.noise_coefs = NoiseCoefs::new(ncoef_char, self.sr);
        self.swing_b = p.swing / 50.0;
        self.gate_att_k = 1.0 - (-1.0 / (p.gate_attack_ms * 0.001 * self.sr)).exp();
        self.gate_rel_k = 1.0 - (-1.0 / (p.gate_release_ms * 0.001 * self.sr)).exp();
        self.tables.ensure(p.table_seed);
        let e = &p.env;
        self.env_ka = 1.0 - (-1.0 / (e.attack_ms.max(0.01) * 0.001 * self.sr)).exp();
        self.env_kr = 1.0 - (-1.0 / (e.release_ms.max(0.01) * 0.001 * self.sr)).exp();
        self.env_g = db_to_gain(e.gain_db);
        self.fb_tone_a = 1.0 - (-2.0 * std::f64::consts::PI * p.fb.tone_hz.min(0.45 * self.sr) / self.sr).exp();

        let v = &p.voice;
        let ms_k = |ms: f64| 1.0 - (-1.0 / (ms.max(0.1) * 0.001 * self.sr)).exp();
        self.env_att_inc = 1.0 / (v.attack_ms.max(0.1) * 0.001 * self.sr);
        self.env_dec_k = ms_k(v.decay_ms);
        self.env_rel_k = ms_k(v.release_ms);
        if !v.retrig {
            self.seq_origin = 0.0;
        }

        // Reloj (igual que @block del JSFX)
        let free_inc = self.tempo / 60.0 / self.sr;
        match (t.playing, t.pos_qn, p.clock) {
            (true, Some(qn_host), c) if c != Clock::Free => {
                let d_qn = qn_host - self.last_qn_host;
                let n_prev = self.last_block_n as f64;
                // Inicio de reproducción o reposicionamiento (ver `resync`).
                let jump = if !self.was_host {
                    true
                } else if let Some(rate) = self.host_rate {
                    (qn_host - (self.last_qn_host + n_prev * rate)).abs() > JUMP_SAMPLES * rate
                } else {
                    // Sin avance medido todavía: solo se reconoce el retroceso.
                    self.last_block_n > 0 && d_qn <= 0.0
                };
                if jump {
                    self.host_rate = None;
                } else if self.last_block_n > 0 && d_qn > 0.0 {
                    self.host_rate = Some(d_qn / n_prev);
                }
                let calibrated = self.was_host
                    && d_qn > 0.0
                    && self.last_block_n > 0
                    && (d_qn / self.last_block_n as f64 - free_inc).abs() < free_inc * 0.5;
                self.qn_inc = if calibrated { d_qn / self.last_block_n as f64 } else { free_inc };
                self.qn = qn_host;
                self.last_qn_host = qn_host;
                self.was_host = true;
                self.seq_running = true;
                if jump {
                    self.resync();
                }
            }
            _ => {
                self.was_host = false;
                self.host_rate = None;
                self.qn_inc = free_inc;
                self.seq_running = p.clock != Clock::HostOnly;
            }
        }
        self.last_block_n = nframes;

        if self.init_pending {
            self.apply_mod_and_snap();
            self.init_pending = false;
        }
    }

    /// Evalúa la modulación y lleva los valores suavizados directamente a su
    /// objetivo (sin rampa).
    fn apply_mod_and_snap(&mut self) {
        self.apply_mod();
        self.drive.1 = self.drive.0;
        self.mix.1 = self.mix.0;
        self.out.1 = self.out.0;
        self.cut.1 = self.cut.0;
        self.reso.1 = self.reso.0;
        self.noise_lvl.1 = self.noise_lvl.0;
        self.pan.1 = self.pan.0;
        self.fb_amt.1 = self.fb_amt.0;
        self.fb_time.1 = self.fb_time.0;
        self.spread.1 = self.spread.0;
        for s in 0..NUM_SLOTS {
            self.cur_mod[s] = self.req_mod[s];
            self.p_sm[s] = self.p_tgt[s];
        }
    }

    /// Resincronización (espec. 9.7). Se llama, con el reloj sincronizado al
    /// host, cuando el transporte empieza a correr y cuando la posición salta
    /// más de `JUMP_SAMPLES`. Devuelve a su estado inicial todo lo que depende
    /// del número de muestras procesadas o de la señal anterior, para que la
    /// salida a partir de aquí dependa solo de la posición:
    ///   * ruido: LCG, LFSR (registro y fase), filtro de Kellet, BROWN, DUST;
    ///   * slots: estado y xorshift del TRIGGER S/H, fase del RING MOD, etc.;
    ///   * filtro SVF y DC blocker;
    ///   * carriles (DRUNK parte del índice 0), gate desde 0, contador de
    ///     control a 0, suavizados en su objetivo.
    /// Los carriles se evalúan ya en la posición nueva y los MOD arrancan en el
    /// valor de su primer paso (sin glide). No se tocan el patrón, el
    /// generador de la mutación, las tablas ni la voz del instrumento.
    fn resync(&mut self) {
        self.noise = NoiseSource::default();
        self.slot_state = initial_slot_state();
        self.slot_fade = [1.0; NUM_SLOTS];
        self.svf = [Svf::default(); 2];
        self.dc_x1 = [0.0; 2];
        self.dc_y1 = [0.0; 2];
        self.fb_clear();
        self.env_e = 0.0;
        for lane in self.lanes.iter_mut() {
            *lane = Lane { spq: lane.spq, ..Lane::default() };
        }
        for l in 0..NUM_LANES {
            self.lane_clock(l);
        }
        for lane in self.lanes.iter_mut().skip(1) {
            lane.val = lane.tgt;
        }
        self.gate_env = 0.0;
        self.ctrl_count = 0;
        self.apply_mod_and_snap();
        self.init_pending = false;
        self.resyncs += 1;
    }

    // ----- Secuenciador ---------------------------------------------------

    fn lane_pick(&self, l: usize, n: i64) -> usize {
        let len = self.p.lanes[l].steps.clamp(1, NUM_STEPS as u8) as i64;
        let w = |x: i64, m: i64| x.rem_euclid(m);
        let seed = self.p.seq_seed;
        (match self.p.lanes[l].dir {
            Direction::Forward => w(n, len),
            Direction::Backward => len - 1 - w(n, len),
            Direction::PingPong => {
                if len < 2 {
                    0
                } else {
                    let p = w(n, 2 * len - 2);
                    if p < len {
                        p
                    } else {
                        2 * len - 2 - p
                    }
                }
            }
            Direction::Random => (hash01(n, l as u32, 1, seed) * len as f64).floor() as i64,
            Direction::Drunk => {
                let step = if hash01(n, l as u32, 2, seed) < 0.5 { -1 } else { 1 };
                w(self.lanes[l].idx as i64 + step, len)
            }
        }) as usize
    }

    fn lane_step(&mut self, l: usize, n: i64) {
        let idx = self.lane_pick(l, n);
        let lp = self.p.lanes[l];
        let fired = hash01(n, l as u32, 3, self.p.seq_seed) * 100.0 < lp.prob;
        if fired {
            if self.rnd.next01() * 100.0 < lp.mutate {
                let v = self.pattern.steps[l][idx] as f64 + (self.rnd.next01() - 0.5) * 0.6;
                self.pattern.steps[l][idx] = v.clamp(0.0, 1.0) as f32;
                self.pattern_mutated = true;
            }
            self.lanes[l].tgt = self.pattern.steps[l][idx] as f64;
        } else if l == 0 {
            self.lanes[0].tgt = 0.0;
        }
        let lane = &mut self.lanes[l];
        lane.fired = fired;
        lane.idx = idx;
        lane.n = n;
        let step_s = 60.0 / (self.tempo * lane.spq);
        let tau = (lp.glide * step_s * 0.5).max(0.0005);
        lane.k = 1.0 - (-1.0 / (tau * self.sr)).exp();
    }

    fn lane_clock(&mut self, l: usize) {
        let t = (self.qn - self.seq_origin) * self.lanes[l].spq;
        let pair = (t * 0.5).floor();
        let ph = t - pair * 2.0;
        let sb = self.swing_b;
        let n = if ph < sb {
            self.lanes[l].phase = ph / sb;
            pair as i64 * 2
        } else {
            self.lanes[l].phase = (ph - sb) / (2.0 - sb);
            pair as i64 * 2 + 1
        };
        if n != self.lanes[l].n {
            self.lane_step(l, n);
        }
    }

    fn seq_sample(&mut self) {
        let tgt = if self.seq_running {
            for l in 0..NUM_LANES {
                self.lane_clock(l);
            }
            self.qn += self.qn_inc;
            if self.lanes[0].fired && self.lanes[0].phase < self.p.gate_len {
                self.lanes[0].tgt
            } else {
                0.0
            }
        } else {
            1.0
        };
        let k = if tgt > self.gate_env { self.gate_att_k } else { self.gate_rel_k };
        self.gate_env += (tgt - self.gate_env) * k;
        self.gate_gain = 1.0 - self.p.gate_depth + self.p.gate_depth * self.gate_env;
        for lane in self.lanes.iter_mut().skip(1) {
            lane.val += (lane.tgt - lane.val) * lane.k;
        }
    }

    // ----- Modulación -> objetivos ---------------------------------------

    fn apply_mod(&mut self) {
        self.modsum = [0.0; NUM_DEST];
        self.slot_on = [true; NUM_SLOTS];
        for l in 1..NUM_LANES {
            let lp = self.p.lanes[l];
            let env = self.env_e.min(1.0);
            let v = match lp.src {
                LaneSource::Seq => self.lanes[l].val,
                LaneSource::Env => env,
                LaneSource::SeqEnv => self.lanes[l].val * env,
            };
            let d = lp.dest;
            if (1..=13).contains(&d) || (dest::FB_AMOUNT as usize..NUM_DEST).contains(&(d as usize)) {
                self.modsum[d as usize] += lp.depth * v;
            } else if d == dest::PAN {
                self.modsum[d as usize] += lp.depth * (2.0 * v - 1.0);
            } else if (dest::SLOT_ON_FIRST..=dest::SLOT_ON_LAST).contains(&d) && lp.depth != 0.0 {
                self.slot_on[(d - dest::SLOT_ON_FIRST) as usize] =
                    if lp.depth > 0.0 { v >= 0.5 } else { v < 0.5 };
            }
        }
        for s in 0..NUM_SLOTS {
            let m = self.p.slot_module[s];
            let a = (self.p.slot_a[s] + self.modsum[1 + s * 2]).clamp(0.0, 1.0);
            let b = (self.p.slot_b[s] + self.modsum[2 + s * 2]).clamp(0.0, 1.0);
            self.req_mod[s] = m;
            self.p_tgt[s] = modules::map_params(m, a, b, self.sr);
        }
        let p = &self.p;
        self.drive.0 = db_to_gain((p.drive_db + self.modsum[9] * 48.0).clamp(-24.0, 48.0));
        self.mix.0 = (p.mix + self.modsum[10]).clamp(0.0, 1.0);
        self.out.0 = db_to_gain(p.out_db);
        let cut_base = (p.cutoff_hz / 20.0).ln() / 1000f64.ln();
        self.cut.0 = 20.0 * 1000f64.powf((cut_base + self.modsum[11]).clamp(0.0, 1.0));
        self.reso.0 = (p.reso + self.modsum[12]).clamp(0.0, 1.0);
        let db = p.noise_level_db + self.modsum[13] * 60.0;
        self.noise_lvl.0 =
            if db <= -60.0 || p.noise_type == NoiseType::Off { 0.0 } else { db_to_gain(db.min(0.0)) };
        self.pan.0 = self.modsum[14].clamp(-1.0, 1.0);
        self.fb_amt.0 = (p.fb.amount + 1.5 * self.modsum[dest::FB_AMOUNT as usize]).clamp(0.0, 1.5);
        let t_base = (p.fb.time_ms / 0.05).ln() / 1000f64.ln();
        self.fb_time.0 = 0.05 * 1000f64.powf((t_base + self.modsum[dest::FB_TIME as usize]).clamp(0.0, 1.0));
        self.spread.0 = (p.spread + self.modsum[dest::SPREAD as usize]).clamp(0.0, 1.0);

        // Modo instrumento: seguimiento de teclado y afinación.
        self.noise_coefs.lfsr_short = false;
        if let (true, Some(n)) = (self.p.voice.enabled, self.note) {
            let semis = n as f64 - 60.0;
            self.cut.0 = (self.cut.0 * 2f64.powf(semis / 12.0 * self.p.voice.key_track)).clamp(20.0, 20000.0);
            if self.p.voice.pitched {
                let hz = 440.0 * 2f64.powf((n as f64 - 69.0) / 12.0);
                // LFSR corto: 93 pasos por ciclo -> tono a reloj/93
                self.noise_coefs.lfsr_short = true;
                self.noise_coefs.lfsr_inc = (hz * 93.0 / self.sr).min(4.0);
                for s in 0..NUM_SLOTS {
                    if self.p.slot_module[s] == Module::RingMod {
                        self.p_tgt[s].0 = hz / self.sr;
                    }
                }
            }
        }
    }

    // ----- Notas (modo instrumento) ---------------------------------------

    /// Nota nueva: prioridad a la última; reinicia la envolvente (desde su
    /// nivel actual, sin clic) y, con RETRIG, el secuenciador.
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.remove_note(note);
        if self.n_notes == self.notes.len() {
            self.notes.copy_within(1.., 0);
            self.n_notes -= 1;
        }
        self.notes[self.n_notes] = note;
        self.n_notes += 1;
        self.note = Some(note);
        self.velocity = velocity.clamp(0.0, 1.0) as f64;
        self.env_stage = 1;
        if self.p.voice.retrig {
            // La fuente de ruido (LCG, LFSR, Kellet, BROWN, DUST) arranca igual
            // en cada nota.
            self.noise = NoiseSource::default();
            self.seq_origin = self.qn;
            for lane in self.lanes.iter_mut() {
                lane.n = i64::MIN;
                lane.idx = 0;
            }
        }
        self.ctrl_count = 0; // aplicar la afinación ya mismo
    }

    /// Soltar nota: si quedan otras apretadas vuelve a la anterior (legato).
    pub fn note_off(&mut self, note: u8) {
        self.remove_note(note);
        if self.n_notes > 0 {
            self.note = Some(self.notes[self.n_notes - 1]);
            self.ctrl_count = 0;
        } else if self.env_stage != 0 {
            self.env_stage = 4;
        }
    }

    pub fn all_notes_off(&mut self) {
        self.n_notes = 0;
        if self.env_stage != 0 {
            self.env_stage = 4;
        }
    }

    fn remove_note(&mut self, note: u8) {
        if let Some(i) = self.notes[..self.n_notes].iter().position(|&n| n == note) {
            self.notes.copy_within(i + 1..self.n_notes, i);
            self.n_notes -= 1;
        }
    }

    #[inline]
    fn voice_gain(&mut self) -> f64 {
        let v = self.p.voice;
        match self.env_stage {
            1 => {
                self.env += self.env_att_inc;
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.env_stage = 2;
                }
            }
            2 => {
                self.env += (v.sustain - self.env) * self.env_dec_k;
                if (self.env - v.sustain).abs() < 1e-4 {
                    self.env_stage = 3;
                }
            }
            3 => self.env = v.sustain,
            4 => {
                self.env -= self.env * self.env_rel_k;
                if self.env < 1e-5 {
                    self.env = 0.0;
                    self.env_stage = 0;
                }
            }
            _ => self.env = 0.0,
        }
        self.env * (1.0 - v.vel_sens + v.vel_sens * self.velocity)
    }

    fn update_slots(&mut self) {
        for s in 0..NUM_SLOTS {
            if self.cur_mod[s] != self.req_mod[s] {
                self.slot_fade[s] -= self.fade_step;
                if self.slot_fade[s] <= 0.0 {
                    self.slot_fade[s] = 0.0;
                    self.cur_mod[s] = self.req_mod[s];
                    for st in self.slot_state[s].iter_mut() {
                        st.reset();
                    }
                    self.p_sm[s] = self.p_tgt[s];
                }
            } else {
                self.slot_fade[s] = if self.slot_on[s] {
                    (self.slot_fade[s] + self.fade_step).min(1.0)
                } else {
                    (self.slot_fade[s] - self.fade_step).max(0.0)
                };
                let k = self.sm_k;
                self.p_sm[s].0 += (self.p_tgt[s].0 - self.p_sm[s].0) * k;
                self.p_sm[s].1 += (self.p_tgt[s].1 - self.p_sm[s].1) * k;
            }
        }
    }

    /// Procesa un frame estéreo.
    #[inline]
    pub fn process_frame(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        self.seq_sample();
        if self.ctrl_count == 0 {
            self.apply_mod();
            self.ctrl_count = CTRL_N;
        }
        self.ctrl_count -= 1;

        self.update_slots();
        let vgain = if self.p.voice.enabled { self.voice_gain() } else { 1.0 };
        let (vpre, vpost) = if self.p.voice.env_post { (1.0, vgain) } else { (vgain, 1.0) };
        let k = self.sm_k;
        for v in [
            &mut self.drive,
            &mut self.mix,
            &mut self.out,
            &mut self.cut,
            &mut self.reso,
            &mut self.noise_lvl,
            &mut self.pan,
        ] {
            v.1 += (v.0 - v.1) * k;
        }
        self.spread.1 += (self.spread.0 - self.spread.1) * k;
        if self.spread.0 == 0.0 && self.spread.1 < FB_OFF_EPS {
            self.spread.1 = 0.0;
        }
        let sp = self.spread.1;
        let fbv = self.fb_read();

        let fmode = self.p.filter_mode;
        let coefs = if fmode != FilterMode::Off {
            SvfCoefs::new(self.cut.1, self.reso.1, self.sr, self.p.scream)
        } else {
            SvfCoefs::default()
        };
        let (nl, nr) = self.noise.frame(self.p.noise_type, self.p.noise_char, &self.noise_coefs);
        let nz = [nl * self.noise_lvl.1, nr * self.noise_lvl.1];
        let input = [in_l as f64, in_r as f64];
        let mut out = [0.0f64; 2];
        let mut tap = [0.0f64; 2];
        let mut slot_out = [0.0f64; 2];
        let spread_r = sp > 0.0;

        for ch in 0..2 {
            let dry = input[ch];
            let mut wet = (dry + nz[ch]) * vpre;
            if !self.p.gate_post {
                wet *= self.gate_gain;
            }
            if fmode != FilterMode::Off && self.p.filter_pre {
                wet = self.svf[ch].process(wet, &coefs, fmode);
            }
            wet *= self.drive.1;
            if self.fb_on {
                wet += fbv[ch];
            }
            for s in 0..NUM_SLOTS {
                let (a, b) = self.p_sm[s];
                let st = &mut self.slot_state[s][ch];
                let y = if spread_r && ch == 1 {
                    modules::process_spread(self.cur_mod[s], wet, a, b, st, &self.tables, sp)
                } else {
                    modules::process(self.cur_mod[s], wet, a, b, st, &self.tables)
                };
                wet += (y - wet) * self.slot_fade[s];
            }
            slot_out[ch] = wet;
            if self.fb_on && !self.p.fb.post_filter {
                tap[ch] = wet;
            }
            if fmode != FilterMode::Off && !self.p.filter_pre {
                wet = self.svf[ch].process(wet, &coefs, fmode);
            }
            if self.fb_on && self.p.fb.post_filter {
                tap[ch] = wet;
            }
            let mut y = dry + (wet - dry) * self.mix.1;
            if self.p.gate_post {
                y *= self.gate_gain;
            }
            y *= vpost;
            y *= if ch == 0 { (1.0 - self.pan.1).min(1.0) } else { (1.0 + self.pan.1).min(1.0) };
            y *= self.out.1;
            let dc = y - self.dc_x1[ch] + self.dc_r * self.dc_y1[ch];
            self.dc_x1[ch] = y;
            self.dc_y1[ch] = dc;
            out[ch] = dc.clamp(-self.lim, self.lim);
        }
        if self.fb_on {
            self.fb_write(tap);
        }
        // Seguidor de envolvente: por muestra; la matriz lo lee cada 16.
        let lvl = match self.p.env.tap {
            EnvTap::Input => input[0].abs().max(input[1].abs()),
            EnvTap::PreFilter => slot_out[0].abs().max(slot_out[1].abs()),
            EnvTap::Voice => vgain,
        } * self.env_g;
        if lvl < FB_GUARD {
            let k = if lvl > self.env_e { self.env_ka } else { self.env_kr };
            self.env_e += (lvl - self.env_e) * k;
        } else {
            self.env_e = 0.0;
        }
        (out[0] as f32, out[1] as f32)
    }

    // ----- Realimentación ------------------------------------------------

    /// Vacía el bucle (delay, tono y DC blocker).
    fn fb_clear(&mut self) {
        for b in self.fb_buf.iter_mut() {
            b.fill(0.0);
        }
        self.fb_w = 0;
        self.fb_tone = [0.0; 2];
        self.fb_dc_x1 = [0.0; 2];
        self.fb_dc_y1 = [0.0; 2];
    }

    /// Suaviza AMOUNT y TIME, decide si el bucle está conectado y lee la
    /// señal realimentada de cada canal: delay -> tono -> DC -> ganancia -> tanh.
    #[inline]
    fn fb_read(&mut self) -> [f64; 2] {
        let k = self.sm_k;
        self.fb_amt.1 += (self.fb_amt.0 - self.fb_amt.1) * k;
        self.fb_time.1 += (self.fb_time.0 - self.fb_time.1) * k;
        if self.fb_amt.0 == 0.0 && self.fb_amt.1 < FB_OFF_EPS {
            self.fb_amt.1 = 0.0;
        }
        let on = self.fb_amt.0 > 0.0 || self.fb_amt.1 > 0.0;
        if on && !self.fb_on {
            // Se conecta: el bucle arranca vacío.
            self.fb_clear();
        }
        self.fb_on = on;
        if !on {
            return [0.0; 2];
        }
        let max_d = (FB_LEN - 2) as f64;
        let mut y = [0.0; 2];
        for ch in 0..2 {
            let stretch = if ch == 1 { 1.0 + 0.3 * self.spread.1 } else { 1.0 };
            let d = (self.fb_time.1 * stretch * 0.001 * self.sr).clamp(1.0, max_d);
            let i = d.floor();
            let f = d - i;
            let i = i as usize;
            let b = &self.fb_buf[ch];
            let x0 = b[self.fb_w.wrapping_sub(i) & FB_MASK];
            let x1 = b[self.fb_w.wrapping_sub(i + 1) & FB_MASK];
            let r = x0 + (x1 - x0) * f;
            self.fb_tone[ch] += (r - self.fb_tone[ch]) * self.fb_tone_a;
            let t = self.fb_tone[ch];
            let dc = t - self.fb_dc_x1[ch] + self.dc_r * self.fb_dc_y1[ch];
            self.fb_dc_x1[ch] = t;
            self.fb_dc_y1[ch] = dc;
            y[ch] = (self.fb_amt.1 * dc).clamp(-20.0, 20.0).tanh();
        }
        y
    }

    /// Escribe la toma en el delay. Un valor no finito (o desbocado) reinicia
    /// el bucle.
    #[inline]
    fn fb_write(&mut self, tap: [f64; 2]) {
        if !(tap[0].abs() < FB_GUARD && tap[1].abs() < FB_GUARD) {
            self.fb_clear();
            return;
        }
        for ch in 0..2 {
            self.fb_buf[ch][self.fb_w] = tap[ch];
        }
        self.fb_w = (self.fb_w + 1) & FB_MASK;
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut s = Snapshot {
            seq_running: self.seq_running,
            synced: self.was_host,
            gate_env: self.gate_env as f32,
            env_follow: self.env_e.min(1.0) as f32,
            voice_env: self.env as f32,
            note: self.note.map(|n| n as i16).unwrap_or(-1),
            ..Default::default()
        };
        for l in 0..NUM_LANES {
            s.lane_idx[l] = self.lanes[l].idx as u8;
            s.lane_fired[l] = self.lanes[l].fired;
            s.lane_val[l] = self.lanes[l].val as f32;
        }
        for i in 0..NUM_SLOTS {
            s.slot_active[i] = self.cur_mod[i] != Module::Off && self.slot_fade[i] > 0.5;
            s.slot_seq_on[i] = self.slot_on[i];
        }
        for (d, v) in self.modsum.iter().enumerate() {
            s.modsum[d] = *v as f32;
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(p: &Params, secs: f64, sr: f64) -> (Vec<f32>, Vec<f32>) {
        let mut e = Engine::new(sr);
        let block = 512;
        let n = (secs * sr) as usize;
        let mut l = vec![0.0f32; n];
        let mut r = vec![0.0f32; n];
        let mut pos = 0usize;
        while pos < n {
            let len = block.min(n - pos);
            let t = Transport {
                playing: true,
                pos_qn: Some(pos as f64 / sr * 2.0), // 120 bpm
                tempo: 120.0,
                ts_num: 4.0,
                ts_den: 4.0,
            };
            e.begin_block(p, &t, len);
            for i in pos..pos + len {
                let (a, b) = e.process_frame(0.0, 0.0);
                l[i] = a;
                r[i] = b;
            }
            pos += len;
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    }

    /// Misma configuración que la prueba del JSFX (test.rpp): pink -> foldback
    /// -> LP, gate E(7,16) al 100 %. El gate debe sonar en los pasos 0,3,5,7,
    /// 10,12,14 y quedar en silencio en el resto.
    #[test]
    fn gate_follows_default_euclid() {
        let mut p = Params::default();
        p.slot_module = [Module::Foldback, Module::Off, Module::Off, Module::Off];
        p.slot_a[0] = 0.4;
        p.drive_db = 6.0;
        p.out_db = -6.0;
        p.noise_type = NoiseType::White;
        p.noise_level_db = -12.0;
        p.filter_mode = FilterMode::Lowpass;
        p.reso = 0.3;
        p.gate_depth = 1.0;
        let sr = 48000.0;
        let (l, _) = render(&p, 2.0, sr);
        let st = (sr * 0.125) as usize; // 1/16 a 120 bpm
        let hits = [0, 3, 5, 7, 10, 12, 14];
        for k in 0..16 {
            let first_half = rms(&l[k * st + st / 10..k * st + st / 2]);
            if hits.contains(&k) {
                assert!(first_half > 0.05, "paso {k} debería sonar: {first_half}");
            } else if !hits.contains(&(k.wrapping_sub(1))) {
                assert!(first_half < 0.002, "paso {k} debería estar en silencio: {first_half}");
            }
        }
    }

    /// Como `render`, con una entrada de audio dada por `input(n)`.
    fn render_in(p: &Params, secs: f64, sr: f64, input: impl Fn(usize) -> f32) -> (Vec<f32>, Vec<f32>) {
        let mut e = Engine::new(sr);
        let n = (secs * sr) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        let mut pos = 0usize;
        while pos < n {
            let len = 512.min(n - pos);
            let t = Transport { playing: true, pos_qn: Some(pos as f64 / sr * 2.0), ..Transport::default() };
            e.begin_block(p, &t, len);
            for i in pos..pos + len {
                let x = input(i);
                let (a, b) = e.process_frame(x, -x);
                l[i] = a;
                r[i] = b;
            }
            pos += len;
        }
        (l, r)
    }

    /// Lleva al máximo las funciones de la 0.8 (variante `v`).
    fn max_new_features(p: &mut Params, v: usize) {
        p.fb.amount = 1.5;
        p.fb.time_ms = if v % 2 == 0 { 0.05 } else { 50.0 };
        p.fb.tone_hz = 20000.0;
        p.fb.post_filter = v % 3 == 0;
        p.lanes[1].dest = dest::FB_AMOUNT;
        p.lanes[1].depth = 1.0;
        p.lanes[2].dest = dest::FB_TIME;
        p.lanes[2].depth = if v % 2 == 0 { 1.0 } else { -1.0 };
        p.lanes[2].glide = 0.0;
        p.scream = true;
        p.reso = 1.0;
        p.env.tap = if v % 2 == 0 { EnvTap::Input } else { EnvTap::PreFilter };
        p.env.gain_db = 24.0;
        p.env.attack_ms = 0.1;
        p.env.release_ms = 1.0;
        p.lanes[1].src = LaneSource::Env;
        p.lanes[2].src = LaneSource::SeqEnv;
        p.lanes[3].src = LaneSource::Env;
        p.lanes[3].dest = if v % 2 == 0 { 12 } else { dest::SPREAD };
        p.lanes[3].depth = 1.0;
        p.spread = 1.0;
        p.filter_mode = FilterMode::from_index(1 + v % 3);
        p.filter_pre = v % 2 == 1;
    }

    fn assert_limited(l: &[f32], r: &[f32], what: &str) {
        for v in l.iter().chain(r.iter()) {
            assert!(v.is_finite(), "{what}: {v}");
            assert!(v.abs() <= 0.9661, "{what}: {v}");
        }
    }

    #[test]
    fn output_is_finite_and_limited_for_every_module() {
        for (i, m) in Module::ALL.iter().enumerate() {
            let mut p = Params::default();
            p.slot_module = [*m, Module::Foldback, Module::RingMod, Module::Slew];
            p.drive_db = 36.0;
            p.noise_type = NoiseType::from_index(1 + i % 5);
            p.noise_level_db = 0.0;
            p.filter_mode = FilterMode::from_index(1 + i % 3);
            p.reso = 1.0;
            p.lanes[1].dest = 11;
            p.lanes[1].depth = 1.0;
            p.lanes[2].dest = 15;
            p.lanes[2].depth = 1.0;
            p.lanes[3].dest = 14;
            p.lanes[3].depth = 1.0;
            let (l, r) = render(&p, 0.5, 44100.0);
            assert_limited(&l, &r, &format!("{m:?}"));

            // Funciones nuevas al máximo, con entrada a fondo de escala y en
            // varias frecuencias de muestreo (incluida la máxima del delay).
            max_new_features(&mut p, i);
            p.drive_db = 48.0;
            p.slot_module[1] = *m;
            for sr in [44100.0, 96000.0, MAX_SR] {
                let (l, r) = render_in(&p, 0.25, sr, |n| if (n / 200) % 2 == 0 { 1.0 } else { -1.0 });
                assert_limited(&l, &r, &format!("{m:?} al máximo, {sr} Hz"));
            }
        }
    }

    /// FB AMOUNT = 0: el bucle no se conecta nunca.
    #[test]
    fn feedback_off_never_connects() {
        let p = stateful_params(NoiseType::White);
        let mut e = Engine::new(48000.0);
        play(&mut e, &p, 0.0, 9600, 512, 48000.0);
        assert!(!e.fb_on);
        assert!(e.fb_buf[0].iter().chain(e.fb_buf[1].iter()).all(|v| *v == 0.0));
    }

    /// Un impulso vuelve a la entrada del slot 1 después de FB TIME.
    #[test]
    fn feedback_echo_arrives_after_fb_time() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.fb.amount = 0.8;
        p.fb.time_ms = 10.0;
        p.fb.tone_hz = 20000.0;
        let (l, _) = render_in(&p, 0.1, sr, |n| if n == 1000 { 0.5 } else { 0.0 });
        let echo = 1000 + 480;
        let peak = |a: usize, b: usize| l[a..b].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak(1010, echo - 2) < 0.02, "antes del eco: {}", peak(1010, echo - 2));
        assert!(peak(echo - 1, echo + 3) > 0.2, "eco: {}", peak(echo - 1, echo + 3));
        assert!(peak(echo + 480 - 1, echo + 480 + 3) > 0.1, "segunda repetición");
    }

    /// SCREAM con r = 1: tras un impulso, el filtro sostiene una oscilación
    /// cercana a fc con la entrada en silencio, y la salida queda acotada.
    #[test]
    fn scream_self_oscillates_near_cutoff() {
        let sr = 48000.0;
        for (fc, mode) in [(1000.0, FilterMode::Lowpass), (440.0, FilterMode::Bandpass), (3000.0, FilterMode::Highpass)] {
            let mut p = Params::default();
            p.slot_module = [Module::Off; 4];
            p.filter_mode = mode;
            p.cutoff_hz = fc;
            p.reso = 1.0;
            p.scream = true;
            let (l, r) = render_in(&p, 2.0, sr, |n| if n == 100 { 0.5 } else { 0.0 });
            assert_limited(&l, &r, "scream");
            let seg: Vec<f64> = l[48000..].iter().map(|v| *v as f64).collect();
            let lv = |x: &[f64]| (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
            let (a, b) = (lv(&seg[..24000]), lv(&seg[24000..]));
            if std::env::var("NG_DEBUG").is_ok() {
                println!("{fc} {mode:?}: rms 1.0-1.5 s = {a:.4}, 1.5-2.0 s = {b:.4}");
            }
            // Sostenida: nivel apreciable y sin decaer entre la primera y la segunda mitad.
            assert!(b > 0.02 && (b / a - 1.0).abs() < 0.05, "{fc} Hz: no se sostiene ({a} -> {b})");
            let lo = (sr / fc * 0.5) as usize;
            let hi = (sr / fc * 2.0) as usize;
            let c = |lag: usize| seg.iter().zip(&seg[lag..]).map(|(u, v)| u * v).sum::<f64>();
            let best = (lo..hi).max_by(|&a, &b| c(a).partial_cmp(&c(b)).unwrap()).unwrap();
            let f = sr / best as f64;
            assert!((f / fc - 1.0).abs() < 0.05, "{fc} Hz ({mode:?}): oscila a {f} Hz");
        }
        // Sin SCREAM, el mismo filtro con r = 1 se apaga.
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.filter_mode = FilterMode::Lowpass;
        p.cutoff_hz = 1000.0;
        p.reso = 1.0;
        let (l, _) = render_in(&p, 2.0, sr, |n| if n == 100 { 0.5 } else { 0.0 });
        assert!(l[48000..].iter().all(|v| v.abs() < 1e-4));
    }

    /// ENV sigue a la entrada (ataque y release) y modula con las tres fuentes.
    #[test]
    fn env_follower_modulates_lanes() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.env.attack_ms = 2.0;
        p.env.release_ms = 50.0;
        p.lanes[1].dest = dest::MIX;
        p.lanes[1].depth = 1.0;
        p.lanes[1].src = LaneSource::Env;
        p.lanes[2].dest = dest::RESO;
        p.lanes[2].depth = 1.0;
        p.lanes[2].src = LaneSource::SeqEnv;
        p.lanes[3].dest = dest::DRIVE;
        p.lanes[3].depth = 0.5;
        let mut e = Engine::new(sr);
        let t = Transport::default();
        let run = |e: &mut Engine, n: usize, x: f32| {
            e.begin_block(&p, &t, n);
            for _ in 0..n {
                e.process_frame(x, -x * 0.5);
            }
            e.snapshot()
        };
        let s = run(&mut e, 4800, 0.0);
        assert_eq!(s.env_follow, 0.0);
        assert_eq!(s.modsum[10], 0.0);
        // Ataque de 2 ms: tras 100 ms de señal a 0.6, ENV ≈ 0.6 (máx. de |L|, |R|).
        let s = run(&mut e, 4800, 0.6);
        assert!((s.env_follow - 0.6).abs() < 1e-3, "{}", s.env_follow);
        assert!((s.modsum[10] - 0.6).abs() < 2e-3, "ENV -> MIX: {}", s.modsum[10]);
        assert!((s.modsum[12] - s.lane_val[2] * s.env_follow).abs() < 2e-3, "SEQ×ENV");
        // Release de 50 ms: tras 50 ms baja a ~0.6/e.
        let s = run(&mut e, 2400, 0.0);
        assert!((s.env_follow - 0.6 * (-1.0f32).exp()).abs() < 0.01, "{}", s.env_follow);
        // ENV GAIN +12 dB satura la fuente en 1.
        let mut p2 = p;
        p2.env.gain_db = 12.0;
        e.begin_block(&p2, &t, 4800);
        for _ in 0..4800 {
            e.process_frame(0.6, 0.0);
        }
        assert_eq!(e.snapshot().env_follow, 1.0);
    }

    /// VOICE (instrumento): ENV sigue la envolvente de la voz.
    #[test]
    fn env_voice_tap_follows_adsr() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.noise_type = NoiseType::Pink;
        p.voice.enabled = true;
        p.voice.sustain = 0.5;
        p.voice.vel_sens = 0.0;
        p.env.tap = EnvTap::Voice;
        p.env.attack_ms = 0.1;
        p.env.release_ms = 1.0;
        let mut e = Engine::new(sr);
        let t = Transport::default();
        let run = |e: &mut Engine, n: usize| {
            e.begin_block(&p, &t, n);
            for _ in 0..n {
                e.process_frame(0.0, 0.0);
            }
            e.snapshot()
        };
        assert_eq!(run(&mut e, 480).env_follow, 0.0);
        e.note_on(60, 1.0);
        let s = run(&mut e, 48000);
        assert!((s.env_follow - 0.5).abs() < 0.01, "sostenido: {}", s.env_follow);
        assert!((s.env_follow - s.voice_env).abs() < 0.01);
        e.note_off(60);
        assert!(run(&mut e, 96000).env_follow < 1e-3);
    }

    /// PRE-FILTER: sin entrada, ENV sigue la salida de los slots (ruido).
    #[test]
    fn env_prefilter_tap_follows_chain() {
        let mut p = Params::default();
        p.noise_type = NoiseType::White;
        p.noise_level_db = -6.0;
        p.env.tap = EnvTap::PreFilter;
        let mut e = Engine::new(48000.0);
        e.begin_block(&p, &Transport::default(), 4800);
        for _ in 0..4800 {
            e.process_frame(0.0, 0.0);
        }
        assert!(e.snapshot().env_follow > 0.3);
        p.env.tap = EnvTap::Input;
        e.begin_block(&p, &Transport::default(), 48000);
        for _ in 0..48000 {
            e.process_frame(0.0, 0.0);
        }
        assert!(e.snapshot().env_follow < 2e-3); // 1·e^(−1 s / 150 ms) ≈ 1.3e-3
    }

    /// Render estéreo con la misma entrada en L y R.
    fn render_mono_in(p: &Params, n: usize, sr: f64) -> (Vec<f64>, Vec<f64>) {
        let mut e = Engine::new(sr);
        let (mut l, mut r) = (vec![], vec![]);
        let mut pos = 0usize;
        while pos < n {
            let len = 512.min(n - pos);
            let t = Transport { playing: true, pos_qn: Some(pos as f64 / sr * 2.0), ..Transport::default() };
            e.begin_block(p, &t, len);
            for i in pos..pos + len {
                let x = (0.3 * (i as f64 * 0.013).sin() + 0.2 * (i as f64 * 0.0021).sin()) as f32;
                let (a, b) = e.process_frame(x, x);
                l.push(a as f64);
                r.push(b as f64);
            }
            pos += len;
        }
        (l, r)
    }

    fn correlation(a: &[f64], b: &[f64]) -> f64 {
        let n = a.len() as f64;
        let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
        let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
        let va: f64 = a.iter().map(|x| (x - ma).powi(2)).sum();
        let vb: f64 = b.iter().map(|y| (y - mb).powi(2)).sum();
        cov / (va * vb).sqrt()
    }

    /// SPREAD = 0 deja L y R idénticos con entrada mono (y la salida es la de
    /// la 0.7: ver `matches_v0_7_reference`); SPREAD = 1 los decorrelaciona.
    #[test]
    fn spread_decorrelates_stereo() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::RingMod, Module::Downsample, Module::Off, Module::Off];
        p.slot_a = [0.35, 0.3, 0.5, 0.5];
        p.slot_b = [0.2, 0.5, 0.5, 0.5];
        p.noise_type = NoiseType::White;
        p.noise_char = 0.0; // ruido mono
        p.noise_level_db = -24.0;
        p.fb.amount = 0.7;
        p.fb.time_ms = 3.0;
        let (l, r) = render_mono_in(&p, 48000, sr);
        assert!(bit_identical(
            &l.iter().map(|v| *v as f32).collect::<Vec<_>>(),
            &r.iter().map(|v| *v as f32).collect::<Vec<_>>()
        ));
        let c0 = correlation(&l, &r);
        p.spread = 1.0;
        let (l, r) = render_mono_in(&p, 48000, sr);
        let c1 = correlation(&l, &r);
        if std::env::var("NG_DEBUG").is_ok() {
            println!("correlación L/R: SPREAD 0 = {c0:.4}, SPREAD 1 = {c1:.4}");
        }
        assert!(c0 > 0.9999 && c1 < c0 - 0.1, "SPREAD 0: {c0}, SPREAD 1: {c1}");
    }

    /// Un valor no finito en la toma reinicia el bucle en vez de guardarse.
    #[test]
    fn feedback_recovers_from_nan() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.fb.amount = 1.2;
        let mut e = Engine::new(sr);
        let t = Transport::default();
        e.begin_block(&p, &t, 4800);
        for n in 0..4800 {
            let x = if n < 2000 { 0.3 } else { 0.0 };
            e.process_frame(x, x);
        }
        assert!(e.fb_on && e.fb_buf[0].iter().any(|v| *v != 0.0));
        e.fb_write([f64::NAN, 0.0]);
        assert!(e.fb_buf[0].iter().chain(e.fb_buf[1].iter()).all(|v| *v == 0.0));
        e.fb_write([f64::INFINITY, 0.0]);
        e.fb_write([0.0, -1e31]);
        assert!(e.fb_buf[1].iter().all(|v| *v == 0.0));
        e.begin_block(&p, &t, 4800);
        assert!((0..4800).all(|_| e.process_frame(0.0, 0.0).0.is_finite()));
    }

    /// Instrumento: sin notas no suena; con nota suena; al soltar se apaga.
    #[test]
    fn voice_follows_notes() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.noise_type = NoiseType::Pink;
        p.noise_level_db = -6.0;
        p.voice.enabled = true;
        p.voice.release_ms = 50.0;
        let mut e = Engine::new(sr);
        let t = Transport::default();
        let run = |e: &mut Engine, n: usize| {
            e.begin_block(&p, &t, n);
            let out: Vec<f32> = (0..n).map(|_| e.process_frame(0.0, 0.0).0).collect();
            rms(&out)
        };
        assert!(run(&mut e, 4800) < 1e-6, "sin notas debe haber silencio");
        e.note_on(60, 1.0);
        assert!(run(&mut e, 4800) > 0.02, "con nota debe sonar");
        e.note_off(60);
        run(&mut e, 48000);
        assert!(run(&mut e, 4800) < 1e-6, "tras el release debe volver el silencio");
    }

    /// PITCH: el LFSR corto suena afinado a la nota (A4 = 440 Hz).
    #[test]
    fn pitched_lfsr_follows_note() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.noise_type = NoiseType::Lfsr;
        p.noise_level_db = -6.0;
        p.voice.enabled = true;
        p.voice.pitched = true;
        p.voice.sustain = 1.0;
        let mut e = Engine::new(sr);
        e.begin_block(&p, &Transport::default(), 1);
        e.note_on(69, 1.0);
        let n = 48000;
        e.begin_block(&p, &Transport::default(), n);
        let x: Vec<f64> = (0..n).map(|_| e.process_frame(0.0, 0.0).0 as f64).collect();
        // Autocorrelación: el período más fuerte entre 2 y 10 ms debe ser 1/440 s.
        let seg = &x[24000..];
        let best = (96..480)
            .max_by(|&a, &b| {
                let c = |lag: usize| seg.iter().zip(&seg[lag..]).map(|(u, v)| u * v).sum::<f64>();
                c(a).partial_cmp(&c(b)).unwrap()
            })
            .unwrap();
        let f = sr / best as f64;
        assert!((f - 440.0).abs() < 8.0, "frecuencia {f}");
    }

    /// Reproduce `n` muestras desde la negra `qn0`, a 120 bpm, en bloques de
    /// `block`. Devuelve L y R intercalados.
    fn play(e: &mut Engine, p: &Params, qn0: f64, n: usize, block: usize, sr: f64) -> Vec<f32> {
        let mut out = Vec::with_capacity(2 * n);
        let mut pos = 0usize;
        while pos < n {
            let len = block.min(n - pos);
            let t = Transport {
                playing: true,
                pos_qn: Some(qn0 + pos as f64 / sr * 2.0),
                tempo: 120.0,
                ts_num: 4.0,
                ts_den: 4.0,
            };
            e.begin_block(p, &t, len);
            for _ in 0..len {
                let (a, b) = e.process_frame(0.0, 0.0);
                out.push(a);
                out.push(b);
            }
            pos += len;
        }
        out
    }

    /// Transporte detenido durante `n` muestras (preroll antes de play).
    fn idle(e: &mut Engine, p: &Params, n: usize, block: usize) {
        let t = Transport { playing: false, pos_qn: Some(0.0), ..Transport::default() };
        let mut pos = 0usize;
        while pos < n {
            let len = block.min(n - pos);
            e.begin_block(p, &t, len);
            for _ in 0..len {
                e.process_frame(0.0, 0.0);
            }
            pos += len;
        }
    }

    fn bit_identical(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
    }

    /// Configuración con todo el estado que depende del historial: ruido,
    /// S/H, ring mod, downsample, slew, filtro, glide, DRUNK, RANDOM,
    /// probabilidad y reloj AUTO (corre libre mientras el transporte está
    /// detenido).
    fn stateful_params(noise: NoiseType) -> Params {
        let mut p = Params::default();
        p.slot_module = [Module::TriggerSh, Module::RingMod, Module::Downsample, Module::Slew];
        p.slot_a = [0.3, 0.4, 0.2, 0.6];
        p.slot_b = [0.4, 0.5, 0.5, 0.3];
        p.drive_db = 6.0;
        p.noise_type = noise;
        p.noise_level_db = -6.0;
        p.filter_mode = FilterMode::Lowpass;
        p.filter_pre = true;
        p.reso = 0.4;
        p.gate_depth = 0.8;
        p.lanes[0].prob = 80.0;
        p.lanes[1].dest = dest::CUTOFF;
        p.lanes[1].glide = 0.7;
        p.lanes[2].dest = 1;
        p.lanes[2].dir = Direction::Random;
        p.lanes[3].dest = dest::PAN;
        p.lanes[3].dir = Direction::Drunk;
        p
    }

    /// 13.4: dos renders con distinto preroll (transporte detenido) antes de
    /// play producen la misma salida, bit a bit.
    #[test]
    fn preroll_does_not_change_render() {
        let sr = 48000.0;
        for t in 1..=5 {
            let p = stateful_params(NoiseType::from_index(t));
            let render = |pre: usize, pre_block: usize| {
                let mut e = Engine::new(sr);
                idle(&mut e, &p, pre, pre_block);
                let out = play(&mut e, &p, 8.0, 72000, 512, sr);
                assert_eq!(e.resyncs, 1);
                out
            };
            let a = render(0, 512);
            assert!(rms(&a) > 0.01, "tipo {t}: la prueba necesita señal");
            assert!(bit_identical(&a, &render(12_345, 300)), "tipo {t}: preroll 12345");
            assert!(bit_identical(&a, &render(777, 64)), "tipo {t}: preroll 777");
        }
    }

    /// 13.4: reposicionar y volver a reproducir da la misma salida que la
    /// primera pasada, con stop intermedio o con salto durante la reproducción.
    #[test]
    fn reposition_replays_identically() {
        let sr = 48000.0;
        let p = stateful_params(NoiseType::Pink);
        let mut e = Engine::new(sr);
        let first = play(&mut e, &p, 4.0, 48000, 512, sr);
        // Stop, más audio detenido, y play desde el mismo punto.
        idle(&mut e, &p, 5000, 512);
        assert!(bit_identical(&first, &play(&mut e, &p, 4.0, 48000, 512, sr)), "tras stop");
        assert_eq!(e.resyncs, 2);
        // Salto hacia atrás sin detener (bucle): el bloque anterior seguía en marcha.
        play(&mut e, &p, 6.0, 20000, 512, sr);
        assert_eq!(e.resyncs, 2, "seguir reproduciendo no es un salto");
        assert!(bit_identical(&first, &play(&mut e, &p, 4.0, 48000, 512, sr)), "salto atrás");
        assert_eq!(e.resyncs, 3);
        // Salto hacia adelante, también sin detener.
        play(&mut e, &p, 20.0, 1024, 512, sr);
        assert_eq!(e.resyncs, 4);
    }

    /// La reproducción continua no se confunde con un salto aunque cambien el
    /// tamaño de bloque (incluidos bloques vacíos) y el tempo.
    #[test]
    fn continuous_playback_never_resyncs() {
        let sr = 48000.0;
        let p = stateful_params(NoiseType::White);
        let mut e = Engine::new(sr);
        let sizes = [512usize, 37, 1024, 0, 64, 333, 0, 0, 2048, 1];
        let mut qn = 3.0;
        let mut tempo = 120.0;
        for i in 0..2000 {
            let len = sizes[i % sizes.len()];
            let t = Transport { playing: true, pos_qn: Some(qn), tempo, ts_num: 4.0, ts_den: 4.0 };
            e.begin_block(&p, &t, len);
            for _ in 0..len {
                e.process_frame(0.0, 0.0);
            }
            qn += len as f64 * tempo / 60.0 / sr;
            tempo = 120.0 + 60.0 * (i as f64 / 2000.0); // rampa 120 -> 180 bpm
        }
        assert_eq!(e.resyncs, 1);
    }

    /// GEN con RETRIG: cada nota reinicia la fuente de ruido, así que dos notas
    /// iguales que parten del silencio suenan igual. Sin RETRIG, no.
    #[test]
    fn retrig_restarts_noise() {
        let sr = 48000.0;
        let mut p = Params::default();
        p.slot_module = [Module::Off; 4];
        p.noise_type = NoiseType::Pink;
        p.noise_level_db = -6.0;
        p.voice.enabled = true;
        p.voice.release_ms = 5.0;
        let note = |e: &mut Engine, p: &Params| {
            e.note_on(60, 1.0);
            e.begin_block(p, &Transport::default(), 4800);
            let x: Vec<f32> = (0..4800).map(|_| e.process_frame(0.0, 0.0).0).collect();
            e.note_off(60);
            e.begin_block(p, &Transport::default(), 96000);
            for _ in 0..96000 {
                e.process_frame(0.0, 0.0);
            }
            x
        };
        let max_diff = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        for retrig in [true, false] {
            p.voice.retrig = retrig;
            let mut e = Engine::new(sr);
            let a = note(&mut e, &p);
            let b = note(&mut e, &p);
            assert!(rms(&a) > 0.02);
            let d = max_diff(&a, &b);
            if retrig {
                assert!(d < 1e-6, "con RETRIG: {d}");
            } else {
                assert!(d > 0.01, "sin RETRIG: {d}");
            }
        }
    }

    /// FNV-1a de 64 bits sobre los bits f32 de un render.
    fn fnv(x: &[f32]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for v in x {
            for b in v.to_bits().to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }

    /// Configuraciones de referencia: solo usan parámetros que existían en la
    /// 0.7 (el resto queda en su valor por defecto).
    fn reference_renders() -> Vec<Vec<f32>> {
        let sr = 48000.0;
        let mut out = vec![];
        // Todos los tipos de ruido con todo el estado que depende del historial.
        for t in 1..=5 {
            let p = stateful_params(NoiseType::from_index(t));
            out.push(play(&mut Engine::new(sr), &p, 2.0, 24000, 512, sr));
        }
        // Cada módulo en el slot 1, con filtro y modulación de todos los tipos de destino.
        for (i, m) in Module::ALL.iter().enumerate() {
            let mut p = Params::default();
            p.slot_module = [*m, Module::TableShaper, Module::Bitcrush, Module::Rectifier];
            p.slot_a = [0.7, 0.3, 0.8, 0.6];
            p.drive_db = 12.0;
            p.noise_type = NoiseType::from_index(1 + i % 5);
            p.noise_level_db = -3.0;
            p.filter_mode = FilterMode::from_index(1 + i % 3);
            p.reso = 0.9;
            p.gate_post = i % 2 == 0;
            p.gate_depth = 0.5;
            p.lanes[1].dest = (1 + i % 13) as u8;
            p.lanes[2].dest = dest::PAN;
            p.lanes[3].dest = 15 + (i % 4) as u8;
            p.lanes[3].depth = if i % 2 == 0 { 1.0 } else { -1.0 };
            p.swing = 62.0;
            p.seq_seed = 77 + i as u32;
            out.push(play(&mut Engine::new(sr), &p, 0.0, 12000, 333, sr));
        }
        // Instrumento: notas, PITCH, TRACK y RETRIG, con el reloj libre.
        let mut p = Params::default();
        p.noise_type = NoiseType::Lfsr;
        p.noise_level_db = -6.0;
        p.slot_module = [Module::RingMod, Module::SoftClip, Module::Off, Module::Off];
        p.filter_mode = FilterMode::Lowpass;
        p.voice.enabled = true;
        p.voice.pitched = true;
        p.voice.key_track = 1.0;
        p.voice.env_post = false;
        let mut e = Engine::new(sr);
        let mut x = vec![];
        for (k, n) in [57u8, 64, 69, 52].iter().enumerate() {
            e.note_on(*n, 0.5 + 0.1 * k as f32);
            e.begin_block(&p, &Transport::default(), 6000);
            x.extend((0..6000).map(|_| e.process_frame(0.0, 0.0).0));
            e.note_off(*n);
            e.begin_block(&p, &Transport::default(), 3000);
            x.extend((0..3000).map(|_| e.process_frame(0.0, 0.0).1));
        }
        out.push(x);
        out
    }

    /// Compatibilidad: con los parámetros nuevos en su valor por defecto, la
    /// salida es idéntica bit a bit a la de la versión 0.7. Hashes generados con
    /// el núcleo 0.7 antes de agregar las funciones de la 0.8.
    #[test]
    fn matches_v0_7_reference() {
        const REF_0_7: [u64; 17] = [
            0x2059f21d07fb45cf,
            0xc4840d011e33faac,
            0x000a71607c884946,
            0x8b132d343f4e71ed,
            0xb7bbaa661fac7324,
            0xd32a81c90948387b,
            0x06d757fc79c6e80a,
            0xeafe21c516862a72,
            0xd78450b744f3887e,
            0xb99ef52bd6540914,
            0x822b0ebc797f34d0,
            0xa45ef55c24438d83,
            0xa476a384859dd857,
            0x5b9205fe8d41f3d1,
            0xcce5750ac52cc3ae,
            0x48d144381abdbb98,
            0xb2063f7f00abc108,
        ];
        let got: Vec<u64> = reference_renders().iter().map(|x| fnv(x)).collect();
        if std::env::var("NG_PRINT_REF").is_ok() {
            println!("REF {got:#018x?}");
        }
        assert_eq!(got.as_slice(), &REF_0_7[..]);
    }

    /// Reloj libre: no hay resincronización (comportamiento declarado).
    #[test]
    fn free_clock_never_resyncs() {
        let sr = 48000.0;
        let mut p = stateful_params(NoiseType::White);
        p.clock = Clock::Free;
        let mut e = Engine::new(sr);
        play(&mut e, &p, 0.0, 4800, 512, sr);
        idle(&mut e, &p, 4800, 512);
        play(&mut e, &p, 0.0, 4800, 512, sr);
        assert_eq!(e.resyncs, 0);
    }

    #[test]
    fn silence_in_silence_out_without_noise() {
        let (l, r) = render(&Params::default(), 0.3, 48000.0);
        assert!(rms(&l) < 1e-9 && rms(&r) < 1e-9);
    }
}
