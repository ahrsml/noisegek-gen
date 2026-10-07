//! Generadores pseudoaleatorios. Idénticos a los del JSFX (que los emula con
//! doubles) para que los patrones y tablas coincidan entre versiones.

const TWO32: f64 = 4_294_967_296.0;

/// xorshift32 clásico (Marsaglia).
#[inline]
pub fn xorshift32(mut s: u32) -> u32 {
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    s
}

/// PRNG con estado: uniforme en [0, 1).
#[derive(Clone, Copy, Debug)]
pub struct XorShift(pub u32);

impl XorShift {
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }
    #[inline]
    pub fn next01(&mut self) -> f64 {
        self.0 = xorshift32(self.0);
        self.0 as f64 / TWO32
    }
}

/// Hash determinista -> [0, 1). Mezcla (n, carril, sal, seed) y pasa por 3
/// rondas de xorshift. Mismo resultado que `hash01` del JSFX.
pub fn hash01(n: i64, lane: u32, salt: u32, seed: u32) -> f64 {
    let v = (n as i128) * 73_856_093
        + (lane as i128) * 19_349_663
        + (salt as i128) * 83_492_791
        + (seed as i128) * 2_654_435
        + 1;
    let mut s = v.rem_euclid(1i128 << 32) as u32;
    if s == 0 {
        s = 1;
    }
    for _ in 0..3 {
        s = xorshift32(s);
    }
    s as f64 / TWO32
}

/// LCG barato para el ruido de audio (se usan los bits altos).
#[derive(Clone, Copy, Debug)]
pub struct Lcg(pub u32);

impl Lcg {
    #[inline]
    pub fn next01(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 as f64 / TWO32
    }
    #[inline]
    pub fn bipolar(&mut self) -> f64 {
        self.next01() * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift_known_values() {
        // Primeros valores de xorshift32 desde 1 (referencia de Marsaglia).
        let mut s = 1u32;
        s = xorshift32(s);
        assert_eq!(s, 270_369);
        s = xorshift32(s);
        assert_eq!(s, 67_634_689);
    }

    #[test]
    fn hash_is_deterministic_and_in_range() {
        for n in -50..50 {
            let a = hash01(n, 2, 3, 77);
            assert_eq!(a, hash01(n, 2, 3, 77));
            assert!((0.0..1.0).contains(&a));
        }
        assert_ne!(hash01(5, 0, 1, 0), hash01(5, 0, 1, 1));
    }
}
