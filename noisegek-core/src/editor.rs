//! Interfaz (egui). Misma organización, paleta y textos que el JSFX.
//! Desde la 0.8 hay una fila FEEDBACK · ENV FOLLOW · STEREO bajo SEQUENCER; el
//! instrumento (GEN) agrega además la fila VOICE antes de ella. Todo se dibuja
//! en un lienzo de diseño de 1000 x 632 (GEN: 1000 x 686) que se escala para
//! llenar la ventana (redimensionable desde la esquina inferior derecha).
//!
//! Controles:
//!   barra    : arrastrar · Shift = fino · doble clic = default · rueda
//!   selector : clic = menú · rueda = anterior/siguiente
//!   pasos    : clic izq. arrastrado dibuja · clic der. arrastrado borra

use crate::labels::{self, HELP_DEFAULT};
use crate::params::{NgParams, UiParams, VoiceUiParams};
use crate::{Shared, UiSnap};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{
    self, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Pos2, Rect, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow};
use noisegek_dsp::rng::XorShift;
use noisegek_dsp::{Module, Pattern, NUM_LANES, NUM_STEPS};
use std::sync::Arc;

// --- Paleta NOISEGEK ---------------------------------------------------------
const VOID: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
const IRON: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
const WHITE: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);
const RED: Color32 = Color32::from_rgb(0xFF, 0x00, 0x33);
const SHADOW: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);
/// Esquinas rectas (radio 0).
const R0: f32 = 0.0;

fn al(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

// Fuentes libres (SIL OFL) incluidas en el plugin: Anton ~ Impact,
// Courier Prime ~ Courier.
const ANTON: &[u8] = include_bytes!("../assets/fonts/Anton-Regular.ttf");
const CP_BOLD: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Bold.ttf");
const CP_REG: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Regular.ttf");

fn title_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("anton".into()))
}
fn mono_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("monob".into()))
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono".into()))
}

const DW: f32 = 1000.0;
const DH: f32 = 632.0;
/// Alto de una fila de panel (SEQUENCER, VOICE, FEEDBACK...) más su separación.
const ROW: f32 = 54.0;
/// Alto de las filas de panel y distancia entre carriles.
const ROW_H: f32 = 50.0;
const LANE_STEP: f32 = 74.0;

/// Lienzo: convierte unidades de diseño a píxeles.
#[derive(Clone, Copy)]
struct Cv {
    o: Pos2,
    s: f32,
}

impl Cv {
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            Pos2::new(self.o.x + x * self.s, self.o.y + y * self.s),
            Vec2::new(w * self.s, h * self.s),
        )
    }
    fn f(&self, size: f32) -> f32 {
        size * self.s
    }
}

pub struct EditorState {
    rng: XorShift,
    drag_norm: f32,
    step_last: Option<usize>,
    help: String,
}

pub fn create<P: UiParams>(params: Arc<P>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let egui_state = params.core().editor_state.clone();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(7);
    create_egui_editor(
        params.core().editor_state.clone(),
        EditorState { rng: XorShift::new(seed), drag_norm: 0.0, step_last: None, help: String::new() },
        |ctx, _| setup_style(ctx),
        move |ctx, setter, st| {
            let snap = shared.snap.lock().map(|s| *s).unwrap_or_default();
            ResizableWindow::new("noisegek-window")
                .min_size(Vec2::new(600.0, 380.0))
                .show(ctx, egui_state.as_ref(), |ui| {
                    draw(ui, setter, params.core(), params.voice(), &snap, st);
                });
        },
    )
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallback: Vec<String> =
        fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (name, data) in [("anton", ANTON), ("cp_bold", CP_BOLD), ("cp", CP_REG)] {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(data)));
    }
    for (family, font) in [("anton", "anton"), ("monob", "cp_bold"), ("mono", "cp")] {
        let mut list = vec![font.to_string()];
        list.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(fonts);

    let mut v = egui::Visuals::dark();
    v.panel_fill = VOID;
    v.window_fill = IRON;
    v.extreme_bg_color = VOID;
    v.window_stroke = Stroke::new(1.0_f32, SHADOW);
    v.override_text_color = Some(WHITE);
    v.selection.bg_fill = RED;
    v.widgets.hovered.weak_bg_fill = SHADOW;
    v.widgets.hovered.bg_fill = SHADOW;
    v.widgets.active.weak_bg_fill = RED;
    ctx.set_visuals(v);
}

// --- Primitivas --------------------------------------------------------------

fn text(ui: &Ui, rect: Rect, align: Align2, s: &str, font: FontId, color: Color32) {
    let p = ui.painter().with_clip_rect(rect);
    let pos = align.pos_in_rect(&rect.shrink2(Vec2::new(rect.height() * 0.22, 0.0)));
    p.text(pos, align, s, font, color);
}

fn panel(ui: &Ui, cv: &Cv, x: f32, y: f32, w: f32, h: f32, title: Option<&str>, strip: Color32) {
    let p = ui.painter();
    p.rect_filled(cv.r(x, y, w, h), R0, IRON);
    p.rect_filled(cv.r(x, y, 3.0, h), R0, strip);
    if let Some(t) = title {
        // Título con "fantasma" rojo desplazado (texto desalineado, decorativo)
        p.text(cv.r(x + 12.0, y + 2.0, 0.0, 0.0).min, Align2::LEFT_TOP, t, title_font(cv.f(18.0)), al(RED, 0.45));
        p.text(cv.r(x + 10.0, y + 1.0, 0.0, 0.0).min, Align2::LEFT_TOP, t, title_font(cv.f(18.0)), WHITE);
    }
}

fn btn(ui: &mut Ui, cv: &Cv, st: &mut EditorState, x: f32, y: f32, w: f32, h: f32, label: &str, help: &str) -> egui::Response {
    let rect = cv.r(x, y, w, h);
    let resp = ui.interact(rect, Id::new(("btn", x as i32, y as i32)), Sense::click());
    if resp.hovered() {
        st.help = help.to_string();
    }
    ui.painter().rect_filled(rect, R0, if resp.hovered() { RED } else { SHADOW });
    text(ui, rect, Align2::CENTER_CENTER, label, mono_bold(cv.f(13.0)), WHITE);
    resp
}

