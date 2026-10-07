//! Los 10 módulos de distorsión. Firma común: (x, a, b, estado) -> y, con
//! a y b ya mapeados (ver [`map_params`]) y suavizados.

use crate::rng::XorShift;

pub const TABLE_SIZE: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Module {
    #[default]
    Off = 0,
    HardClip = 1,
    SoftClip = 2,
    Foldback = 3,
    Bitcrush = 4,
    Downsample = 5,
    Rectifier = 6,
    TableShaper = 7,
    TriggerSh = 8,
    RingMod = 9,
    Slew = 10,
}

impl Module {
    pub const ALL: [Module; 11] = [
        Module::Off,
        Module::HardClip,
        Module::SoftClip,
        Module::Foldback,
        Module::Bitcrush,
        Module::Downsample,
        Module::Rectifier,
        Module::TableShaper,
        Module::TriggerSh,
        Module::RingMod,
        Module::Slew,
    ];

    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).copied().unwrap_or(Module::Off)
    }

    /// ¿Usa el parámetro A / B? (para atenuarlo en la UI)
    pub fn uses(self, b: bool) -> bool {
        match self {
            Module::Off => false,
            _ if !b => true,
            Module::Foldback
            | Module::Rectifier
            | Module::TableShaper
            | Module::TriggerSh
            | Module::RingMod
            | Module::Slew => true,
            _ => false,
        }
    }
}

/// Estado por slot y canal.
#[derive(Clone, Copy, Debug)]
pub struct SlotState {
    ds_count: f64,
    ds_hold: f64,
    sh_hold: f64,
    sh_prev: f64,
    sh_rng: XorShift,
    ring_phase: f64,
    slew_y: f64,
}

impl SlotState {
    pub fn new(seed: u32) -> Self {
        Self {
            ds_count: 0.0,
            ds_hold: 0.0,
            sh_hold: 0.0,
            sh_prev: 0.0,
            sh_rng: XorShift::new(seed),
            ring_phase: 0.0,
            slew_y: 0.0,
        }
    }
    pub fn reset(&mut self) {
        let rng = self.sh_rng;
        *self = Self::new(1);
        self.sh_rng = rng;
    }
}

/// Tablas del waveshaper (cruda y suavizada), generadas desde un seed.
#[derive(Clone)]
pub struct Tables {
    pub raw: Vec<f64>,
    pub smooth: Vec<f64>,
    pub seed: Option<u32>,
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            raw: vec![0.0; TABLE_SIZE],
            smooth: vec![0.0; TABLE_SIZE],
            seed: None,
        }
    }
}

fn normalize_table(t: &mut [f64]) {
    let center = (t[TABLE_SIZE / 2 - 1] + t[TABLE_SIZE / 2]) * 0.5;
    let mut peak: f64 = 0.0;
    for v in t.iter_mut() {
        *v -= center;
        peak = peak.max(v.abs());
    }
    if peak > 0.0 {
        for v in t.iter_mut() {
            *v /= peak;
        }
    }
}

impl Tables {
    /// Regenera solo si el seed cambió. Media móvil O(N) con suma corrida
    /// (el JSFX la calcula O(N·97); el resultado es el mismo).
    pub fn ensure(&mut self, seed: u32) {
        if self.seed == Some(seed) {
            return;
        }
        let mut s = seed.wrapping_add(1).max(1);
        for _ in 0..16 {
            s = crate::rng::xorshift32(s);
        }
        for v in self.raw.iter_mut() {
            s = crate::rng::xorshift32(s);
            *v = s as f64 / 4_294_967_296.0 * 2.0 - 1.0;
        }
        // En la pila: sin asignar memoria en el hilo de audio.
        let mut prefix = [0.0f64; TABLE_SIZE + 1];
        for i in 0..TABLE_SIZE {
            prefix[i + 1] = prefix[i] + self.raw[i];
        }
        for i in 0..TABLE_SIZE {
            let j0 = i.saturating_sub(48);
            let j1 = (i + 48).min(TABLE_SIZE - 1);
            self.smooth[i] = (prefix[j1 + 1] - prefix[j0]) / (j1 - j0 + 1) as f64;
        }
        normalize_table(&mut self.raw);
        normalize_table(&mut self.smooth);
        self.seed = Some(seed);
    }

