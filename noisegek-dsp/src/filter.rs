//! SVF de Andrew Simper (Cytomic), trapezoidal: estable con modulación rápida.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FilterMode {
    #[default]
    Off = 0,
    Lowpass = 1,
    Bandpass = 2,
    Highpass = 3,
}

impl FilterMode {
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Lowpass,
            2 => Self::Bandpass,
            3 => Self::Highpass,
            _ => Self::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SvfCoefs {
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    scream: bool,
}

/// Con SCREAM, k = 2 − SCREAM_K·reso: en reso = 1, k = −0.04 (autooscilación).
pub const SCREAM_K: f64 = 2.04;

impl SvfCoefs {
    /// `reso` en 0..1 (k = 2 - 1.96 reso). Con `scream`, k = 2 - 2.04 reso y
    /// los integradores saturan (ver `Svf::process`).
    pub fn new(cutoff_hz: f64, reso: f64, sr: f64, scream: bool) -> Self {
        let g = (std::f64::consts::PI * cutoff_hz.min(0.45 * sr) / sr).tan();
        let k = if scream { 2.0 - SCREAM_K * reso } else { 2.0 - 1.96 * reso };
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        Self { k, a1, a2, a3: g * a2, scream }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f64,
    ic2: f64,
}

impl Svf {
    #[inline]
    pub fn process(&mut self, x: f64, c: &SvfCoefs, mode: FilterMode) -> f64 {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        if c.scream {
            // Integradores saturados: |ic| < 1 acota la energía del filtro
            // aunque k < 0, y la oscilación queda en un ciclo límite.
            self.ic1 = (2.0 * v1 - self.ic1).clamp(-20.0, 20.0).tanh();
            self.ic2 = (2.0 * v2 - self.ic2).clamp(-20.0, 20.0).tanh();
            if !(self.ic1.abs() <= 1.0 && self.ic2.abs() <= 1.0) {
                *self = Self::default();
            }
        } else {
            self.ic1 = 2.0 * v1 - self.ic1;
            self.ic2 = 2.0 * v2 - self.ic2;
        }
        match mode {
            FilterMode::Lowpass => v2,
            FilterMode::Bandpass => v1,
            FilterMode::Highpass => x - c.k * v1 - v2,
            FilterMode::Off => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gain_at(freq: f64, cutoff: f64, mode: FilterMode) -> f64 {
        let sr = 48000.0;
        let c = SvfCoefs::new(cutoff, 0.0, sr, false);
        let mut f = Svf::default();
        let mut peak: f64 = 0.0;
        for i in 0..48000 {
            let x = (2.0 * std::f64::consts::PI * freq * i as f64 / sr).sin();
            let y = f.process(x, &c, mode);
            if i > 24000 {
                peak = peak.max(y.abs());
            }
        }
        peak
    }

    #[test]
    fn lowpass_passes_lows_cuts_highs() {
        assert!(gain_at(100.0, 2000.0, FilterMode::Lowpass) > 0.95);
        assert!(gain_at(15000.0, 500.0, FilterMode::Lowpass) < 0.01);
    }

    #[test]
    fn highpass_cuts_lows() {
        assert!(gain_at(50.0, 5000.0, FilterMode::Highpass) < 0.01);
        assert!(gain_at(15000.0, 500.0, FilterMode::Highpass) > 0.95);
    }
}
