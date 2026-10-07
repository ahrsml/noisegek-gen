//! Tipos del secuenciador: patrón, carriles, reloj y operaciones de edición.

use crate::rng::XorShift;

pub const NUM_LANES: usize = 4; // 0 = GATE, 1..3 = MOD 1..3
pub const NUM_STEPS: usize = 16;
pub const NUM_DEST: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Direction {
    #[default]
    Forward = 0,
    Backward = 1,
    PingPong = 2,
    Random = 3,
    Drunk = 4,
}

impl Direction {
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Backward,
            2 => Self::PingPong,
            3 => Self::Random,
            4 => Self::Drunk,
            _ => Self::Forward,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Clock {
    /// Sigue al host al reproducir, corre libre en stop.
    #[default]
    Auto = 0,
    /// Sigue al host; en stop se congela (gate abierto).
    HostOnly = 1,
    /// Siempre libre al tempo del proyecto.
    Free = 2,
}

/// Destinos de modulación (índices fijos, iguales al JSFX).
pub mod dest {
    pub const OFF: u8 = 0;
    /// 1..8 = S1 A, S1 B, S2 A, ... S4 B
    pub const DRIVE: u8 = 9;
    pub const MIX: u8 = 10;
    pub const CUTOFF: u8 = 11;
    pub const RESO: u8 = 12;
    pub const NOISE: u8 = 13;
    pub const PAN: u8 = 14;
    /// 15..18 = S1..S4 On/Off
    pub const SLOT_ON_FIRST: u8 = 15;
    pub const SLOT_ON_LAST: u8 = 18;
    /// 0.8: realimentación (al final de la tabla; 1..18 no cambian)
    pub const FB_AMOUNT: u8 = 19;
    pub const FB_TIME: u8 = 20;
    pub const SPREAD: u8 = 21;
}

/// Pasos por negra para cada índice de Rate (0 = 8 compases .. 11 = 1/64).
pub fn rate_spq(r: u8, ts_num: f64, ts_den: f64) -> f64 {
    let bar = ts_num * 4.0 / ts_den.max(1.0);
    match r {
        0 => 1.0 / (8.0 * bar),
        1 => 1.0 / (4.0 * bar),
        2 => 1.0 / (2.0 * bar),
        3 => 1.0 / bar,
        4 => 0.5,
        5 => 1.0,
        6 => 1.5,
        7 => 2.0,
        8 => 3.0,
        9 => 4.0,
        10 => 8.0,
        _ => 16.0,
    }
}

pub const RATE_NAMES: [&str; 12] = [
    "8 BAR", "4 BAR", "2 BAR", "1 BAR", "1/2", "1/4", "1/8T", "1/8", "1/16T", "1/16", "1/32", "1/64",
];

/// Fuente de un carril MOD (0.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LaneSource {
    /// Valor del paso con glide (comportamiento anterior).
    #[default]
    Seq = 0,
    /// Seguidor de envolvente.
    Env = 1,
    /// Producto de ambos.
    SeqEnv = 2,
}

impl LaneSource {
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Env,
            2 => Self::SeqEnv,
            _ => Self::Seq,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LaneParams {
    pub rate: u8,
    pub steps: u8,
    pub dir: Direction,
    /// 0..1
    pub glide: f64,
    /// 0..100
    pub prob: f64,
    /// 0..100
    pub mutate: f64,
    pub dest: u8,
    /// -1..1
    pub depth: f64,
    /// Fuente de la modulación (solo carriles MOD)
    pub src: LaneSource,
}

impl LaneParams {
    pub const fn new(rate: u8, steps: u8, dir: Direction, glide: f64) -> Self {
        Self { rate, steps, dir, glide, prob: 100.0, mutate: 0.0, dest: 0, depth: 0.5, src: LaneSource::Seq }
    }
}

/// Patrón editable: 4 carriles x 16 valores 0..1 + golpes euclidianos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pattern {
    pub steps: [[f32; NUM_STEPS]; NUM_LANES],
    pub euc_k: [u8; NUM_LANES],
}

impl Default for Pattern {
    /// El mismo patrón por defecto que el JSFX.
    fn default() -> Self {
        let mut steps = [[0.0f32; NUM_STEPS]; NUM_LANES];
        for i in 0..NUM_STEPS {
            steps[0][i] = if (i * 7) % 16 < 7 {
                if i % 4 == 0 {
                    1.0
                } else {
                    0.7
                }
            } else {
                0.0
            };
            steps[1][i] = i as f32 / 15.0;
            let fi = i as f64;
            steps[2][i] = (0.5 + 0.5 * (fi * 0.9).sin() * (fi * 0.37).cos()) as f32;
            steps[3][i] = if i % 2 == 1 { 0.2 } else { 0.8 };
        }
        Self { steps, euc_k: [7, 0, 0, 0] }
    }
}

impl Pattern {
    /// Euclidiano (Bresenham): golpe en i si (i*k mod len) < k.
    pub fn euclid(&mut self, lane: usize, k: u8, len: u8) {
        let len = len.clamp(1, NUM_STEPS as u8) as usize;
        let k = (k as usize).min(len);
        self.euc_k[lane] = k as u8;
        for i in 0..NUM_STEPS {
            self.steps[lane][i] = if i < len && (i * k) % len < k { 1.0 } else { 0.0 };
        }
    }

    /// Rota los `len` pasos activos (dir > 0 derecha, < 0 izquierda).
    pub fn rotate(&mut self, lane: usize, dir: i32, len: u8) {
        let len = len.clamp(1, NUM_STEPS as u8) as usize;
        let s = &mut self.steps[lane][..len];
        if dir > 0 {
            s.rotate_right(1);
        } else {
            s.rotate_left(1);
        }
    }

    /// Valores al azar: el gate tiende a golpes, los mods a continuos.
    pub fn randomize(&mut self, lane: usize, rng: &mut XorShift) {
        for i in 0..NUM_STEPS {
            let r = rng.next01();
            self.steps[lane][i] = if lane == 0 {
                if r < 0.45 {
                    0.0
                } else {
                    (0.5 + rng.next01() * 0.5) as f32
                }
            } else {
                r as f32
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gate_is_e7_16() {
        let p = Pattern::default();
        let hits: Vec<usize> = (0..16).filter(|&i| p.steps[0][i] > 0.0).collect();
        assert_eq!(hits, vec![0, 3, 5, 7, 10, 12, 14]);
    }

    #[test]
    fn euclid_counts_hits() {
        let mut p = Pattern::default();
        p.euclid(1, 5, 12);
        let hits = p.steps[1].iter().filter(|&&v| v > 0.5).count();
        assert_eq!(hits, 5);
        assert!(p.steps[1][12..].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn rotate_roundtrip() {
        let mut p = Pattern::default();
        let orig = p;
        p.rotate(2, 1, 9);
        assert_ne!(p, orig);
        p.rotate(2, -1, 9);
        assert_eq!(p, orig);
    }
}