/// Cambio puntual (menú, rueda, doble clic): begin/set/end en un paso.
fn set_once<P: Param>(setter: &ParamSetter, p: &P, norm: f32) {
    setter.begin_set_parameter(p);
    setter.set_parameter_normalized(p, norm.clamp(0.0, 1.0));
    setter.end_set_parameter(p);
}

#[allow(clippy::too_many_arguments)]
fn bar<P: Param>(
    ui: &mut Ui,
    cv: &Cv,
    st: &mut EditorState,
    setter: &ParamSetter,
    p: &P,
    (x, y, w, h): (f32, f32, f32, f32),
    label: &str,
    value: String,
    help: &str,
    mod_norm: Option<f32>,
    dim: bool,
    bipolar: bool,
) {
    let rect = cv.r(x, y, w, h);
    let resp = ui.interact(rect, Id::new(("bar", x as i32, y as i32)), Sense::click_and_drag());
    let fine = ui.input(|i| i.modifiers.shift);
    if resp.hovered() {
        st.help = help.to_string();
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 && !resp.dragged() {
            let stepn = match p.step_count() {
                Some(n) if n > 0 => 1.0 / n as f32,
                _ => if fine { 0.005 } else { 0.02 },
            };
            set_once(setter, p, p.unmodulated_normalized_value() + stepn * scroll.signum());
        }
    }
    if resp.double_clicked() {
        set_once(setter, p, p.default_normalized_value());
    } else {
        if resp.drag_started() {
            st.drag_norm = p.unmodulated_normalized_value();
            setter.begin_set_parameter(p);
        }
        if resp.dragged() {
            let d = resp.drag_delta();
            st.drag_norm = (st.drag_norm + (d.x - d.y) / (250.0 * cv.s) * if fine { 0.1 } else { 1.0 }).clamp(0.0, 1.0);
            setter.set_parameter_normalized(p, st.drag_norm);
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(p);
        }
    }

    let active = resp.hovered() || resp.dragged();
    let t = p.unmodulated_normalized_value();
    let painter = ui.painter();
    painter.rect_filled(rect, R0, VOID);
    let fill = if dim { al(SHADOW, 0.3) } else if active { al(WHITE, 0.32) } else { SHADOW };
    if bipolar {
        let (a, b) = if t > 0.5 { (0.5, t) } else { (t, 0.5) };
        painter.rect_filled(cv.r(x + w * a, y, w * (b - a), h), R0, fill);
        painter.rect_filled(cv.r(x + w * 0.5, y, 1.0 / cv.s, h), R0, al(WHITE, 0.35));
    } else {
        painter.rect_filled(cv.r(x, y, w * t, h), R0, fill);
    }
    if let Some(m) = mod_norm {
        if (m - t).abs() > 1e-4 {
            painter.rect_filled(cv.r(x + w * m.clamp(0.0, 1.0) - 1.5, y, 3.0, h), R0, RED);
        }
    }
    if active {
        painter.rect_stroke(rect, R0, Stroke::new(1.0_f32, al(WHITE, 0.9)), StrokeKind::Inside);
    }
    let la = if dim { 0.3 } else { 0.6 };
    text(ui, rect, Align2::LEFT_CENTER, label, mono_bold(cv.f(12.0)), al(WHITE, la));
    text(ui, rect, Align2::RIGHT_CENTER, &value, mono_bold(cv.f(13.0)), al(WHITE, if dim { 0.35 } else { 1.0 }));
}

#[allow(clippy::too_many_arguments)]
fn sel(
    ui: &mut Ui,
    cv: &Cv,
    st: &mut EditorState,
    setter: &ParamSetter,
    p: &IntParam,
    (x, y, w, h): (f32, f32, f32, f32),
    label: Option<&str>,
    names: &[String],
    help: &str,
) {
    let rect = cv.r(x, y, w, h);
    let id = Id::new(("sel", x as i32, y as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let popup_id = id.with("popup");
    let cur = p.value().clamp(0, names.len() as i32 - 1) as usize;
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    if resp.hovered() {
        st.help = help.to_string();
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let n = names.len() as i32;
            let next = (cur as i32 + if scroll > 0.0 { -1 } else { 1 }).rem_euclid(n);
            setter.begin_set_parameter(p);
            setter.set_parameter(p, next);
            setter.end_set_parameter(p);
        }
    }
    let painter = ui.painter();
    painter.rect_filled(rect, R0, if resp.hovered() { SHADOW } else { VOID });
    painter.rect_filled(cv.r(x, y + h - 2.0, w, 2.0), R0, if resp.hovered() { RED } else { al(WHITE, 0.35) });
    let inner = cv.r(x, y, w - 18.0, h);
    if let Some(l) = label {
        text(ui, inner, Align2::LEFT_CENTER, l, mono_bold(cv.f(12.0)), al(WHITE, 0.6));
        text(ui, inner, Align2::RIGHT_CENTER, &names[cur], mono_bold(cv.f(13.0)), WHITE);
    } else {
        text(ui, inner, Align2::LEFT_CENTER, &names[cur], mono_bold(cv.f(13.0)), WHITE);
    }
    // flecha
    let c = cv.r(x + w - 11.0, y + h / 2.0, 0.0, 0.0).min;
    let k = cv.s;
    painter.add(egui::Shape::convex_polygon(
        vec![Pos2::new(c.x - 4.0 * k, c.y - 2.0 * k), Pos2::new(c.x + 4.0 * k, c.y - 2.0 * k), Pos2::new(c.x, c.y + 3.0 * k)],
        al(WHITE, 0.6),
        Stroke::NONE,
    ));

    #[allow(deprecated)]
    egui::popup::popup_below_widget(ui, popup_id, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(rect.width());
        for (i, n) in names.iter().enumerate() {
            let rt = egui::RichText::new(n).font(mono_bold(cv.f(13.0))).color(if i == cur { RED } else { WHITE });
            if ui.selectable_label(i == cur, rt).clicked() {
                setter.begin_set_parameter(p);
                setter.set_parameter(p, i as i32);
                setter.end_set_parameter(p);
            }
        }
    });
}

