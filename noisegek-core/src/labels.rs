//! Textos de la UI: nombres de opciones, etiquetas A/B según el módulo,
//! valores legibles y ayudas (en español, como el JSFX).

use noisegek_dsp::Module;

pub static MODULES: [&str; 11] = [
    "OFF", "HARD CLIP", "SOFT CLIP", "FOLDBACK", "BITCRUSH", "DOWNSAMPLE", "RECTIFIER",
    "TABLE SHAPER", "TRIGGER S/H", "RING MOD", "SLEW LIMITER",
];
pub static NOISES: [&str; 6] = ["OFF", "WHITE", "PINK", "BROWN", "DUST", "LFSR"];
pub static FILTER_MODES: [&str; 4] = ["OFF", "LOWPASS", "BANDPASS", "HIGHPASS"];
pub static FILTER_POS: [&str; 2] = ["POST CHAIN", "PRE CHAIN"];
pub static CLOCKS: [&str; 3] = ["AUTO", "HOST ONLY", "FREE RUN"];
pub static GATE_POS: [&str; 2] = ["PRE FX", "POST OUT"];
pub static DIRS: [&str; 5] = ["FWD", "BWD", "PINGPONG", "RANDOM", "DRUNK"];
/// Nombres "fijos" de destinos para el host (la UI muestra los dinámicos).
/// Los índices 0..18 conservan su significado; los de la 0.8 van al final.
pub static DESTS_STATIC: [&str; noisegek_dsp::NUM_DEST] = [
    "OFF", "S1 A", "S1 B", "S2 A", "S2 B", "S3 A", "S3 B", "S4 A", "S4 B", "DRIVE", "MIX",
    "CUTOFF", "RESO", "NOISE LVL", "PAN", "S1 ON/OFF", "S2 ON/OFF", "S3 ON/OFF", "S4 ON/OFF",
    "FB AMOUNT", "FB TIME", "SPREAD",
];
pub static FB_TAPS: [&str; 2] = ["PRE FILTER", "POST FILTER"];
pub static ENV_TAPS: [&str; 2] = ["INPUT", "PRE-FILTER"];
/// En el instrumento (sin entrada de audio) el índice 0 es la voz.
pub static ENV_TAPS_GEN: [&str; 2] = ["VOICE", "PRE-FILTER"];
pub static LANE_SOURCES: [&str; 3] = ["SEQ", "ENV", "SEQ×ENV"];

/// Nombre real del parámetro A/B según el módulo.
pub fn pab_label(m: Module, b: bool) -> &'static str {
    use Module::*;
    if !m.uses(b) {
        return if b { "B" } else { "A" };
    }
    match (m, b) {
        (Foldback, true) => "FOLDS",
        (Rectifier, true) => "ORIG",
        (TableShaper, true) => "SMOOTH",
        (TriggerSh, true) => "SKIP",
        (RingMod, true) => "SHAPE",
        (Slew, true) => "ASYM",
        (SoftClip, false) => "DRIVE",
        (Bitcrush, false) => "BITS",
        (Downsample, false) => "FACTOR",
        (Rectifier, false) => "MODE",
        (TableShaper, false) => "AMOUNT",
        (RingMod, false) => "FREQ",
        (Slew, false) => "SPEED",
        _ => "THRESH",
    }
}

/// Valor legible de A/B (v = valor crudo 0..1).
pub fn pab_text(m: Module, b: bool, v: f32) -> String {
    use Module::*;
    if !m.uses(b) {
        return "-".into();
    }
    let v = v as f64;
    if b {
        return match m {
            Foldback => format!("{}", (1.0 + 15.0 * v + 0.5).floor()),
            Slew => {
                let d = ((v - 0.5) * 200.0 + 0.5).floor();
                format!("{}{}%", if d > 0.0 { "+" } else { "" }, d)
            }
            _ => format!("{}%", (v * 100.0 + 0.5).floor()),
        };
    }
    match m {
        HardClip => format!("{:.2}", v.max(0.01)),
        SoftClip => format!("x{:.1}", 1.0 + 19.0 * v),
        Foldback => format!("{:.2}", v.max(0.02)),
        Bitcrush => format!("{:.1} bit", 1.0 + 15.0 * v * v),
        Downsample => format!("1/{}", (1.0 + 63.0 * v * v + 0.5).floor()),
        Rectifier => (if v >= 0.5 { "full" } else { "half" }).into(),
        TableShaper => format!("{}%", (v * 100.0 + 0.5).floor()),
        TriggerSh => format!("{:.3}", v * v),
        RingMod => {
            let f = 20.0 * 250f64.powf(v);
            if f >= 1000.0 {
                format!("{:.2}k", f / 1000.0)
            } else {
                format!("{f:.0} Hz")
            }
        }
        _ => format!("{:.0}/s", 5.0 * 10000f64.powf(v)),
    }
}

