//! SOURCE: generador de ruido interno (por canal).

use crate::rng::Lcg;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum NoiseType {
    #[default]
    Off = 0,
    White = 1,
    Pink = 2,
    Brown = 3,
    Dust = 4,
    Lfsr = 5,
}

impl NoiseType {
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::White,
            2 => Self::Pink,
            3 => Self::Brown,
            4 => Self::Dust,
            5 => Self::Lfsr,
            _ => Self::Off,
        }
    }
}

/// Coeficientes que dependen de "Character" y del sample rate.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoiseCoefs {
    pub brown_lk: f64,
    pub brown_g: f64,
    pub dust_p: f64,
    pub lfsr_inc: f64,
    /// Modo "corto" del LFSR (NES modo 1, toma del bit 6): secuencia de 93
    /// pasos -> suena afinado a reloj / 93. Lo usa el modo PITCH del GEN.
    pub lfsr_short: bool,
}

impl NoiseCoefs {
    pub fn new(character: f64, sr: f64) -> Self {
        let lk = 0.0002 + 0.05 * (1.0 - character).powi(2);
        Self {
            brown_lk: lk,
            brown_g: (0.6 * lk).sqrt(),
            dust_p: (2.0 + character.powi(3) * 20000.0) / sr,
            lfsr_inc: (50.0 * 2f64.powf(character * 9.5) / sr).min(4.0),
            lfsr_short: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NoiseChannel {
    lcg: Lcg,
    pink: [f64; 7],
    brown: f64,
    dust: f64,
    lfsr: u16,
    lfsr_ph: f64,
}

impl NoiseChannel {
    pub fn new(lcg_seed: u32, lfsr_seed: u16) -> Self {
        Self {
            lcg: Lcg(lcg_seed),
            pink: [0.0; 7],
            brown: 0.0,
            dust: 0.0,
            lfsr: lfsr_seed.max(1),
            lfsr_ph: 0.0,
        }
    }

    #[inline]
    pub fn white(&mut self) -> f64 {
        self.lcg.bipolar()
    }

    /// Filtro "refined" de Paul Kellet (-3 dB/oct).
    #[inline]
    pub fn pink(&mut self) -> f64 {
        let w = self.white();
        let b = &mut self.pink;
        b[0] = 0.99886 * b[0] + w * 0.0555179;
        b[1] = 0.99332 * b[1] + w * 0.0750759;
        b[2] = 0.96900 * b[2] + w * 0.1538520;
        b[3] = 0.86650 * b[3] + w * 0.3104856;
        b[4] = 0.55000 * b[4] + w * 0.5329522;
        b[5] = -0.7616 * b[5] - w * 0.0168980;
        let out = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
        b[6] = w * 0.115926;
        out * 0.11
    }

    #[inline]
    pub fn brown(&mut self, c: &NoiseCoefs) -> f64 {
        let w = self.white();
        self.brown = self.brown * (1.0 - c.brown_lk) + w * c.brown_g;
        self.brown
    }

    #[inline]
    pub fn dust(&mut self, c: &NoiseCoefs) -> f64 {
        self.dust *= 0.45;
        if self.lcg.next01() < c.dust_p {
            self.dust = self.white();
        }
        self.dust
    }

    /// LFSR de 15 bits (canal de ruido estilo NES), salida ±0.7.
    #[inline]
    pub fn lfsr(&mut self, c: &NoiseCoefs) -> f64 {
        self.lfsr_ph += c.lfsr_inc;
        while self.lfsr_ph >= 1.0 {
            self.lfsr_ph -= 1.0;
            let s = self.lfsr;
            let tap = if c.lfsr_short { 6 } else { 1 };
            let fb = (s & 1) ^ ((s >> tap) & 1);
            self.lfsr = (s >> 1) | (fb << 14);
        }
        if self.lfsr & 1 == 1 {
            0.7
        } else {
            -0.7
        }
    }
}

/// Fuente estéreo completa.
#[derive(Clone, Copy, Debug)]
pub struct NoiseSource {
    pub ch: [NoiseChannel; 2],
}

impl Default for NoiseSource {
    fn default() -> Self {
        Self {
            ch: [NoiseChannel::new(22222, 1), NoiseChannel::new(777777, 0x5A5A)],
        }
    }
}

impl NoiseSource {
    /// Muestra estéreo sin escalar por nivel. `character` = ancho estéreo en
    /// white/pink.
    #[inline]
    pub fn frame(&mut self, t: NoiseType, character: f64, c: &NoiseCoefs) -> (f64, f64) {
        let [l, r] = &mut self.ch;
        match t {
            NoiseType::Off => (0.0, 0.0),
            NoiseType::White => {
                let a = l.white();
                let b = r.white();
                (a, a + (b - a) * character)
            }
            NoiseType::Pink => {
                let a = l.pink();
                let b = r.pink();
                (a, a + (b - a) * character)
            }
            NoiseType::Brown => (l.brown(c), r.brown(c)),
            NoiseType::Dust => (l.dust(c), r.dust(c)),
            NoiseType::Lfsr => (l.lfsr(c), r.lfsr(c)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(t: NoiseType, character: f64) -> f64 {
        let mut s = NoiseSource::default();
        let c = NoiseCoefs::new(character, 48000.0);
        let n = 96000;
        let mut acc = 0.0;
        for _ in 0..n {
            let (l, _) = s.frame(t, character, &c);
            assert!(l.is_finite() && l.abs() < 4.0);
            acc += l * l;
        }
        (acc / n as f64).sqrt()
    }

    #[test]
    fn levels_are_sane() {
        assert!((rms(NoiseType::White, 0.5) - 0.577).abs() < 0.02);
        let p = rms(NoiseType::Pink, 0.5);
        assert!(p > 0.05 && p < 0.6, "pink {p}");
        let b = rms(NoiseType::Brown, 0.5);
        assert!(b > 0.05 && b < 0.8, "brown {b}");
        assert!(rms(NoiseType::Lfsr, 0.5) > 0.69);
        assert!(rms(NoiseType::Dust, 0.5) > 0.0);
    }
}