/// Interruptor ON/OFF (BoolParam): clic alterna. LED rojo = encendido.
fn toggle(ui: &mut Ui, cv: &Cv, st: &mut EditorState, setter: &ParamSetter, p: &BoolParam,
          (x, y, w, h): (f32, f32, f32, f32), label: &str, help: &str) {
    let rect = cv.r(x, y, w, h);
    let resp = ui.interact(rect, Id::new(("tgl", x as i32, y as i32)), Sense::click());
    let on = p.value();
    if resp.hovered() {
        st.help = help.to_string();
    }
    if resp.clicked() {
        setter.begin_set_parameter(p);
        setter.set_parameter(p, !on);
        setter.end_set_parameter(p);
    }
    let painter = ui.painter();
    painter.rect_filled(rect, R0, if resp.hovered() { SHADOW } else { VOID });
    painter.rect_filled(cv.r(x + 6.0, y + h / 2.0 - 4.0, 8.0, 8.0), R0, if on { RED } else { SHADOW });
    text(ui, cv.r(x + 12.0, y, w - 12.0, h), Align2::LEFT_CENTER, label, mono_bold(cv.f(12.0)), al(WHITE, 0.6));
    text(ui, rect, Align2::RIGHT_CENTER, if on { "ON" } else { "OFF" }, mono_bold(cv.f(13.0)),
         if on { WHITE } else { al(WHITE, 0.45) });
}

fn note_name(n: i16) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    if n < 0 { "-".into() } else { format!("{}{}", N[(n % 12) as usize], n / 12 - 1) }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

// --- Randomizadores -------------------------------------------------------

fn rnd_int(st: &mut EditorState, lo: i32, hi: i32) -> i32 {
    lo + (st.rng.next01() * (hi - lo + 1) as f64).floor() as i32
}

fn randomize_fx(st: &mut EditorState, setter: &ParamSetter, p: &NgParams) {
    for s in &p.slots {
        let m = rnd_int(st, 0, 10);
        setter.begin_set_parameter(&s.module);
        setter.set_parameter(&s.module, m);
        setter.end_set_parameter(&s.module);
        set_once(setter, &s.a, st.rng.next01() as f32);
        set_once(setter, &s.b, st.rng.next01() as f32);
    }
    set_once(setter, &p.drive, st.rng.next01() as f32);
    set_once(setter, &p.mix, st.rng.next01() as f32);
    // Out: -12..0 dB para no saltar de volumen
    let out = -12.0 + st.rng.next01() as f32 * 12.0;
    set_once(setter, &p.out, p.out.preview_normalized(out));
    set_once(setter, &p.table_seed, st.rng.next01() as f32);
}

fn randomize_seq(st: &mut EditorState, setter: &ParamSetter, p: &NgParams) {
    let set_int = |p: &IntParam, v: i32| {
        setter.begin_set_parameter(p);
        setter.set_parameter(p, v);
        setter.end_set_parameter(p);
    };
    let gr = rnd_int(st, 7, 10);
    set_int(&p.gate_rate, gr);
    let gs = rnd_int(st, 5, 16);
    set_int(&p.gate_steps, gs);
    let gd = if st.rng.next01() < 0.7 { 0 } else { rnd_int(st, 1, 4) };
    set_int(&p.gate_dir, gd);
    set_once(setter, &p.gate_prob, p.gate_prob.preview_normalized(60.0 + st.rng.next01() as f32 * 40.0));
    let gm = if st.rng.next01() < 0.5 { 0.0 } else { st.rng.next01() as f32 * 25.0 };
    set_once(setter, &p.gate_mutate, p.gate_mutate.preview_normalized(gm));
    for m in &p.mods {
        let d = rnd_int(st, 1, 18);
        set_int(&m.dest, d);
        set_once(setter, &m.depth, st.rng.next01() as f32);
        let r = rnd_int(st, 0, 11);
        set_int(&m.rate, r);
        let s = rnd_int(st, 2, 16);
        set_int(&m.steps, s);
        let dir = rnd_int(st, 0, 4);
        set_int(&m.dir, dir);
        set_once(setter, &m.glide, st.rng.next01() as f32);
        set_once(setter, &m.prob, 0.5 + st.rng.next01() as f32 * 0.5);
        let mu = if st.rng.next01() < 0.6 { 0.0 } else { st.rng.next01() as f32 * 30.0 };
        set_once(setter, &m.mutate, m.mutate.preview_normalized(mu));
    }
}

fn edit_pattern(p: &NgParams, f: impl FnOnce(&mut Pattern)) {
    if let Ok(mut w) = p.pattern.write() {
        let mut pat: Pattern = (*w).into();
        f(&mut pat);
        *w = pat.into();
    }
}

// --- Dibujo completo ----------------------------------------------------------