    #[inline]
    fn lookup(t: &[f64], x: f64) -> f64 {
        let pos = (x.clamp(-1.0, 1.0) + 1.0) * 0.5 * (TABLE_SIZE - 1) as f64;
        let i = (pos.floor() as usize).min(TABLE_SIZE - 2);
        let f = pos - i as f64;
        t[i] + (t[i + 1] - t[i]) * f
    }
}

#[inline]
fn tanh(x: f64) -> f64 {
    x.clamp(-20.0, 20.0).tanh()
}

/// Mapea A/B crudos (0..1) al rango útil del módulo.
pub fn map_params(m: Module, a: f64, b: f64, sr: f64) -> (f64, f64) {
    match m {
        Module::HardClip => (a.max(0.01), b),
        Module::SoftClip => {
            let d = 1.0 + 19.0 * a;
            (d, 1.0 / tanh(d))
        }
        Module::Foldback => (a.max(0.02), 1.0 + 15.0 * b),
        Module::Bitcrush => {
            let bits = 1.0 + 15.0 * a * a;
            (1.0 / 2f64.powf(bits - 1.0), b)
        }
        Module::Downsample => (1.0 + 63.0 * a * a, b),
        Module::Rectifier => (if a >= 0.5 { 1.0 } else { 0.0 }, b),
        Module::TriggerSh => (a * a, b),
        Module::RingMod => (20.0 * 250f64.powf(a) / sr, b),
        Module::Slew => {
            let rate = 5.0 * 10000f64.powf(a) / sr;
            let up = rate * if b < 0.5 { 0.01f64.powf((0.5 - b) * 2.0) } else { 1.0 };
            let down = rate * if b > 0.5 { 0.01f64.powf((b - 0.5) * 2.0) } else { 1.0 };
            (up, down)
        }
        Module::TableShaper | Module::Off => (a, b),
    }
}

/// Procesa una muestra con el módulo `m`.
#[inline]
pub fn process(m: Module, x: f64, a: f64, b: f64, st: &mut SlotState, tables: &Tables) -> f64 {
    match m {
        Module::Off => x,
        Module::HardClip => x.clamp(-a, a) / a,
        Module::SoftClip => tanh(a * x) * b,
        Module::Foldback => {
            let mut x = x;
            let nmax = (b + 0.5).floor() as u32;
            let mut n = 0;
            while (x > a || x < -a) && n < nmax {
                x = if x > a { 2.0 * a - x } else { -2.0 * a - x };
                n += 1;
            }
            x.clamp(-a, a) / a
        }
        Module::Bitcrush => (x / a + 0.5).floor() * a,
        Module::Downsample => {
            if st.ds_count <= 0.0 {
                st.ds_hold = x;
                st.ds_count = (a + 0.5).floor();
            }
            st.ds_count -= 1.0;
            st.ds_hold
        }
        Module::Rectifier => {
            let r = x.max(0.0) + a * (-x).max(0.0);
            r + (x - r) * b
        }
        Module::TableShaper => {
            let raw = Tables::lookup(&tables.raw, x);
            let smooth = Tables::lookup(&tables.smooth, x);
            let shape = raw + (smooth - raw) * b;
            x + (shape - x) * a
        }
        Module::TriggerSh => {
            let trig = ((x >= 0.0) != (st.sh_prev >= 0.0)) || (x - st.sh_hold).abs() > a;
            if trig && st.sh_rng.next01() >= b {
                st.sh_hold = x;
            }
            st.sh_prev = x;
            st.sh_hold
        }
        Module::RingMod => {
            let s = (2.0 * std::f64::consts::PI * st.ring_phase).sin();
            let osc = s + (s.signum() * (s != 0.0) as i32 as f64 - s) * b;
            st.ring_phase += a;
            if st.ring_phase >= 1.0 {
                st.ring_phase -= 1.0;
            }
            x * osc
        }
        Module::Slew => {
            let d = (x - st.slew_y).clamp(-b, a);
            st.slew_y += d;
            st.slew_y
        }
    }
}