pub fn pab_help(m: Module, b: bool) -> &'static str {
    use Module::*;
    if m == Off {
        return "Slot apagado: elige un módulo en el menú de arriba.";
    }
    if !m.uses(b) {
        return "Este módulo no usa el parámetro B.";
    }
    if b {
        return match m {
            Foldback => "FOLDS: máximo de pliegues (1..16). Más = más caótico y metálico.",
            Rectifier => "ORIG: cuánto vuelve de la señal original sin rectificar.",
            TableShaper => "SMOOTH: suavizado de la curva. Bajo = caos ruidoso, alto = saturación suave.",
            TriggerSh => "SKIP: probabilidad de ignorar disparos -> escalones erráticos.",
            RingMod => "SHAPE: forma del oscilador, de seno (0%) a cuadrada (100%).",
            _ => "ASYM: <0 sube lento, >0 baja lento. Redondea solo un lado de la onda.",
        };
    }
    match m {
        HardClip => "THRESH: umbral de recorte duro. Más bajo = más distorsión, más áspero.",
        SoftClip => "DRIVE: saturación tanh interna (x1..x20). De cálido a muy comprimido.",
        Foldback => "THRESH: umbral de pliegue. Más bajo = más pliegues y armónicos metálicos.",
        Bitcrush => "BITS: resolución. Menos bits = escalones y ruido de cuantización.",
        Downsample => "FACTOR: reducción de muestreo (1..64). Aliasing lo-fi y metálico.",
        Rectifier => "MODE: media onda (<50%) u onda completa (>=50%, sube una octava).",
        TableShaper => "AMOUNT: intensidad del waveshaper. T.SEED (abajo) elige la curva.",
        TriggerSh => "THRESH: umbral de disparo del sample & hold. Alto = escalones largos.",
        RingMod => "FREQ: frecuencia del oscilador (20 Hz..5 kHz). Bajo = trémolo, alto = metálico.",
        _ => "SPEED: velocidad máxima de cambio. Bajo = filtra y redondea (oscuro, lento).",
    }
}

/// Nombre dinámico del destino: S1..S4 A/B usan el nombre real del parámetro.
pub fn dest_name(i: usize, modules: &[Module; 4]) -> String {
    if (1..=8).contains(&i) {
        let s = (i - 1) / 2;
        let b = (i - 1) % 2 == 1;
        format!("S{} {}", s + 1, pab_label(modules[s], b))
    } else {
        DESTS_STATIC.get(i).copied().unwrap_or("OFF").to_string()
    }
}

pub fn noise_char_label(t: usize) -> &'static str {
    match t {
        1 | 2 => "WIDTH",
        3 => "DARK",
        4 => "DENSITY",
        5 => "CLOCK",
        _ => "CHAR",
    }
}

pub fn noise_char_text(t: usize, v: f32) -> String {
    let v = v as f64;
    match t {
        4 => format!("{:.0}/s", 2.0 + v * v * v * 20000.0),
        5 => format!("{:.0} Hz", 50.0 * 2f64.powf(v * 9.5)),
        _ => format!("{:.0}%", v * 100.0),
    }
}

pub fn noise_char_help(t: usize) -> &'static str {
    match t {
        1 | 2 => "WIDTH: ancho estéreo del ruido (0 = mono, 100% = L/R independientes).",
        3 => "DARK: más alto = más grave y lento.",
        4 => "DENSITY: cuántos impulsos por segundo (2..20000).",
        5 => "CLOCK: velocidad del LFSR. Bajo = zumbido tonal, alto = ruido digital.",
        _ => "CHAR: depende del tipo de ruido (elige uno arriba).",
    }
}

pub fn noise_desc(t: usize) -> [&'static str; 3] {
    match t {
        1 => ["Blanco: todas las", "frecuencias igual.", "Siseo brillante."],
        2 => ["Rosa: -3 dB/oct.", "Equilibrado, tipo", "lluvia o cascada."],
        3 => ["Café: -6 dB/oct.", "Grave: rumor,", "viento, trueno."],
        4 => ["Dust: impulsos al", "azar. Crujido de", "vinilo, chispas."],
        5 => ["LFSR 15 bits (NES).", "De zumbido tonal", "a ruido áspero."],
        _ => ["Sin ruido interno.", "Solo procesa la", "entrada de audio."],
    }
}

/// La versión sale del paquete (antes estaba escrita a mano y quedó en 0.6).
pub const HELP_DEFAULT: &str = concat!(
    "Arrastra = ajustar · Shift = fino · Doble clic = default · Rueda = paso a paso · v",
    env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"),
    " © Adolfo Rosas (@ahrsml)"
);