fn draw(ui: &mut Ui, setter: &ParamSetter, p: &NgParams, voice: Option<&VoiceUiParams>, snap: &UiSnap, st: &mut EditorState) {
    // El GEN tiene una fila más (VOICE): lienzo más alto y carriles más abajo.
    let extra = if voice.is_some() { ROW } else { 0.0 };
    let dh = DH + extra;
    let fx_y = 246.0 + extra; // fila FEEDBACK · ENV FOLLOW · STEREO
    let lanes_y0 = fx_y + 56.0;
    let full = ui.max_rect();
    ui.painter().rect_filled(full, R0, VOID);
    let s = (full.width() / DW).min(full.height() / dh).max(0.3);
    let cv = Cv { o: Pos2::new(full.min.x + ((full.width() - DW * s) / 2.0).max(0.0), full.min.y), s };
    st.help = HELP_DEFAULT.to_string();
    let sn = &snap.s;

    let modules: [Module; 4] = std::array::from_fn(|i| Module::from_index(p.slots[i].module.value() as usize));

    // ===== CABECERA ==========================================================
    {
        let painter = ui.painter();
        painter.text(cv.r(15.0, 3.0, 0.0, 0.0).min, Align2::LEFT_TOP, "NOISEGEK", title_font(cv.f(34.0)), RED);
        painter.text(cv.r(12.0, 1.0, 0.0, 0.0).min, Align2::LEFT_TOP, "NOISEGEK", title_font(cv.f(34.0)), al(WHITE, 0.9));
    }
    if btn(ui, &cv, st, 196.0, 8.0, 88.0, 24.0, "RND FX", "RND FX: randomiza la cadena (módulos, A/B, drive, mix, out, table seed).").clicked() {
        randomize_fx(st, setter, p);
    }
    if btn(ui, &cv, st, 290.0, 8.0, 88.0, 24.0, "RND PAT", "RND PAT: nuevos valores para los pasos de los 4 carriles.").clicked() {
        let mut rng = st.rng;
        edit_pattern(p, |pat| (0..NUM_LANES).for_each(|l| pat.randomize(l, &mut rng)));
        st.rng = rng;
    }
    if btn(ui, &cv, st, 384.0, 8.0, 88.0, 24.0, "RND SEQ", "RND SEQ: randomiza la configuración de los carriles (destinos, velocidades, largos...).").clicked() {
        randomize_seq(st, setter, p);
    }
    if voice.is_some() {
        ui.painter().text(cv.r(480.0, 20.0, 0.0, 0.0).min, Align2::LEFT_CENTER, "GEN", title_font(cv.f(24.0)), RED);
        text(ui, cv.r(522.0, 1.0, 200.0, 38.0), Align2::LEFT_CENTER, "© Adolfo Rosas", mono(cv.f(13.0)), al(WHITE, 0.6));
    } else {
        text(ui, cv.r(480.0, 1.0, 220.0, 38.0), Align2::LEFT_CENTER, "© Adolfo Rosas (@ahrsml)", mono(cv.f(13.0)), al(WHITE, 0.6));
    }
    {
        let clk = if !sn.seq_running { "STOP" } else if sn.synced { "SYNC" } else { "FREE" };
        let status = format!("{clk}  {:.1} BPM", snap.tempo);
        let r = cv.r(700.0, 1.0, 288.0, 38.0);
        let galley_w = ui.painter().layout_no_wrap(status.clone(), mono_bold(cv.f(13.0)), WHITE).size().x / cv.s;
        ui.painter().rect_filled(cv.r(988.0 - galley_w - 22.0, 15.0, 9.0, 9.0), R0, if sn.seq_running { RED } else { SHADOW });
        text(ui, r, Align2::RIGHT_CENTER, &status, mono_bold(cv.f(13.0)), al(WHITE, if sn.seq_running { 1.0 } else { 0.6 }));
    }

    // ===== SOURCE ===========================================================
    let nt = p.noise_type.value() as usize;
    panel(ui, &cv, 8.0, 42.0, 190.0, 144.0, Some("SOURCE"), SHADOW);
    sel(ui, &cv, st, setter, &p.noise_type, (18.0, 66.0, 176.0, 24.0), Some("TYPE"), &names(&labels::NOISES),
        "TYPE: ruido interno que se suma a la entrada. Funciona en una pista vacía.");
    let nl = p.noise_level.value();
    bar(ui, &cv, st, setter, &p.noise_level, (18.0, 92.0, 176.0, 24.0), "LEVEL",
        if nl <= -60.0 { "off".into() } else { format!("{nl:.1} dB") },
        "LEVEL: volumen del ruido antes de la cadena (-60 dB = apagado).",
        Some(p.noise_level.preview_normalized(nl + sn.modsum[13] * 60.0)), false, false);
    bar(ui, &cv, st, setter, &p.noise_char, (18.0, 118.0, 176.0, 24.0), labels::noise_char_label(nt),
        labels::noise_char_text(nt, p.noise_char.value()), labels::noise_char_help(nt), None, false, false);
    for (i, line) in labels::noise_desc(nt).iter().enumerate() {
        text(ui, cv.r(14.0, 144.0 + i as f32 * 13.5, 184.0, 14.0), Align2::LEFT_CENTER, line, mono(cv.f(13.0)), al(WHITE, 0.55));
    }

    // ===== CHAIN ============================================================
    panel(ui, &cv, 204.0, 42.0, 560.0, 144.0, Some("CHAIN"), SHADOW);
    text(ui, cv.r(266.0, 43.0, 494.0, 20.0), Align2::LEFT_CENTER, "in > drive > 1 > 2 > 3 > 4 > filter > mix", mono(cv.f(13.0)), al(WHITE, 0.5));
    let mod_names = names(&labels::MODULES);
    for (si, sp) in p.slots.iter().enumerate() {
        let cx = 212.0 + si as f32 * 137.0;
        let m = modules[si];
        sel(ui, &cv, st, setter, &sp.module, (cx, 66.0, 130.0, 24.0), Some(&format!("{}", si + 1)), &mod_names,
            "Módulo del slot. Clic = menú, rueda = cambiar. La señal pasa por los slots de 1 a 4.");
        ui.painter().rect_filled(cv.r(cx + 18.0, 75.0, 6.0, 6.0), R0, if sn.slot_active[si] { RED } else { SHADOW });
        if m != Module::Off && !sn.slot_seq_on[si] {
            ui.painter().rect_filled(cv.r(cx + 28.0, 66.0, 102.0, 22.0), R0, al(VOID, 0.7));
            text(ui, cv.r(cx, 66.0, 112.0, 24.0), Align2::RIGHT_CENTER, "SEQ OFF", mono_bold(cv.f(13.0)), RED);
        }
        for (bi, (fp, row)) in [(&sp.a, 92.0), (&sp.b, 118.0)].into_iter().enumerate() {
            let b = bi == 1;
            let v = fp.value();
            bar(ui, &cv, st, setter, fp, (cx, row, 130.0, 24.0), labels::pab_label(m, b), labels::pab_text(m, b, v),
                labels::pab_help(m, b), Some((v + sn.modsum[1 + si * 2 + bi]).clamp(0.0, 1.0)), !m.uses(b), false);
        }
    }
    ui.painter().rect_filled(cv.r(212.0, 148.0, 541.0, 1.0), R0, SHADOW);
    let dv = p.drive.value();
    bar(ui, &cv, st, setter, &p.drive, (212.0, 154.0, 130.0, 24.0), "DRIVE", format!("{dv:.1} dB"),
        "DRIVE: ganancia de entrada a la cadena (-12..+36 dB). Más = más distorsión.",
        Some(p.drive.preview_normalized(dv + sn.modsum[9] * 48.0)), false, false);
    let mv = p.mix.value();
    bar(ui, &cv, st, setter, &p.mix, (349.0, 154.0, 130.0, 24.0), "MIX", format!("{:.0}%", mv * 100.0),
        "MIX: mezcla entre la entrada limpia (dry) y la cadena procesada (wet).",
        Some((mv + sn.modsum[10]).clamp(0.0, 1.0)), false, false);
    bar(ui, &cv, st, setter, &p.out, (486.0, 154.0, 130.0, 24.0), "OUT", format!("{:.1} dB", p.out.value()),
        "OUT: volumen de salida. Hay un limitador de seguridad a -0.3 dBFS.", None, false, false);
    bar(ui, &cv, st, setter, &p.table_seed, (623.0, 154.0, 130.0, 24.0), "T.SEED", format!("{}", p.table_seed.value()),
        "T.SEED: elige la curva aleatoria del módulo TABLE SHAPER.", None, false, false);

    // ===== FILTER ===========================================================
    panel(ui, &cv, 770.0, 42.0, 222.0, 144.0, Some("FILTER"), SHADOW);
    sel(ui, &cv, st, setter, &p.filter_mode, (780.0, 66.0, 204.0, 24.0), Some("MODE"), &names(&labels::FILTER_MODES),
        "MODE: filtro resonante (LOWPASS / BANDPASS / HIGHPASS) u OFF.");
    sel(ui, &cv, st, setter, &p.filter_pos, (780.0, 92.0, 204.0, 24.0), Some("POS"), &names(&labels::FILTER_POS),
        "POS: POST = filtra la distorsión (doma). PRE = filtra antes de distorsionar.");
    let cut = p.cutoff.value();
    let cut_mod = {
        let base = (cut / 20.0).ln() / 1000f32.ln();
        20.0 * 1000f32.powf((base + sn.modsum[11]).clamp(0.0, 1.0))
    };
    bar(ui, &cv, st, setter, &p.cutoff, (780.0, 118.0, 204.0, 24.0), "CUTOFF",
        if cut >= 1000.0 { format!("{:.2} kHz", cut / 1000.0) } else { format!("{cut:.0} Hz") },
        "CUTOFF: frecuencia de corte (20 Hz..20 kHz).", Some(p.cutoff.preview_normalized(cut_mod)), false, false);
    let rv = p.reso.value();
    bar(ui, &cv, st, setter, &p.reso, (780.0, 154.0, 104.0, 24.0), "RESO", format!("{:.0}%", rv * 100.0),
        "RESO: resonancia. Alta = pico silbante en el corte.", Some((rv + sn.modsum[12]).clamp(0.0, 1.0)), false, false);
    toggle(ui, &cv, st, setter, &p.scream, (888.0, 154.0, 96.0, 24.0), "SCREAM",
        "SCREAM: con RESO alta el filtro autooscila en el corte (silbido sostenido) y satura en vez de desbordarse.");

    // ===== SEQUENCER + GATE ================================================
    panel(ui, &cv, 8.0, 192.0, 984.0, ROW_H, Some("SEQUENCER"), SHADOW);
    sel(ui, &cv, st, setter, &p.clock, (18.0, 216.0, 140.0, 22.0), Some("CLOCK"), &names(&labels::CLOCKS),
        "CLOCK: AUTO = sigue al DAW al reproducir y corre libre en stop. HOST ONLY = se congela. FREE = siempre libre.");
    bar(ui, &cv, st, setter, &p.swing, (164.0, 216.0, 100.0, 22.0), "SWING", format!("{:.0}%", p.swing.value()),
        "SWING: retrasa los pasos impares (50% = recto, 66% = tresillo).", None, false, false);
    bar(ui, &cv, st, setter, &p.seq_seed, (270.0, 216.0, 100.0, 22.0), "SEED", format!("{}", p.seq_seed.value()),
        "SEED: semilla del azar (random, drunk, probabilidad). Mismo seed = mismo resultado.", None, false, false);
    ui.painter().text(cv.r(380.0, 227.0, 0.0, 0.0).min, Align2::LEFT_CENTER, "GATE", title_font(cv.f(18.0)), RED);
    bar(ui, &cv, st, setter, &p.gate_depth, (424.0, 216.0, 100.0, 22.0), "DEPTH", format!("{:.0}%", p.gate_depth.value() * 100.0),
        "GATE DEPTH: cuánto corta el gate (0% = sin efecto, 100% = silencio total entre golpes).", None, false, false);
    bar(ui, &cv, st, setter, &p.gate_len, (530.0, 216.0, 100.0, 22.0), "LEN", format!("{:.0}%", p.gate_len.value() * 100.0),
        "GATE LENGTH: cuánto del paso queda abierto. Bajo = golpes secos, 100% = legato.", None, false, false);
    let ga = p.gate_attack.value();
    bar(ui, &cv, st, setter, &p.gate_attack, (636.0, 216.0, 100.0, 22.0), "ATK",
        if ga >= 100.0 { format!("{ga:.0} ms") } else { format!("{ga:.1} ms") },
        "GATE ATTACK: tiempo de subida. Corto = ataque duro, largo = entra suave.", None, false, false);
    let gr = p.gate_release.value();
    bar(ui, &cv, st, setter, &p.gate_release, (742.0, 216.0, 100.0, 22.0), "REL",
        if gr >= 100.0 { format!("{gr:.0} ms") } else { format!("{gr:.1} ms") },
        "GATE RELEASE: tiempo de caída. Largo = colas, ideal para ambient.", None, false, false);
    sel(ui, &cv, st, setter, &p.gate_pos, (848.0, 216.0, 136.0, 22.0), Some("POS"), &names(&labels::GATE_POS),
        "GATE POS: PRE = corta lo que entra a la distorsión (colas cambian). POST = corta la salida.");

    // ===== VOICE (solo GEN) =================================================
    if let Some(v) = voice {
        panel(ui, &cv, 8.0, 246.0, 984.0, ROW_H, Some("VOICE"), RED);
        let note = sn.note;
        text(ui, cv.r(90.0, 247.0, 300.0, 22.0), Align2::LEFT_CENTER,
             &format!("nota {}", note_name(note)), mono(cv.f(13.0)), if note >= 0 { RED } else { al(WHITE, 0.45) });
        let env = sn.voice_env.clamp(0.0, 1.0);
        ui.painter().rect_filled(cv.r(230.0, 252.0, 150.0, 10.0), R0, VOID);
        ui.painter().rect_filled(cv.r(230.0, 252.0, 150.0 * env, 10.0), R0, RED);
        let msf = |x: f32| if x >= 100.0 { format!("{x:.0} ms") } else { format!("{x:.1} ms") };
        let y = 270.0;
        bar(ui, &cv, st, setter, &v.attack, (18.0, y, 100.0, 22.0), "ATK", msf(v.attack.value()),
            "ATTACK: subida de cada nota. Corto = golpe, largo = entra suave.", None, false, false);
        bar(ui, &cv, st, setter, &v.decay, (124.0, y, 100.0, 22.0), "DEC", msf(v.decay.value()),
            "DECAY: tiempo en bajar del pico al nivel de SUSTAIN.", None, false, false);
        bar(ui, &cv, st, setter, &v.sustain, (230.0, y, 100.0, 22.0), "SUS", format!("{:.0}%", v.sustain.value() * 100.0),
            "SUSTAIN: nivel mientras mantienes la nota. 0% = solo golpes.", None, false, false);
        bar(ui, &cv, st, setter, &v.release, (336.0, y, 100.0, 22.0), "REL", msf(v.release.value()),
            "RELEASE: cola después de soltar la nota. Largo = ambient.", None, false, false);
        bar(ui, &cv, st, setter, &v.velocity, (442.0, y, 100.0, 22.0), "VEL", format!("{:.0}%", v.velocity.value() * 100.0),
            "VELOCITY: cuánto afecta la fuerza de la nota al volumen (0% = todas iguales).", None, false, false);
        bar(ui, &cv, st, setter, &v.track, (548.0, y, 100.0, 22.0), "TRACK", format!("{:.0}%", v.track.value() * 100.0),
            "KEY TRACK: el filtro sigue la nota (100% = una octava por octava). Con BANDPASS y RESO alta = ruido afinado.", None, false, false);
        toggle(ui, &cv, st, setter, &v.pitched, (654.0, y, 100.0, 22.0), "PITCH",
            "PITCH: el LFSR (modo tonal 8-bit) y el RING MOD se afinan a la nota del piano roll.");
        toggle(ui, &cv, st, setter, &v.retrig, (760.0, y, 100.0, 22.0), "RETRIG",
            "RETRIG: cada nota reinicia el secuenciador desde el paso 1.");
        sel(ui, &cv, st, setter, &v.env_pos, (866.0, y, 118.0, 22.0), Some("ENV"), &names(&labels::GATE_POS),
            "ENV: PRE = la envolvente entra a la distorsión (cambia el timbre). POST = solo volumen final.");
    }

    // ===== FEEDBACK · ENV FOLLOW · STEREO ==================================
    let fy = fx_y + 24.0;
    panel(ui, &cv, 8.0, fx_y, 424.0, ROW_H, Some("FEEDBACK"), SHADOW);
    let fa = p.fb_amount.value();
    bar(ui, &cv, st, setter, &p.fb_amount, (18.0, fy, 96.0, 24.0), "AMT", format!("{:.0}%", fa * 100.0),
        "FB AMOUNT: cuánto de la salida de la cadena vuelve al slot 1. Sobre 100% el bucle se desboca y lo contiene la saturación.",
        Some(p.fb_amount.preview_normalized((fa + 1.5 * sn.modsum[19]).clamp(0.0, 1.5))), false, false);
    let ft = p.fb_time.value();
    let ft_mod = {
        let base = (ft / 0.05).ln() / 1000f32.ln();
        0.05 * 1000f32.powf((base + sn.modsum[20]).clamp(0.0, 1.0))
    };
    bar(ui, &cv, st, setter, &p.fb_time, (118.0, fy, 96.0, 24.0), "TIME",
        if ft >= 10.0 { format!("{ft:.1}") } else { format!("{ft:.2}") },
        "FB TIME: retardo del bucle (0.05..50 ms). Corto = resonancia metálica; largo = ecos. Modulado, cambia la altura.",
        Some(p.fb_time.preview_normalized(ft_mod)), false, false);
    let fto = p.fb_tone.value();
    bar(ui, &cv, st, setter, &p.fb_tone, (218.0, fy, 96.0, 24.0), "TONE",
        if fto >= 1000.0 { format!("{:.1}k", fto / 1000.0) } else { format!("{fto:.0}") },
        "FB TONE: corte del filtro de un polo dentro del bucle. Bajo = repeticiones oscuras.", None, false, false);
    sel(ui, &cv, st, setter, &p.fb_tap, (318.0, fy, 108.0, 24.0), None, &names(&labels::FB_TAPS),
        "FB TAP: el bucle toma la señal a la salida de los slots (PRE FILTER) o después del filtro (POST FILTER).");

    panel(ui, &cv, 438.0, fx_y, 420.0, ROW_H, Some("ENV FOLLOW"), SHADOW);
    let msf = |x: f32| if x >= 100.0 { format!("{x:.0}") } else { format!("{x:.1}") };
    bar(ui, &cv, st, setter, &p.env_attack, (448.0, fy, 88.0, 24.0), "ATK", msf(p.env_attack.value()),
        "ENV ATTACK (ms): qué tan rápido sube el seguidor de envolvente.", None, false, false);
    bar(ui, &cv, st, setter, &p.env_release, (540.0, fy, 88.0, 24.0), "REL", msf(p.env_release.value()),
        "ENV RELEASE (ms): qué tan rápido baja el seguidor.", None, false, false);
    bar(ui, &cv, st, setter, &p.env_gain, (632.0, fy, 88.0, 24.0), "GAIN", format!("{:+.0}", p.env_gain.value()),
        "ENV GAIN (dB): ganancia antes del detector. La fuente ENV vale entre 0 y 1.", None, false, false);
    let taps = if voice.is_some() { names(&labels::ENV_TAPS_GEN) } else { names(&labels::ENV_TAPS) };
    sel(ui, &cv, st, setter, &p.env_tap, (724.0, fy, 108.0, 24.0), None, &taps,
        if voice.is_some() {
            "ENV TAP: VOICE = sigue la envolvente de cada nota. PRE-FILTER = sigue la salida de los slots (automodulación)."
        } else {
            "ENV TAP: INPUT = sigue la entrada de la pista. PRE-FILTER = sigue la salida de los slots (automodulación)."
        });
    let ev = sn.env_follow.clamp(0.0, 1.0);
    ui.painter().rect_filled(cv.r(838.0, fy, 12.0, 22.0), R0, VOID);
    ui.painter().rect_filled(cv.r(838.0, fy + 22.0 * (1.0 - ev), 12.0, 22.0 * ev), R0, RED);

    panel(ui, &cv, 864.0, fx_y, 128.0, ROW_H, Some("STEREO"), SHADOW);
    let spv = p.spread.value();
    bar(ui, &cv, st, setter, &p.spread, (874.0, fy, 110.0, 24.0), "SPREAD", format!("{:.0}%", spv * 100.0),
        "SPREAD: diferencias entre L y R (fase del RING MOD, DOWNSAMPLE desfasado, FB TIME de R más largo).",
        Some((spv + sn.modsum[21]).clamp(0.0, 1.0)), false, false);

    // ===== CARRILES ========================================================
    let (gx, gw, lht) = (456.0f32, 528.0f32, 70.0f32);
    let cw = gw / NUM_STEPS as f32;
    let pattern: Pattern = p.pattern.read().map(|g| (*g).into()).unwrap_or_default();
    let rate_names = names(&noisegek_dsp::seq::RATE_NAMES);
    let dir_names = names(&labels::DIRS);
    let dest_names: Vec<String> = (0..noisegek_dsp::NUM_DEST).map(|i| labels::dest_name(i, &modules)).collect();
    let pct = |v: f32| format!("{v:.0}%");

    for l in 0..NUM_LANES {
        let ly = lanes_y0 + l as f32 * LANE_STEP;
        panel(ui, &cv, 8.0, ly, 984.0, lht, None, if l == 0 { RED } else { SHADOW });
        let title = if l == 0 { "GATE".to_string() } else { format!("MOD {l}") };
        ui.painter().text(cv.r(18.0, ly, 0.0, 0.0).min, Align2::LEFT_TOP, title, title_font(cv.f(18.0)),
            if l == 0 { RED } else { WHITE });

        let len = if l == 0 { p.gate_steps.value() } else { p.mods[l - 1].steps.value() } as u8;
        if l > 0 {
            sel(ui, &cv, st, setter, &p.mod_src[l - 1].src, (120.0, ly + 3.0, 176.0, 18.0), Some("SRC"),
                &names(&labels::LANE_SOURCES),
                "SOURCE: qué mueve este carril. SEQ = los pasos; ENV = el seguidor de envolvente; SEQ×ENV = el producto.");
        }

        // Herramientas del carril
        if btn(ui, &cv, st, 304.0, ly + 3.0, 20.0, 18.0, "<", "Rotar el patrón un paso a la izquierda.").clicked() {
            edit_pattern(p, |pat| pat.rotate(l, -1, len));
        }
        if btn(ui, &cv, st, 328.0, ly + 3.0, 20.0, 18.0, ">", "Rotar el patrón un paso a la derecha.").clicked() {
            edit_pattern(p, |pat| pat.rotate(l, 1, len));
        }
        if btn(ui, &cv, st, 352.0, ly + 3.0, 40.0, 18.0, "RND", "Valores aleatorios para este carril.").clicked() {
            let mut rng = st.rng;
            edit_pattern(p, |pat| pat.randomize(l, &mut rng));
            st.rng = rng;
        }
        let k = pattern.euc_k[l];
        let r = btn(ui, &cv, st, 396.0, ly + 3.0, 52.0, 18.0, &format!("EUC{k}"),
            "Euclidiano: reparte N golpes lo más parejo posible. Clic izq. = +1 golpe, der. = -1.");
        if r.clicked() {
            edit_pattern(p, |pat| pat.euclid(l, if k + 1 > len { 0 } else { k + 1 }, len));
        }
        if r.secondary_clicked() {
            edit_pattern(p, |pat| pat.euclid(l, if k < 1 { len } else { k - 1 }, len));
        }

        let (y1, y2) = (ly + 24.0, ly + 46.0);
        let help_steps = "STEPS: largo del patrón (1..16). Largos distintos entre carriles = polimetría.";
        let help_dir = "DIRECTION: FWD, BWD, PINGPONG, RANDOM o DRUNK (paso a paso al azar).";
        let help_mut = "MUTATE: probabilidad de que un paso se reescriba al sonar (el patrón evoluciona).";
        if l == 0 {
            sel(ui, &cv, st, setter, &p.gate_rate, (18.0, y1, 100.0, 20.0), None, &rate_names, "RATE: duración de cada paso del gate.");
            bar(ui, &cv, st, setter, &p.gate_steps, (122.0, y1, 100.0, 20.0), "STP", format!("{}", p.gate_steps.value()), help_steps, None, false, false);
            bar(ui, &cv, st, setter, &p.gate_prob, (226.0, y1, 100.0, 20.0), "PRB", pct(p.gate_prob.value()),
                "PROB: probabilidad de que cada golpe suene.", None, false, false);
            bar(ui, &cv, st, setter, &p.gate_mutate, (330.0, y1, 100.0, 20.0), "MUT", pct(p.gate_mutate.value()), help_mut, None, false, false);
            sel(ui, &cv, st, setter, &p.gate_dir, (18.0, y2, 100.0, 20.0), None, &dir_names, help_dir);
            text(ui, cv.r(122.0, y2, 310.0, 20.0), Align2::LEFT_CENTER, "depth/len/env: SEQUENCER", mono(cv.f(13.0)), al(WHITE, 0.5));
        } else {
            let m = &p.mods[l - 1];
            sel(ui, &cv, st, setter, &m.dest, (18.0, y1, 100.0, 20.0), None, &dest_names,
                "DESTINO: qué parámetro mueve este carril. S1..S4 ON/OFF prende y apaga slots.");
            let dep = m.depth.value();
            bar(ui, &cv, st, setter, &m.depth, (122.0, y1, 100.0, 20.0), "DEP", format!("{dep:+.0}%"),
                "DEPTH: cuánto mueve el carril al destino. Negativo = invierte.", None, false, true);
            sel(ui, &cv, st, setter, &m.rate, (226.0, y1, 100.0, 20.0), None, &rate_names, "RATE: duración de cada paso (8 compases..1/64).");
            bar(ui, &cv, st, setter, &m.steps, (330.0, y1, 100.0, 20.0), "STP", format!("{}", m.steps.value()), help_steps, None, false, false);
            sel(ui, &cv, st, setter, &m.dir, (18.0, y2, 100.0, 20.0), None, &dir_names, help_dir);
            bar(ui, &cv, st, setter, &m.glide, (122.0, y2, 100.0, 20.0), "GLD", pct(m.glide.value()),
                "GLIDE: deslizamiento entre pasos. 0 = escalones, 100% = curvas suaves (ambient).", None, false, false);
            bar(ui, &cv, st, setter, &m.prob, (226.0, y2, 100.0, 20.0), "PRB", pct(m.prob.value()),
                "PROB: probabilidad de que cada paso cambie el valor (si no, se mantiene).", None, false, false);
            bar(ui, &cv, st, setter, &m.mutate, (330.0, y2, 100.0, 20.0), "MUT", pct(m.mutate.value()), help_mut, None, false, false);
        }

        // Grilla de pasos
        let grid = cv.r(gx, ly + 3.0, gw, lht - 6.0);
        let resp = ui.interact(grid, Id::new(("grid", l)), Sense::click_and_drag());
        if resp.hovered() {
            st.help = "PASOS: clic izq. arrastrado = dibujar · clic der. arrastrado = borrar.".into();
        }
        let painter = ui.painter();
        for i in 0..NUM_STEPS {
            let x = gx + i as f32 * cw;
            let playing = sn.seq_running && i == sn.lane_idx[l] as usize;
            let in_len = i < len as usize;
            painter.rect_filled(cv.r(x + 1.0, ly + 3.0, cw - 2.0, lht - 6.0), 0.0,
                if playing { al(SHADOW, 0.6) } else { al(VOID, if in_len { 1.0 } else { 0.45 }) });
            let bh = pattern.steps[l][i] * (lht - 10.0);
            let col = if !in_len { al(SHADOW, 0.8) } else if playing && sn.lane_fired[l] { RED } else { al(WHITE, 0.7) };
            painter.rect_filled(cv.r(x + 3.0, ly + lht - 5.0 - bh, cw - 6.0, bh), R0, col);
            if i % 4 == 0 {
                painter.rect_filled(cv.r(x + 1.0, ly + 3.0, 1.0 / cv.s, lht - 6.0), R0, SHADOW);
            }
        }
        if l > 0 && p.mods[l - 1].dest.value() > 0 {
            let x = gx + sn.lane_idx[l] as f32 * cw;
            painter.rect_filled(cv.r(x + 1.0, ly + lht - 5.0 - sn.lane_val[l] * (lht - 10.0), cw - 2.0, 2.0), R0, WHITE);
        }
        if l == 0 && p.gate_depth.value() > 0.0 {
            let e = sn.gate_env.clamp(0.0, 1.0);
            painter.rect_filled(cv.r(gx + gw + 1.0, ly + 3.0 + (1.0 - e) * (lht - 6.0), 4.0, e * (lht - 6.0)), R0, RED);
        }
        // Edición: dibujar (izq.) o borrar (der.), con relleno entre posiciones
        let (primary, secondary, pos) = ui.input(|i| (i.pointer.primary_down(), i.pointer.secondary_down(), i.pointer.interact_pos()));
        if resp.is_pointer_button_down_on() && (primary || secondary) {
            if let Some(pos) = pos {
                let i = (((pos.x - grid.min.x) / (cw * cv.s)).floor() as i32).clamp(0, NUM_STEPS as i32 - 1) as usize;
                let v = if primary {
                    (1.0 - (pos.y - cv.r(0.0, ly + 5.0, 0.0, 0.0).min.y) / ((lht - 10.0) * cv.s)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (a, b) = match st.step_last {
                    Some(j) => (i.min(j), i.max(j)),
                    None => (i, i),
                };
                edit_pattern(p, |pat| (a..=b).for_each(|j| pat.steps[l][j] = v));
                st.step_last = Some(i);
            }
        }
    }
    if !ui.input(|i| i.pointer.any_down()) {
        st.step_last = None;
    }

    // ===== BARRA DE AYUDA ==================================================
    let hy = dh - 30.0;
    let hb = cv.r(8.0, hy, 984.0, 26.0);
    ui.painter().rect_filled(hb, R0, IRON);
    text(ui, cv.r(8.0, hy, 24.0, 26.0), Align2::LEFT_CENTER, ">", mono(cv.f(13.0)), RED);
    let help = st.help.clone();
    text(ui, cv.r(26.0, hy, 962.0, 26.0), Align2::LEFT_CENTER, &help, mono(cv.f(13.0)), al(WHITE, 0.9));
}