/// Canal R con SPREAD = `sp` > 0 (0.8). RING MOD: el oscilador se lee con la
/// fase adelantada 0.25·sp ciclos. DOWNSAMPLE: la retención ocurre `o` muestras
/// después del inicio de cada ciclo de N, con o = ⌊sp·N/2⌋. El resto de los
/// módulos no cambia (el xorshift del TRIGGER S/H ya es propio de cada canal).
#[inline(never)]
pub fn process_spread(m: Module, x: f64, a: f64, b: f64, st: &mut SlotState, tables: &Tables, sp: f64) -> f64 {
    match m {
        Module::RingMod => {
            let mut ph = st.ring_phase + 0.25 * sp;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            let s = (2.0 * std::f64::consts::PI * ph).sin();
            let osc = s + (s.signum() * (s != 0.0) as i32 as f64 - s) * b;
            st.ring_phase += a;
            if st.ring_phase >= 1.0 {
                st.ring_phase -= 1.0;
            }
            x * osc
        }
        Module::Downsample => {
            let n = (a + 0.5).floor();
            let o = (sp * 0.5 * n).floor();
            if st.ds_count <= 0.0 {
                st.ds_count = n;
            }
            if st.ds_count == n - o {
                st.ds_hold = x;
            }
            st.ds_count -= 1.0;
            st.ds_hold
        }
        _ => process(m, x, a, b, st, tables),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(m: Module, a: f64, b: f64, xs: &[f64]) -> Vec<f64> {
        let mut st = SlotState::new(12345);
        let mut t = Tables::default();
        t.ensure(1234);
        let (ta, tb) = map_params(m, a, b, 48000.0);
        xs.iter().map(|&x| process(m, x, ta, tb, &mut st, &t)).collect()
    }

    #[test]
    fn hardclip_normalizes_to_unity() {
        let y = run(Module::HardClip, 0.5, 0.0, &[2.0, -2.0, 0.25]);
        assert_eq!(y, vec![1.0, -1.0, 0.5]);
    }

    #[test]
    fn softclip_maps_unity_to_unity() {
        let y = run(Module::SoftClip, 0.7, 0.0, &[1.0]);
        assert!((y[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn foldback_stays_bounded() {
        let xs: Vec<f64> = (0..1000).map(|i| (i as f64 - 500.0) / 50.0).collect();
        for y in run(Module::Foldback, 0.3, 0.5, &xs) {
            assert!(y.abs() <= 1.0 + 1e-12);
        }
    }

    #[test]
    fn bitcrush_one_bit_is_ternary() {
        let y = run(Module::Bitcrush, 0.0, 0.0, &[0.9, 0.2, -0.7]);
        assert_eq!(y, vec![1.0, 0.0, -1.0]);
    }

    #[test]
    fn downsample_holds() {
        let xs: Vec<f64> = (0..8).map(|i| i as f64).collect();
        // a = 1/sqrt(63) * sqrt(3) -> N = 4
        let a = (3.0f64 / 63.0).sqrt();
        let y = run(Module::Downsample, a, 0.0, &xs);
        assert_eq!(y, vec![0.0, 0.0, 0.0, 0.0, 4.0, 4.0, 4.0, 4.0]);
    }

    #[test]
    fn table_is_normalized_and_centered() {
        let mut t = Tables::default();
        t.ensure(42);
        let peak = t.raw.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!((peak - 1.0).abs() < 1e-12);
        let c = (t.smooth[511] + t.smooth[512]) * 0.5;
        assert!(c.abs() < 1e-12);
    }

    #[test]
    fn b_usage_matches_jsfx() {
        assert!(!Module::HardClip.uses(true));
        assert!(Module::Foldback.uses(true));
        assert!(Module::HardClip.uses(false));
        assert!(!Module::Off.uses(false));
    }
}
