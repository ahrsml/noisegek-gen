# -*- coding: utf-8 -*-
"""
NOISEGEK · por @ahrsml
Genera las ESPECIFICACIONES TÉCNICAS completas (documentación de ingeniería) en
la carpeta raíz del proyecto:
    ../../NOISEGEK - Especificacion tecnica.html
    ../../NOISEGEK GEN - Especificacion tecnica.html

A diferencia de las carpetas de distribución, estos documentos SÍ describen la
implementación: matemática, constantes, algoritmos, formatos de estado,
concurrencia, interfaz, JSFX, compilación y verificación. Las partes
susceptibles de desactualizarse (sliders del JSFX, geometría de la interfaz,
lista de tests, código fuente del DSP) se extraen del código al generar.
"""
import html
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from generar_docs import CSS, AUTOR, FECHA, VERSION, PARAMS_CORE, PARAMS_VOICE, MODULOS  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
VST = os.path.dirname(HERE)
ROOT = VST
BS = chr(92)

# El "todos los derechos reservados" cubre el texto, no el software.
COPY_SPEC = ("Texto de la especificación © 2026 " + AUTOR + ". Todos los derechos reservados. "
             "El código fuente reproducido en el Apéndice A se rige por la licencia del software "
             "correspondiente (GPL-3.0-or-later), no por la del texto.")

CSS_EXTRA = r"""
.part { font-family:Impact,"Anton",sans-serif; font-size:44px; color:var(--red); margin:96px 0 0; letter-spacing:.03em; }
pre.src { background:#111; border-left:3px solid var(--shadow); padding:14px 16px; overflow-x:auto; font-size:13px;
          line-height:1.45; font-family:"Courier New",monospace; white-space:pre; }
pre.code { background:var(--iron); border-left:3px solid var(--red); padding:14px 18px; overflow-x:auto; font-size:15px;
           line-height:1.5; font-family:"Courier New",monospace; font-weight:bold; white-space:pre; }
.srcname { font-family:"Courier New",monospace; font-weight:bold; color:var(--red); margin-top:28px; }
td code { white-space:nowrap; }
"""

_eq = [0]


def eq(t):
    _eq[0] += 1
    return f'<div class="eq"><span class="n">({_eq[0]})</span>{html.escape(t.strip(chr(10)))}</div>'


def code(t):
    return f'<pre class="code">{html.escape(t.strip(chr(10)))}</pre>'


def fig(t, cap):
    return f'<div class="fig">{html.escape(t.strip(chr(10)))}</div><p class="cap">{cap}</p>'


def table(headers, rows, ncols=()):
    h = "".join(f'<th{" class=n" if i in ncols else ""}>{c}</th>' for i, c in enumerate(headers))
    b = "".join("<tr>" + "".join(f'<td{" class=n" if i in ncols else ""}>{c}</td>' for i, c in enumerate(r)) + "</tr>" for r in rows)
    return f"<table><tr>{h}</tr>{b}</table>"


def esc(s):
    return html.escape(s)


def read(*parts):
    with open(os.path.join(*parts), encoding="utf-8") as f:
        return f.read()


# =============================================================================
# Extracción automática desde el código
# =============================================================================

def jsfx_sliders():
    path = os.path.join(ROOT, "jsfx", "NOISEGEK.jsfx")
    if not os.path.isfile(path):
        return []
    rows = []
    for line in read(path).splitlines():
        m = re.match(r"^slider(\d+):([^<]*)<([^>]*)>-?(.*)$", line)
        if m:
            n, default, rng, name = m.groups()
            rows.append([n, esc(name.strip()), esc(default), "<code>" + esc(rng) + "</code>"])
    return rows


def ui_layout(gen):
    """Una fila por llamada a widget en editor.rs (análisis línea a línea)."""
    src = read(VST, "noisegek-core", "src", "editor.rs")
    nombres = {"bar": "barra", "sel": "selector", "toggle": "interruptor", "btn": "botón"}
    rows = []
    for m in re.finditer(r'\b(bar|sel|toggle|btn)\(ui, &cv, st,([^\n]*)', src):
        kind, rest = m.group(1), m.group(2).strip()
        if kind == "btn":
            parts = [p.strip() for p in rest.split(",")]
            if len(parts) < 5:
                continue
            x, y, w, h, lab = parts[:5]
            rows.append([nombres[kind], esc(lab), esc(x), esc(y), esc(w), esc(h)])
            continue
        mm = re.match(r'setter, (&?[^,]+), \(([^)]*)\),\s*(.*)', rest)
        if not mm:
            continue
        target, geom, tail = mm.groups()
        if target.startswith("&v.") and not gen:
            continue  # fila VOICE: solo existe en el instrumento
        parts = [p.strip() for p in geom.split(",")]
        if len(parts) != 4:
            continue
        lab = tail.split(",")[0].strip()
        etiqueta = esc(lab) if lab.startswith(('"', "Some(")) else "<i>dinámica</i>"
        rows.append([nombres[kind], "<code>" + esc(target.lstrip("&")) + "</code> · " + etiqueta] + [esc(p) for p in parts])
    return rows


def test_names():
    names = []
    fuentes = [("noisegek-dsp", f) for f in ("rng.rs", "modules.rs", "noise.rs", "filter.rs", "seq.rs", "engine.rs")]
    fuentes += [("noisegek-core", "params.rs")]
    for crate, f in fuentes:
        src = read(VST, crate, "src", f)
        for m in re.finditer(r"#\[test\]\s*fn (\w+)", src):
            names.append([crate + "/" + f, "<code>" + m.group(1) + "</code>"])
    return names


def dsp_source_appendix():
    out = ""
    for f in ("lib.rs", "rng.rs", "noise.rs", "modules.rs", "filter.rs", "seq.rs", "engine.rs"):
        src = read(VST, "noisegek-dsp", "src", f)
        out += f'<div class="srcname">noisegek-dsp/src/{f}</div><pre class="src">{esc(src)}</pre>'
    return out


def toolchain_info():
    info = {"nih": "de421011f41a6d10fc8c7a6084e4f4dee0143683", "egui": "0.31.1", "rustc": "1.99.0"}
    lock = os.path.join(VST, "Cargo.lock")
    if os.path.isfile(lock):
        t = read(lock)
        m = re.search(r'name = "egui"\nversion = "([^"]+)"', t)
        if m:
            info["egui"] = m.group(1)
    return info


# =============================================================================
# Secciones
# =============================================================================

def s_ident(gen):
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    rows = [
        ["Producto", nombre],
        ["Clase", "instrumento virtual monofónico (generador de ruido tocable por MIDI)" if gen else "procesador de efectos (ruido, distorsión, filtro, secuenciador)"],
        ["Versión", VERSION],
        ["Autor y titular de los derechos", AUTOR],
        ["Derechos", "El texto de esta especificación es © 2026 " + AUTOR + ", todos los derechos reservados. El código fuente "
                     "reproducido en el Apéndice A se rige por la licencia del software correspondiente (crate "
                     "<code>noisegek-dsp</code>: GPL-3.0-or-later) y no por la del texto. Binarios VST3/CLAP: GPL-3.0-or-later, "
                     "porque enlazan el crate <code>vst3-sys</code> (licencia GPLv3), que NIH-plug incorpora con su "
                     "característica <code>vst3</code>; el resto de sus dependencias tiene licencias permisivas"
                     + ("" if gen else ". JSFX: todos los derechos reservados")],
        ["Fecha del documento", FECHA],
        ["Formatos", "VST3, CLAP" if gen else "VST3, CLAP, JSFX (REAPER)"],
        ["CLAP ID", "<code>com.ahrsml.noisegek-gen</code>" if gen else "<code>com.ahrsml.noisegek</code>"],
        ["VST3 class ID (16 bytes ASCII)", "<code>NOISEGEKGENahrsm</code>" if gen else "<code>NOISEGEKahrsml01</code>"],
        ["Subcategorías VST3", "<code>Instrument</code>, <code>Synth</code>" if gen else "<code>Fx</code>, <code>Distortion</code>, <code>Filter</code>"],
        ["Características CLAP", "<code>instrument</code>, <code>synthesizer</code>, <code>stereo</code>" if gen else "<code>audio-effect</code>, <code>stereo</code>, <code>distortion</code>, <code>filter</code>"],
        ["Fabricante (vendor)", "<code>@ahrsml</code>"],
    ]
    return f"""
<h2 id="i1">1. Identificación</h2>
{table(["Campo", "Valor"], rows, ncols=(0,))}
<p>Este documento es la especificación de ingeniería de {nombre}. Es normativo respecto del comportamiento: una
implementación que respete las ecuaciones, constantes, órdenes de evaluación y formatos aquí descritos reproduce la
salida del original muestra a muestra en las condiciones de la sección 13.4, salvo el único elemento no determinista
declarado allí: la mutación. El
Apéndice A contiene el código fuente completo del núcleo de señal, que prevalece ante cualquier discrepancia.</p>
"""


def s_arquitectura(gen):
    jsfx = "" if gen else "\n  NOISEGEK.jsfx      implementación EEL2 equivalente para REAPER (Parte IV)"
    diag = f"""
  noisegek-dsp       núcleo de señal: Rust puro, sin dependencias, sin E/S
      │  Params (valores planos), Transport, notas → process_frame(l, r) → (l, r)
  noisegek-core      parámetros del host, estado persistente, interfaz (egui),
      │              estructuras Noisegek y NoisegekGen (impl Plugin)
  {"noisegek-gen " if gen else "noisegek     "}      exporta {"NoisegekGen" if gen else "Noisegek"} como VST3 y CLAP (cdylib){jsfx}
"""
    hilos = fig("""
  HILO DE AUDIO (host)                         HILO DE INTERFAZ (host / baseview)
  ─────────────────────                        ───────────────────────────────────
  process():                                   update() de egui, ~60 Hz:
    pattern.try_read()  → copia al motor          lee NgParams (valores atómicos)
    params → Params (lectura atómica)             pattern.read()/write() al editar pasos
    begin_block(); process_frame() × N            snap.lock() → copia de Snapshot
    si mutación: pattern.try_write()              ParamSetter: begin/set/end (host)
    si editor abierto: snap.try_lock() ← Snapshot
""", "Figura 2. Modelo de concurrencia. El hilo de audio nunca espera un bloqueo.")
    return f"""
<h2 id="i2">2. Arquitectura del sistema</h2>
{fig(diag, "Figura 1. Componentes.")}
<p>El núcleo <code>noisegek-dsp</code> no conoce el formato de plugin: recibe una estructura <code>Params</code> con los
valores físicos de todos los controles, un <code>Transport</code> y, en el instrumento, eventos de nota. Toda la
aritmética de señal es de 64 bits; la frontera con el host es de 32 bits.</p>
<h3>2.1 Concurrencia</h3>
{hilos}
<p>Los parámetros de host son atómicos (provistos por NIH-plug). El patrón se comparte mediante
<code>Arc&lt;RwLock&lt;PatternData&gt;&gt;</code>: el audio usa exclusivamente <code>try_read</code>/<code>try_write</code>
y, si el bloqueo está ocupado, conserva su copia del bloque anterior. El estado visible (<code>Snapshot</code>) se publica en
un <code>Mutex</code> con <code>try_lock</code>, solo si el editor está abierto. <code>process()</code> no asigna
memoria en el heap. En Rust, la regeneración de las tablas del waveshaper usa un búfer en la pila (1025 × f64); el JSFX
no asigna memoria en <code>@sample</code>. El delay del bucle de realimentación (20.1, 2 × 32768 × f64) se reserva al
crear el motor (<code>Engine::new</code>, llamado desde <code>initialize()</code>) para la frecuencia de muestreo máxima
admitida, 384 kHz; <code>reset()</code> reutiliza ese búfer y las tablas, sin reservar memoria.</p>
"""


def s_numeros():
    return f"""
<h2 id="i3">3. Convenciones numéricas</h2>
{table(["Elemento", "Definición"], [
    ["Tipo interno", "<code>f64</code> (IEEE-754 binary64). E/S del plugin: <code>f32</code>."],
    ["<code>floor</code>, <code>round</code>", "<code>round(x) = floor(x + 0.5)</code> (no se usa redondeo bancario)"],
    ["<code>a mod m</code>", "módulo euclidiano: resultado en [0, m) también para a negativo (<code>rem_euclid</code>)"],
    ["<code>clamp(v, a, b)</code>", "<code>min(max(v, a), b)</code>"],
    ["<code>k(τ)</code>", "<code>1 − exp(−1 / (τ · fs))</code>, τ en segundos"],
    ["<code>dB(g)</code>", "<code>10^(g/20)</code>"],
    ["<code>tanh</code>", "argumento acotado a [−20, 20]"],
    ["<code>sgn</code>", "−1, 0 o 1 (sgn(0) = 0)"],
    ["Enteros sin signo de 32 bits", "aritmética modular 2^32 (wrapping)"],
], ncols=(0,))}
"""


def s_rng():
    return f"""
<h2 id="i4">4. Generadores pseudoaleatorios</h2>
<h3>4.1 xorshift32 (Marsaglia)</h3>
{eq('''s ← s ⊕ (s ≪ 13);   s ← s ⊕ (s ≫ 17);   s ← s ⊕ (s ≪ 5)      (u32)
u = s / 2^32 ∈ [0, 1)''')}
<p>Valores de control: partiendo de s = 1 se obtienen 270369 y 67634689. El estado 0 es absorbente y se sustituye por 1.</p>
<h3>4.2 Hash determinista h(n, l, sal, seed)</h3>
{eq('''v = 73856093·n + 19349663·l + 83492791·sal + 2654435·seed + 1     (aritmética entera exacta, n ∈ ℤ)
s = v mod 2^32;   si s = 0 entonces s = 1
s ← xorshift32(s) tres veces;   h = s / 2^32''')}
<p>Nota: el coeficiente de <code>seed</code> es 2654435, no la constante multiplicativa de Knuth (2654435761) de
la que deriva. Es intencional y normativo: se conserva porque cambiarlo alteraría los patrones RANDOM y DRUNK y la
probabilidad de disparo de todos los proyectos y presets existentes. Al ser impar, cada SEED produce un desplazamiento
distinto módulo 2^32.</p>
{table(["Sal", "Uso"], [["1", "dirección RANDOM"], ["2", "dirección DRUNK"], ["3", "probabilidad de disparo"]], ncols=(0,))}
<h3>4.3 LCG del ruido</h3>
{eq('''s ← (1664525·s + 1013904223) mod 2^32;    u = s / 2^32;    w = 2u − 1''')}
<h3>4.4 Semillas</h3>
{table(["Generador", "Semilla inicial"], [
    ["LCG canal L / R", "22222 / 777777"],
    ["LFSR canal L / R", "1 / 0x5A5A"],
    ["xorshift del TRIGGER S/H, slot s, canal c", "12345 + 7919·(2s + c)"],
    ["xorshift de mutación y botones", "milisegundos desde la época Unix, truncados a u32 (no determinista)"],
    ["Tablas del TABLE SHAPER", "T.SEED + 1, con 16 iteraciones de calentamiento"],
], ncols=(0,))}
<p>Las semillas del LCG, del LFSR y del xorshift del TRIGGER S/H se restablecen en cada resincronización (9.7).</p>
"""


def s_ruido(gen):
    return f"""
<h2 id="i5">5. Fuente de ruido</h2>
<p>Coeficientes recalculados al inicio de cada bloque a partir de CHARACTER = c ∈ [0, 1]:</p>
{eq('''λ = 0.0002 + 0.05·(1 − c)²          g = √(0.6·λ)                       (BROWN)
p = (2 + 20000·c³) / fs                                                (DUST)
Δ = min( 50 · 2^(9.5·c) / fs , 4 )                                     (LFSR, pasos por muestra)''')}
{table(["Tipo", "Muestra del canal (estado por canal)"], [
    ["WHITE", "<code>w</code> (sección 4.3); canal R: <code>R = L + (w<sub>R</sub> − L)·c</code>"],
    ["PINK", "Kellet: <code>b0 = .99886·b0 + .0555179w; b1 = .99332·b1 + .0750759w; b2 = .96900·b2 + .1538520w; "
             "b3 = .86650·b3 + .3104856w; b4 = .55000·b4 + .5329522w; b5 = −.7616·b5 − .0168980w; "
             "y = .11·(b0+b1+b2+b3+b4+b5+b6 + .5362w); b6 = .115926w</code>; R mezclado con c como WHITE"],
    ["BROWN", "<code>y = (1 − λ)·y + g·w</code> (canales independientes)"],
    ["DUST", "<code>y ← .45·y</code>; si <code>u &lt; p</code> entonces <code>y ← w</code> (dos sorteos del mismo LCG, en ese orden)"],
    ["LFSR", "<code>φ ← φ + Δ</code>; mientras <code>φ ≥ 1</code>: <code>φ ← φ − 1</code>, <code>f = b0 ⊕ b<sub>t</sub></code>, "
             "<code>r ← (r ≫ 1) | (f ≪ 14)</code>; salida <code>+0.7</code> si <code>b0 = 1</code>, si no <code>−0.7</code>. "
             "<code>t = 1</code> (período 32767)" + ("; <code>t = 6</code> en modo PITCH (sección 12.5)" if gen else "")],
], ncols=(0,))}
{eq('''nivel = 0                    si LEVEL' ≤ −60 dB o TYPE = OFF
nivel = dB(min(LEVEL', 0))   en otro caso,   LEVEL' = LEVEL + 60·m_noise   (m: modulación, sección 11)
ruido_canal = muestra_canal · nivel_suavizado''')}
"""


def s_modulos():
    return f"""
<h2 id="i6">6. Módulos no lineales</h2>
<p>Cada slot s ∈ {{1..4}} aplica <code>y = M(x; a, b)</code>, con <code>(a, b) = map(M, A', B')</code>, donde A' y B'
son los valores crudos ya modulados (sección 11). El mapeo se evalúa a tasa de control y el resultado se suaviza.</p>
{table(["Módulo", "Transferencia", "a = map(A)", "b = map(B)"], MODULOS, ncols=(0,))}
<h3>6.1 Detalles de implementación</h3>
{table(["Módulo", "Detalle normativo"], [
    ["SOFT CLIP", "<code>b = 1/tanh(d)</code> se calcula en el mapeo y se suaviza junto con <code>d</code>"],
    ["FOLDBACK", "bucle: mientras <code>(x &gt; t ∨ x &lt; −t) ∧ n &lt; N</code>: <code>x ← 2t − x</code> si <code>x &gt; t</code>, si no <code>x ← −2t − x</code>; <code>n ← n + 1</code>. N = floor(b + 0.5) con b = 1 + 15B"],
    ["BITCRUSH", "<code>a = 1 / 2^(bits − 1)</code> es el paso de cuantización; 1 bit ⇒ salidas {−1, 0, 1}"],
    ["DOWNSAMPLE", "estado (contador c, retenido h): si <code>c ≤ 0</code> entonces <code>h ← x; c ← floor(a + 0.5)</code>; <code>c ← c − 1</code>; <code>y = h</code>"],
    ["RECTIFIER", "<code>a ∈ {0, 1}</code> por umbral 0.5, pero suavizado: durante 4 ms toma valores intermedios"],
    ["TRIGGER S/H", "estado (h, previo p, rng): <code>disparo = (x ≥ 0) ≠ (p ≥ 0) ∨ |x − h| &gt; a</code>; si disparo y <code>u ≥ b</code>: <code>h ← x</code>; <code>p ← x</code>"],
    ["RING MOD", "fase φ ∈ [0, 1): <code>s = sin(2πφ)</code>; <code>o = s + (sgn(s) − s)·b</code>; <code>φ ← φ + a</code>, si <code>φ ≥ 1</code>: <code>φ ← φ − 1</code>; a = f/fs"],
    ["SLEW LIMITER", "<code>r = 5·10^(4A)/fs</code>; <code>δ<sub>s</sub> = r·0.01^(2(0.5 − B))</code> si B &lt; 0.5, si no r; <code>δ<sub>b</sub> = r·0.01^(2(B − 0.5))</code> si B &gt; 0.5, si no r"],
], ncols=(0,))}
<h3>6.2 Tablas del TABLE SHAPER (N = 1024)</h3>
{code('''s = max(SEED + 1, 1);  repetir 16: s = xorshift32(s)
para i en 0..N-1:  s = xorshift32(s);  raw[i] = 2·s/2^32 − 1
para i en 0..N-1:  j0 = max(0, i−48);  j1 = min(N−1, i+48)
                   smooth[i] = media(raw[j0..j1])            (ventana recortada en los bordes)
normalizar(t):     c = (t[511] + t[512]) / 2;  t[i] −= c;  t[i] /= max|t|   (si max|t| > 0)
lectura T(x):      p = (clamp(x,−1,1) + 1)·0.5·(N−1);  i = min(floor(p), N−2);  f = p − i
                   T(x) = t[i] + (t[i+1] − t[i])·f''')}
<p>Las tablas se regeneran solo cuando cambia T.SEED (comprobación al inicio de cada bloque).</p>
<h3>6.3 Gestión de slots</h3>
{code('''por muestra, para cada slot s:
  si cur[s] ≠ req[s]:                       # cambio de módulo pendiente
      fade[s] ← fade[s] − Δf                 # Δf = 1 / (0.010·fs)
      si fade[s] ≤ 0: fade[s] = 0; cur[s] = req[s]; reiniciar estado (conserva rng); p_sm[s] = p_tgt[s]
  si no:
      fade[s] ← clamp(fade[s] ± Δf, 0, 1)    # + si on[s], − si apagado por el secuenciador
      p_sm[s] ← p_sm[s] + (p_tgt[s] − p_sm[s])·k(4 ms)      (a y b por separado)
salida del slot:  x ← x + (M(x; p_sm) − x)·fade[s]''')}
<h3>6.4 Canal R con SPREAD (0.8)</h3>
<p>SPREAD (0–1, destino de modulación 21) se suaviza como los demás parámetros (4 ms); con objetivo 0, un valor
suavizado menor que 10<sup>−6</sup> se fija en 0. Con <code>sp = SPREAD<sub>sm</sub> &gt; 0</code>, el canal R de cada slot
usa estas variantes; el canal L, y el R con <code>sp = 0</code>, no cambian respecto de la 0.7:</p>
{eq('''RING MOD (R):    s = sin(2π·frac(φ + 0.25·sp))                 (φ avanza igual que en L)
DOWNSAMPLE (R):  N = floor(a + 0.5);  o = floor(sp·N/2)
                 si c ≤ 0: c ← N;   si c = N − o: h ← x;   c ← c − 1;   y = h''')}
<p>DOWNSAMPLE retiene la muestra <code>o</code> muestras después del inicio de cada ciclo de N (hasta medio ciclo de
desfase). El FB TIME del canal R se multiplica por <code>1 + 0.3·sp</code> (20.1). El TRIGGER S/H no cambia: su
xorshift ya es propio de cada slot y canal (semilla <code>12345 + 7919·(2s + c)</code>, 4.4), de modo que sus decisiones
ya son independientes entre L y R.</p>
"""


def s_filtro():
    return f"""
<h2 id="i7">7. Filtro</h2>
<p>SVF trapezoidal de Simper. Coeficientes por muestra a partir de los valores suavizados de corte y resonancia
(estado <code>ic1, ic2</code> por canal; inicial 0):</p>
{eq('''g = tan(π·min(fc, 0.45·fs)/fs);   k = 2 − 1.96·r;   a1 = 1/(1 + g(g + k));   a2 = g·a1;   a3 = g·a2
v3 = x − ic2;   v1 = a1·ic1 + a2·v3;   v2 = ic2 + a2·ic1 + a3·v3
ic1 ← 2v1 − ic1;   ic2 ← 2v2 − ic2
LP = v2;   BP = v1;   HP = x − k·v1 − v2''')}
<p>Con MODE = OFF no se calculan coeficientes ni se actualiza el estado. Los valores de corte y resonancia se suavizan
linealmente en Hz y en [0, 1] respectivamente (one-pole, 4 ms).</p>
<h3>7.1 SCREAM (0.8)</h3>
<p>Con SCREAM inactivo (valor por defecto) el filtro es exactamente el anterior. Con SCREAM activo, la resonancia puede
llevar k a 0 y ligeramente por debajo, y los integradores saturan:</p>
{eq('''k = 2 − 2.04·r                         (r = 1 ⇒ k = −0.04: autooscilación)
ic1 ← tanh(clamp(2v1 − ic1, −20, 20));   ic2 ← tanh(clamp(2v2 − ic2, −20, 20))
si ¬(|ic1| ≤ 1 ∧ |ic2| ≤ 1):  ic1 = ic2 = 0      (protección contra NaN)''')}
<p>Se saturan los integradores (<code>ic1</code> e <code>ic2</code>) y no solo <code>v1</code>: con ambos estados
acotados a |ic| &lt; 1, la energía interna del filtro está acotada para cualquier entrada y cualquier k, y por lo tanto la
salida también lo está (v1, v2 y HP son combinaciones lineales acotadas de x, ic1 e ic2). Saturar solo <code>v1</code>
acota la salida pasabanda, pero <code>ic2</code> sigue integrando <code>v2</code> sin cota garantizada cuando k &lt; 0.
Con k &lt; 0 la amplitud crece hasta que la saturación compensa el amortiguamiento negativo y la oscilación queda en un
ciclo límite cercano a <code>fc</code> (verificado en 17.1: dentro del 5 % de fc, nivel constante).</p>
"""


def s_salida(gen):
    voz = "\n  y ← y · v_post                  (voz, sección 12)" if gen else ""
    return f"""
<h2 id="i8">8. Mezcla y salida (por canal)</h2>
{code(f'''dry = x_entrada{"   (= 0 en el instrumento)" if gen else ""}
wet = (dry + ruido){" · v_pre" if gen else ""}
si GATE POS = PRE: wet ← wet · g_gate
si FILTER POS = PRE y MODE ≠ OFF: wet ← SVF(wet)
wet ← wet · drive_sm
si el bucle está conectado: wet ← wet + fb_c                        (20.1)
para s = 1..4: wet ← slot_s(wet)                  (canal R con SPREAD: 6.4)
so_c = wet;   si FB TAP = PRE FILTER: tap_c = wet
si FILTER POS = POST y MODE ≠ OFF: wet ← SVF(wet)
si FB TAP = POST FILTER: tap_c = wet
y = dry + (wet − dry)·mix_sm
si GATE POS = POST: y ← y · g_gate{voz}
y ← y · (min(1, 1 − p_sm) si canal L;  min(1, 1 + p_sm) si canal R)
y ← y · out_sm
y_dc = y − x1 + R·y1;  x1 = y;  y1 = y_dc;      R = 1 − 2π·10/fs
salida = clamp(y_dc, −0.96605, 0.96605)          (= ±10^(−0.3/20))''')}
<p>Después de los dos canales se escribe el delay del bucle con <code>tap</code> (si está conectado) y se actualiza el
seguidor de envolvente con <code>so</code> o con la entrada (20.2).</p>
"""


def s_seq(gen):
    return f"""
<h2 id="i9">9. Secuenciador</h2>
<h3>9.1 Patrón</h3>
<p><code>steps[4][16]</code> en f32 ∈ [0, 1] y <code>euc_k[4]</code> en u8. Patrón inicial:</p>
{eq('''steps[0][i] = (7i mod 16 < 7) ? (i mod 4 = 0 ? 1 : 0.7) : 0          → golpes en 0,3,5,7,10,12,14
steps[1][i] = i/15
steps[2][i] = 0.5 + 0.5·sin(0.9i)·cos(0.37i)
steps[3][i] = (i impar) ? 0.2 : 0.8          euc_k = [7, 0, 0, 0]''')}
<h3>9.2 Reloj (inicio de bloque)</h3>
{code('''Δq_libre = max(BPM, 1) / 60 / fs
si host.reproduciendo ∧ host.posición_en_negras existe ∧ CLOCK ≠ FREE:
    d = q_host − q_host_anterior
    salto = ¬sincronizado_antes                                                 (inicio)
          ∨ (r existe ∧ |q_host − (q_host_anterior + N_anterior·r)| > 64·r)       (reposicionamiento)
          ∨ (r no existe ∧ N_anterior > 0 ∧ d ≤ 0)                              (retroceso)
    si salto: r = no existe
    si no, si N_anterior > 0 ∧ d > 0: r = d/N_anterior
    calibrado = sincronizado_antes ∧ d > 0 ∧ N_anterior > 0 ∧ |d/N_anterior − Δq_libre| < 0.5·Δq_libre
    Δq = calibrado ? d/N_anterior : Δq_libre
    q = q_host;  q_host_anterior = q_host;  sincronizado = sí;  corriendo = sí
    si salto: resincronizar (9.7)
si no:
    sincronizado = no;  r = no existe;  Δq = Δq_libre;  corriendo = (CLOCK ≠ HOST ONLY)
N_anterior = N''')}
<p>La posición del host se expresa en negras (NIH-plug <code>pos_beats()</code>; en el JSFX,
<code>beat_position·4/ts_denom</code>). En modo libre <code>q</code> continúa desde su último valor.</p>
<p><code>r</code> es el avance real del host, en negras por muestra, medido entre los dos últimos bloques sincronizados
sin salto. El umbral de reposicionamiento es de <b>64 muestras</b> (<code>JUMP_SAMPLES</code>): hay salto cuando la
posición del bloque se aparta de la prevista más de 64 muestras al avance real del host, de modo que el criterio no
depende del tempo, de las unidades del host ni de la velocidad de reproducción. Un desplazamiento menor no se considera
salto. Justo después de una resincronización, mientras <code>r</code> no existe, solo se reconoce el retroceso. Un
bloque vacío (N = 0) no altera <code>r</code>. Iniciar la reproducción, saltar al reproducir en bucle y cambiar CLOCK de
FREE RUN a AUTO o HOST ONLY durante la reproducción producen, por tanto, una resincronización. Con el reloj libre no hay
resincronización.</p>
<h3>9.3 Pasos por negra</h3>
<p>Índices de RATE 0..11 = 8 BAR, 4 BAR, 2 BAR, 1 BAR, 1/2, 1/4, 1/8T, 1/8, 1/16T, 1/16, 1/32, 1/64;
<code>Q<sub>c</sub> = num·4/max(den, 1)</code>:</p>
{eq('''spq = 1/(8Qc), 1/(4Qc), 1/(2Qc), 1/Qc, 0.5, 1, 1.5, 2, 3, 4, 8, 16''')}
<h3>9.4 Reloj de carril (por muestra, si corriendo)</h3>
{code('''para cada carril l (0..3):
    t = (q − q0)·spq[l];  par = floor(t/2);  φ = t − 2·par;  b = SWING/50
    si φ < b:  n = 2·par;      fase[l] = φ/b
    si no:     n = 2·par + 1;  fase[l] = (φ − b)/(2 − b)
    si n ≠ n_anterior[l]: paso(l, n)
q ← q + Δq''')}
<p><code>n_anterior</code> se inicializa en −2^63 para forzar el primer paso. <code>q0</code> = 0
{"salvo tras una nota con RETRIG (12.6)" if gen else "en el efecto"}.</p>
<h3>9.5 Paso</h3>
{code('''paso(l, n):
    L = clamp(STEPS[l], 1, 16)
    i = FWD: n mod L | BWD: L−1−(n mod L) | PINGPONG: (L<2 ? 0 : p<L ? p : 2L−2−p, p = n mod (2L−2))
        | RANDOM: floor(h(n,l,1,SEED)·L) | DRUNK: (i_anterior + (h(n,l,2,SEED) < 0.5 ? −1 : +1)) mod L
    disparado = h(n,l,3,SEED)·100 < PRB[l]
    si disparado:
        si u_rnd·100 < MUT[l]:  steps[l][i] = clamp(steps[l][i] + (u_rnd − 0.5)·0.6, 0, 1)   (dos sorteos)
        objetivo[l] = steps[l][i]
    si no, si l = 0: objetivo[0] = 0           (los carriles MOD conservan el objetivo)
    i_anterior[l] = i;  n_anterior[l] = n
    T = 60/(max(BPM,1)·spq[l]);  τ = max(0.0005, GLD[l]·T/2);  kg[l] = k(τ)''')}
<h3>9.6 Gate y glide (por muestra)</h3>
{code('''si corriendo:  obj = (disparado[0] ∧ fase[0] < LEN) ? objetivo[0] : 0
si no:         obj = 1
e ← e + (obj − e)·(obj > e ? k(ATK) : k(REL))          e inicial = 1
g_gate = 1 − DEPTH + DEPTH·e
para l = 1..3: val[l] ← val[l] + (objetivo[l] − val[l])·kg[l]''')}
<h3>9.7 Resincronización</h3>
<p>Ocurre al inicio del bloque, después de fijar <code>q</code>, cuando el reloj sincronizado detecta un salto (9.2).
Devuelve a su estado inicial todo lo que depende del número de muestras procesadas o de la señal anterior:</p>
{code(f'''resincronizar():
    ruido:      LCG ← (22222, 777777);  LFSR ← (1, 0x5A5A);  φ_LFSR ← 0;
                Kellet b0..b6 ← 0;  BROWN ← 0;  DUST ← 0                  (ambos canales)
    slots:      estado inicial de cada slot y canal (contador y retenido de DOWNSAMPLE; h y p del
                TRIGGER S/H; fase del RING MOD; salida del SLEW);  xorshift del S/H ← 12345 + 7919·(2s + c);
                fade[s] ← 1
    filtro:     ic1 ← ic2 ← 0;   DC blocker: x1 ← y1 ← 0
    bucle:      delay, tono y DC blocker del bucle ← 0 (20.1);   seguidor: e ← 0 (20.2)
    carriles:   n_anterior ← −2^63, i_anterior ← 0, disparado ← no, fase ← 0, objetivo ← 0, val ← 0, kg ← 1
                para cada carril: reloj de carril (9.4) en la posición q        → primer paso
                carriles MOD: val ← objetivo                                  (el primer paso no hace glide)
    gate:       e ← 0
    modulación: contador ← 0;  evaluar (10);  suavizados (incluidos FB AMOUNT, FB TIME y SPREAD),
                módulos y p_sm ← objetivo   (sin rampa)
sin cambios:    patrón, generador de la mutación, tablas del TABLE SHAPER{", voz (notas, envolvente, q0)" if gen else ""}''')}
<p>El reinicio de los filtros de Kellet y BROWN, del SVF y del DC blocker es necesario: su estado es una función de
toda la señal anterior y, sin restablecerlo, dos renders con distinto preroll diferirían durante segundos (la memoria del
DC blocker decae con <code>R ≈ 0.9987</code> por muestra). El coste es una posible discontinuidad en un salto durante
la reproducción (por ejemplo, el BROWN vuelve a 0), simultánea con la discontinuidad del propio salto. El gate parte de
0 para que el primer paso entre con su ataque y no deje pasar señal si no está disparado.</p>
"""


def s_mod(gen):
    pitch = """
si VOZ activa y hay nota n:
    fc_obj ← clamp(fc_obj · 2^((n−60)/12 · TRACK), 20, 20000)
    si PITCH: f0 = 440·2^((n−69)/12);  LFSR: t = 6, Δ = min(93·f0/fs, 4)
              para slots con RING MOD: a_obj = f0/fs""" if gen else ""
    return f"""
<h2 id="i10">10. Matriz de modulación (cada 16 muestras)</h2>
{code(f'''m[0..21] = 0;  on[0..3] = verdadero;  E = min(e, 1)                 (e: seguidor, 20.2)
para l = 1..3 con d = DEST[l], D = DEP[l]/100:
    v = SRC[l] = SEQ ? val[l] : SRC[l] = ENV ? E : val[l]·E        (SEQ×ENV)
    1 ≤ d ≤ 13 ∨ 19 ≤ d ≤ 21:  m[d] += D·v
    d = 14:      m[14] += D·(2v − 1)
    15 ≤ d ≤ 18 ∧ D ≠ 0:  on[d−15] = (D > 0) ? (v ≥ 0.5) : (v < 0.5)
para s = 0..3:  A' = clamp(A_s + m[1+2s], 0, 1);  B' = clamp(B_s + m[2+2s], 0, 1)
                req[s] = módulo_s;  p_tgt[s] = map(módulo_s, A', B')
drive_obj = dB(clamp(DRIVE + 48·m[9], −24, 48))
mix_obj   = clamp(MIX + m[10], 0, 1)
fc_obj    = 20·1000^clamp(ln(CUTOFF/20)/ln(1000) + m[11], 0, 1)
reso_obj  = clamp(RESO + m[12], 0, 1)
ruido_obj = (sección 5, con m[13]);   out_obj = dB(OUT);   pan_obj = clamp(m[14], −1, 1)
fb_amt_obj  = clamp(FB_AMT + 1.5·m[19], 0, 1.5)
fb_time_obj = 0.05·1000^clamp(ln(FB_TIME/0.05)/ln(1000) + m[20], 0, 1)        (ms)
spread_obj  = clamp(SPREAD + m[21], 0, 1){pitch}''')}
<p>Los índices de destino 0–18 conservan su significado; los de la 0.8 (19 FB AMOUNT, 20 FB TIME, 21 SPREAD) se agregaron
al final. FB TIME se modula en la escala logarítmica de su rango, como CUTOFF. La fuente ENV se lee aquí, a la tasa de
la matriz (cada 16 muestras), aunque el detector corre en cada muestra (20.2).</p>
<p>El contador se inicializa en 0, por lo que la primera evaluación ocurre en la primera muestra.
{"Una nota nueva fuerza una evaluación inmediata." if gen else ""}</p>
"""


def s_voz():
    return f"""
<h2 id="i12">12. Voz (instrumento)</h2>
<h3>12.1 Pila de notas</h3>
{code('''note_on(n, vel):  quitar n de la pila; si la pila tiene 16, descartar la más antigua
                  apilar n; nota = n; velocidad = clamp(vel, 0, 1); etapa = ATAQUE
                  si RETRIG: q0 = q; para cada carril: n_anterior = −2^63, i_anterior = 0;
                             ruido ← estado inicial (LCG, LFSR y fase, Kellet, BROWN, DUST; 4.4)
                  contador de control = 0
note_off(n):      quitar n; si quedan notas: nota = cima (sin reiniciar envolvente)
                  si no y etapa ≠ REPOSO: etapa = RELEASE
eventos:          NoteOn con vel = 0 ⇒ note_off;  Choke ⇒ note_off;  otros eventos se ignoran''')}
<h3>12.2 Envolvente (por muestra)</h3>
{code('''ATAQUE:   e ← e + 1/(max(ATK,0.1 ms)·fs);  si e ≥ 1: e = 1, etapa = CAÍDA
CAÍDA:    e ← e + (SUS − e)·k(DEC);  si |e − SUS| < 1e−4: etapa = SOSTENIDO
SOSTENIDO: e = SUS
RELEASE:  e ← e − e·k(REL);  si e < 1e−5: e = 0, etapa = REPOSO
REPOSO:   e = 0
v = e·(1 − VEL + VEL·velocidad)
ENV = PRE:  v_pre = v, v_post = 1;     ENV = POST: v_pre = 1, v_post = v''')}
<p>Las constantes <code>k(DEC)</code>, <code>k(REL)</code> y el incremento de ataque se calculan al inicio de cada bloque
(mínimo 0.1 ms). La envolvente se evalúa antes del suavizado de parámetros de la muestra.</p>
<h3>12.3–12.6 Seguimiento, afinación y reinicio</h3>
<p>Definidos en la sección 10 (TRACK, PITCH) y 12.1 (RETRIG). Con RETRIG inactivo, <code>q0</code> se fuerza a 0 al
inicio de cada bloque. En modo PITCH el período del LFSR con toma en el bit 6 es de 93 pasos desde los estados semilla
utilizados; la frecuencia fundamental resultante es <code>Δ·fs/93 = f0</code>, con techo <code>4·fs/93</code>.</p>
"""


def s_orden(gen):
    voz = "\n    v = voz()                              (12.2)" if gen else ""
    ev = "\n    procesar eventos MIDI con tiempo = j    (12.1)" if gen else ""
    return f"""
<h2 id="i11">11. Orden de ejecución</h2>
{code(f'''por bloque de N muestras:
    copiar patrón (try_read);  Params ← parámetros del host
    begin_block:  spq de cada carril; coeficientes de ruido; b de swing; k(ATK), k(REL) del gate;
                  tablas (si cambió T.SEED);{" coeficientes de la voz; q0 = 0 si no RETRIG;" if gen else ""}
                  reloj (9.2) y, si hubo salto, resincronización (9.7);
                  si primera vez: aplicar modulación y saltar suavizados a su objetivo
    para j = 0..N−1:{ev}
        secuenciador (9.4–9.6)
        si contador = 0: modulación (10), contador = 16
        contador ← contador − 1
        slots (6.3){voz}
        suavizar drive, mix, out, fc, reso, ruido, pan con k(4 ms)
        suavizar SPREAD; suavizar FB AMOUNT y FB TIME; leer el bucle (20.1)
        coeficientes SVF (si MODE ≠ OFF)
        ruido estéreo (5)
        canales L y R (8)
        escribir el delay del bucle (20.1);  seguidor de envolvente (20.2)
    si hubo mutación: devolver patrón (try_write)
    si el editor está abierto: publicar Snapshot (try_lock)''')}
"""


def s_params(gen):
    rows = PARAMS_CORE + (PARAMS_VOICE if gen else [])
    t = table(["ID", "Nombre en el host", "Rango", "Defecto", "Unidad"],
              [["<code>" + a + "</code>", b, c, d, e] for a, b, c, d, e in rows], ncols=(0,))
    return f"""
<h2 id="i13">13. Parámetros del plugin</h2>
<p>Parámetros declarados con <code>#[derive(Params)]</code> de NIH-plug. Los slots y carriles MOD son
<code>#[nested(array)]</code> y reciben sufijo <code>_1.._N</code>.{" El instrumento anida el núcleo en el grupo «NOISEGEK» y la voz en «Voice» sin prefijo de ID." if gen else ""}</p>
{t}
<h3>13.1 Normalización</h3>
{table(["Tipo", "Normalizado ↔ plano"], [
    ["Lineal", "<code>u = (v − min)/(max − min)</code>"],
    ["Skewed (factor f = 2^−2 = 0.25)", "<code>u = ((v − min)/(max − min))^f</code>;  <code>v = min + (max − min)·u^(1/f)</code>. Usado en CUTOFF, ATK/REL del gate, ATK/DEC/REL de la voz y ATK/REL del seguidor"],
    ["Logarítmico (parche NOISEGEK, 16.1)", "<code>u = ln(v/min)/ln(max/min)</code>;  <code>v = min·(max/min)^u</code>. Usado en FB TIME y FB TONE"],
    ["Entero", "<code>u = (v − min)/(max − min)</code>; el plano se redondea"],
    ["Booleano", "0 / 1"],
], ncols=(0,))}
<h3>13.2 Conversión a <code>Params</code> del motor</h3>
<p>Valores directos excepto: DEP y GLD se dividen entre 100; PRB, MUT y SWING se pasan en porcentaje; MIX, RESO, DEPTH,
LEN, SUS, VEL, TRACK, CHARACTER y SPREAD están en [0, 1]; FB AMOUNT en [0, 1.5]; las posiciones (POS, ENV, FB TAP) se
traducen a booleanos (1 = POST/PRE CHAIN/POST FILTER según el control). SRC: 0 = SEQ, 1 = ENV, 2 = SEQ×ENV. ENV TAP:
1 = PRE-FILTER; 0 = INPUT en el efecto y VOICE en el instrumento (que no tiene entrada de audio).</p>
<h3>13.3 Estado persistente</h3>
{code('''PluginState (JSON, serde):
{
  "version": "<versión>",
  "params": { "<id>": {"f32": v} | {"i32": v} | {"bool": v}, ... },     # valores PLANOS
  "fields": {
    "pattern":      <cadena con el JSON {"steps": [[16 × f32] × 4], "euc_k": [4 × u8]}>,
    "editor-state": "<tamaño de ventana serializado por nih_plug_egui>"
  }
}
CLAP:  [u64 little-endian = longitud][JSON]       (estados > 64 MiB se rechazan; parche NOISEGEK)
VST3:  JSON a través de IBStream''')}
<p><b>Versión y migración (0.8).</b> <code>version</code> es la versión del plugin que guardó el estado. Antes de
aplicarlo, <code>Plugin::filter_state</code> llama a <code>migrate_state</code>: si la versión es anterior a la 0.8
(incluido un estado sin versión), inserta los 13 parámetros nuevos con su valor por defecto; si es anterior a la 0.8.1,
descarta además el tamaño de ventana guardado (campo <code>editor-state</code>), para abrir con el tamaño inicial nuevo. Sin esa migración, un preset
de la 0.6 o la 0.7 cargado en una instancia con, por ejemplo, FB AMOUNT distinto de 0 conservaría ese valor y no sonaría
como se guardó. Con los valores por defecto la salida es idéntica a la de la 0.7 bit a bit (17.1). En el JSFX, los
sliders 61–73 que no figuran en un proyecto anterior toman su valor por defecto; <code>@serialize</code> no cambia.</p>
{table(["ID", "Valor insertado"], [
    ["<code>fbamt</code>, <code>fbtime</code>, <code>fbtone</code>, <code>fbtap</code>", "0, 5 ms, 6000 Hz, 0 (PRE FILTER)"],
    ["<code>scream</code>", "falso"],
    ["<code>eatk</code>, <code>erel</code>, <code>egain</code>", "5 ms, 150 ms, 0 dB"],
    ["<code>etap</code>", "0 (INPUT) en el efecto; 1 (PRE-FILTER) en el instrumento"],
    ["<code>spread</code>", "0"],
    ["<code>src_1</code>, <code>src_2</code>, <code>src_3</code>", "0 (SEQ)"],
], ncols=(0,))}
<h3>13.4 Determinismo</h3>
<p>Con el reloj sincronizado, a partir de cada resincronización (9.7) la salida depende solo de los parámetros, el
patrón, la posición del host{", los eventos de nota" if gen else ""} y la entrada de audio posterior a la
resincronización: no depende de cuántas muestras se procesaron antes de reproducir (preroll) ni de lo que sonó antes de
un salto. La única excepción es la mutación (MUT &gt; 0), que usa un generador sembrado con la hora y modifica el
patrón.{" Con RETRIG, además, cada nota reinicia la fuente de ruido (12.1), de modo que la fuente arranca igual en cada nota también con el reloj libre." if gen else ""}</p>
<p>Con el reloj libre (CLOCK = FREE RUN, CLOCK = AUTO con el transporte detenido o un host que no informa la posición)
no hay resincronización y se conserva el comportamiento anterior a la versión 0.7: la fase del ruido, del LFSR, del
RING MOD y del xorshift del TRIGGER S/H depende del número de muestras procesadas desde la inicialización del
plugin.</p>
"""


def s_ui(gen):
    filas = ui_layout(gen)
    return f"""
<h2 id="i14">14. Interfaz gráfica</h2>
<h3>14.1 Lienzo y escala</h3>
{eq(f'''lienzo de diseño: 1000 × {686 if gen else 632} unidades     (0.7: {712 if gen else 650}; 0.8.0: {774 if gen else 712})
s = max(0.3, min(ancho/1000, alto/{686 if gen else 632}));   origen x = (ancho − 1000·s)/2 (≥ 0), y = 0
ventana inicial: {"960 × 659" if gen else "1000 × 632"} píxeles lógicos; mínima 600 × 380''')}
<p>La 0.8 agregó la fila FEEDBACK · ENV FOLLOW · STEREO bajo {"VOICE" if gen else "SEQUENCER"}, el interruptor SCREAM en
el panel FILTER (junto a RESO) y el selector SRC en la cabecera de cada carril MOD. La 0.8.1 compactó el lienzo para que
la ventana inicial, con el marco del host, quepa en una pantalla de 1366 × 768: cabecera de 40 unidades; paneles
SOURCE, CHAIN y FILTER de 144; filas SEQUENCER, {"VOICE, " if gen else ""}FEEDBACK · ENV FOLLOW · STEREO de 50 con
controles de 22; carriles de 70 cada 74 con controles de 20. Los tamaños de letra no cambiaron. La migración del estado
(13.3) descarta el tamaño de ventana guardado por versiones anteriores a la 0.8.1.</p>
<h3>14.2 Paleta y tipografía</h3>
{table(["Token", "Color", "Uso"], [
    ["VOID", "#0A0A0A", "fondo, interior de barras y selectores"],
    ["IRON", "#1A1A1A", "paneles, barra de ayuda"],
    ["WHITE", "#E6E6E6", "texto (etiquetas al 60 % de opacidad), pasos (70 %)"],
    ["RED", "#FF0033", "activo: paso que suena, marca de modulación (3 u de ancho), LEDs, medidores, hover de botones"],
    ["SHADOW", "#4D4D4D", "relleno de barras, inactivo"],
], ncols=(0,))}
{table(["Familia", "Archivo (embebido)", "Tamaños (unidades × s)"], [
    ["anton", "Anton-Regular.ttf (OFL)", "logo 38, títulos 18, GEN 24"],
    ["monob", "CourierPrime-Bold.ttf (OFL)", "etiquetas 12, valores 13"],
    ["mono", "CourierPrime-Regular.ttf (OFL)", "ayuda y descripciones 13"],
], ncols=(0,))}
<h3>14.3 Widgets</h3>
{table(["Widget", "Comportamiento"], [
    ["barra", "arrastre: <code>u ← clamp(u + (Δx − Δy)/(250·s)·(Shift ? 0.1 : 1), 0, 1)</code>; doble clic: valor por defecto; rueda: <code>±1/pasos</code> (discretos) o ±0.02 (±0.005 con Shift). Relleno desde 0, o desde 0.5 si es bipolar (DEP)"],
    ["selector", "clic: menú emergente bajo el control; rueda: opción anterior/siguiente con envoltura"],
    ["interruptor", "clic: alterna; LED 8×8 u"],
    ["grilla de pasos", "valor = <code>clamp(1 − (y − (ly + 6))/(LHT − 12), 0, 1)</code>; botón derecho = 0; relleno entre el paso anterior y el actual durante el arrastre"],
    ["ayuda", "texto del último widget bajo el puntero; si ninguno, texto por defecto"],
], ncols=(0,))}
<p>Cada interacción con un parámetro se envía al host con <code>begin_set_parameter</code> /
<code>set_parameter</code> / <code>end_set_parameter</code>; el arrastre envía un único begin/end.</p>
<h3>14.4 Geometría (extraída de <code>editor.rs</code>)</h3>
<p>Coordenadas en unidades de diseño. En la fila FEEDBACK · ENV FOLLOW · STEREO, <code>fy = {324 if gen else 270}</code>.
En los carriles, <code>ly = {356 if gen else 302} + 74·l</code>,
<code>y1 = ly + 24</code>, <code>y2 = ly + 46</code>; grilla en x = 456, ancho 528, alto {70 - 6} desde <code>ly + 3</code>.</p>
{table(["Widget", "Control", "x", "y", "ancho", "alto"], filas, ncols=(0, 2, 3, 4, 5))}
"""


def s_jsfx():
    rows = jsfx_sliders()
    return f"""
<div class="part">PARTE IV · IMPLEMENTACIÓN JSFX</div>
<h2 id="i15">15. NOISEGEK.jsfx</h2>
<p>Implementación en EEL2 para REAPER del mismo algoritmo de señal (secciones 3–11). Archivo:
<code>NOISEGEK.jsfx</code> en la raíz del proyecto (copia de trabajo en <code>%APPDATA%{BS}REAPER{BS}Effects{BS}noise{BS}</code>).</p>
<h3>15.1 Sliders</h3>
<p>73 sliders, todos ocultos (prefijo <code>-</code>) y automatizables. Los 16 primeros conservan la numeración de la
versión 0.1; los 61–73 se agregaron en la 0.8 (REAPER admite hasta 256).</p>
{table(["#", "Nombre", "Defecto", "Rango / opciones"], rows, ncols=(0, 2))}
<h3>15.2 Particularidades de EEL2</h3>
{table(["Aspecto", "Solución"], [
    ["Enteros de 32 bits", "EEL2 opera en double; xorshift32 se emula con XOR por mitades de 16 bits (<code>xor32</code>), desplazamientos por multiplicación/división y módulo 2^32 con <code>floor</code>. El operador <code>~</code> es XOR"],
    ["Nombres de variables", "insensibles a mayúsculas: no se usan pares como <code>L</code>/<code>l</code>"],
    ["Hilos", "<code>@sample</code>/<code>@block</code> y <code>@gfx</code> comparten variables globales y corren en hilos distintos; las globales del audio llevan prefijo <code>a_</code> y la interfaz no invoca la evaluación de la modulación"],
    ["Persistencia del patrón", "<code>@serialize</code>: <code>ser_ver</code> (= 1), 64 valores de pasos, 4 valores EUC; deshacer mediante <code>sliderchange(−1)</code>"],
    ["Reinicio", "<code>ext_noinit = 1</code>: <code>@init</code> no se ejecuta al iniciar la reproducción; el patrón por defecto se escribe solo una vez"],
    ["Memoria", "asignador lineal; el patrón y EUC ocupan las primeras 68 posiciones y el estado volátil se borra con <code>memset</code> a partir de ellas"],
    ["Interfaz", "lienzo 1000 × 632 escalado (0.7: 1000 × 650; 0.8.0: 1000 × 712); <code>gfx_ext_retina = 1</code>; ventana solicitada 1000 × 632; fuentes Impact y Courier New del sistema"],
    ["Sliders 65 en adelante", "en el <code>.rpp</code> de REAPER, la línea del JSFX guarda los sliders 1–64, luego el nombre del preset y después los sliders 65 en adelante"],
    ["Literales numéricos", "EEL2 no admite notación científica: <code>1e30</code> impide compilar todo el script (en un render sin interfaz se traduce en silencio). Se escribe <code>10 ^ 30</code>"],
    ["Patrón", "el JSFX guarda los pasos en doble precisión; la versión VST3/CLAP, en f32 (13.3). Ver 17.3"],
    ["Sliders &gt; 32", "<code>slider_automate(sliderN)</code> con la variable explícita (la máscara de bits solo cubre 1–31)"],
], ncols=(0,))}
"""


def s_build(gen):
    t = toolchain_info()
    crate = "noisegek-gen" if gen else "noisegek"
    nb = "NOISEGEK GEN" if gen else "NOISEGEK"
    procedimiento = code(
        "cd vst\n"
        "powershell -ExecutionPolicy Bypass -File ." + BS + "build.ps1 -Test\n"
        "   → cargo test -p noisegek-dsp --release\n"
        "   → cargo xtask bundle -p noisegek -p noisegek-gen --release\n"
        "   → " + BS.join(["vst", "dist", nb + ".vst3", "Contents", "x86_64-win", nb + ".vst3"]) + "   y   " + nb + ".clap\n"
        "   → regenera la documentación (carpetas de distribución y este documento)")
    return f"""
<div class="part">PARTE V · CONSTRUCCIÓN Y VERIFICACIÓN</div>
<h2 id="i16">16. Construcción</h2>
{table(["Componente", "Versión / origen"], [
    ["Compilador", f"rustc {t['rustc']} estable, objetivo <code>x86_64-pc-windows-gnu</code> (enlazador MinGW incluido; sin Visual Studio)"],
    ["Ubicación del toolchain", "<code>vst/.toolchain</code> (RUSTUP_HOME y CARGO_HOME locales; sin cambios en PATH ni registro)"],
    ["NIH-plug", f"commit <code>{t['nih']}</code>, copia en <code>vst/third_party/nih-plug</code> con 3 parches"],
    ["egui", t["egui"]],
    ["Perfil release", "LTO thin, codegen-units = 1, símbolos eliminados"],
], ncols=(0,))}
<h3>16.1 Parches sobre NIH-plug</h3>
{table(["Archivo", "Cambio", "Motivo"], [
    ["<code>src/wrapper/clap/wrapper.rs</code>", "rechazar estados CLAP de más de 64 MiB", "un estado corrupto provocaba una reserva de memoria imposible y abortaba el host"],
    ["<code>src/wrapper/clap/wrapper.rs</code>", "solicitar <code>CLAP_PARAM_RESCAN_VALUES</code> tras cargar un estado", "requisito de la especificación CLAP"],
    ["<code>src/params/range.rs</code>, <code>float.rs</code>", "<code>FloatRange::Logarithmic {{ min, max }}</code>", "NIH-plug solo ofrece rangos lineales y con potencia (Skewed), que no reproducen una escala logarítmica; lo usan FB TIME y FB TONE (0.8)"],
], ncols=(0,))}
<p>La característica <code>assert_process_allocs</code> de NIH-plug es incompatible con el objetivo GNU y no se usa.
<code>build.ps1 -Test</code> ejecuta las pruebas de <code>noisegek-dsp</code> y de <code>noisegek-core</code>.</p>
<h3>16.2 Procedimiento</h3>
{procedimiento}
<p>Crate exportador: <code>{crate}</code> (<code>crate-type = ["cdylib"]</code>). El efecto y el instrumento se compilan
como binarios separados: FL Studio no verifica por defecto archivos con varias clases de plugin. El binario solo importa
DLL del sistema (ADVAPI32, bcrypt, GDI32, KERNEL32, msvcrt, ntdll, ole32, oleaut32, OPENGL32, SHELL32, USER32,
userenv, ws2_32) y exporta <code>GetPluginFactory</code>, <code>InitDll</code>, <code>ExitDll</code> y
<code>clap_entry</code>.</p>
"""


def s_verif(gen):
    tests = test_names()
    val = ("44 pruebas: 35 superadas, 0 fallidas, 9 no aplicables" if gen
           else "44 pruebas: 33 superadas, 0 fallidas, 11 no aplicables")
    return f"""
<h2 id="i17">17. Verificación</h2>
<h3>17.1 Pruebas unitarias del núcleo ({len(tests)})</h3>
{table(["Archivo", "Prueba"], tests, ncols=(0,))}
<h3>17.2 Validación del formato</h3>
<p><code>clap-validator</code> (free-audio): {val}. Cubre carga, descriptor, procesado in-place y out-of-place,
tamaños de bloque y frecuencias de muestreo variables, denormales, eventos de parámetro, modulación, conversión
texto↔valor, reproducibilidad y robustez del estado, y transporte nulo y aleatorio.</p>
<h3>17.3 Equivalencia con el JSFX</h3>
<p>El JSFX se verificó por renderizado sin interfaz en REAPER (<code>reaper.exe -renderproject</code>, WAV de 24 bits)
frente al núcleo Rust con la misma configuración, en compases 4/4 y 6/8, con reloj sincronizado y libre y con los tipos
de ruido WHITE, BROWN, DUST y LFSR. Con el reloj sincronizado y MUT = 0, las dos implementaciones coinciden muestra a
muestra: la diferencia máxima (≤ 7.5·10<sup>−8</sup>) es la cuantización a 24 bits; en esas configuraciones el
resultado Rust fue además idéntico bit a bit con bloques de 64 a 4096 muestras. Eso no está garantizado en general: la
posición se reancla al host al inicio de cada bloque y, dentro del bloque, avanza sumando el incremento por muestra, de
modo que un cambio de paso que cae exactamente en una muestra puede adelantarse o atrasarse una muestra según la
segmentación. REAPER renderiza en bloques de 512 muestras y las comparaciones usan ese tamaño. Con MUT &gt; 0 o con reloj libre la salida difiere, como declara la sección
13.4; el patrón del gate y el RMS por paso coinciden mientras la mutación no altera el patrón.</p>
<p><b>0.8.</b> Los proyectos de la 0.7 renderizados con el JSFX 0.8 son idénticos bit a bit a sus renders con el JSFX
0.7 (los sliders nuevos toman su valor por defecto). Con las funciones nuevas activas, el JSFX y el núcleo Rust coinciden
muestra a muestra (diferencia ≤ 9·10<sup>−8</sup>, la cuantización a 24 bits) en realimentación con AMOUNT &gt; 1 y toma
POST FILTER, seguidor de envolvente con fuentes ENV y SEQ×ENV, SPREAD con RING MOD, DOWNSAMPLE y realimentación, y todas
las funciones combinadas. SCREAM coincide dentro de 2.5·10<sup>−6</sup> (−112 dB): la autooscilación amplifica la
diferencia de redondeo entre el <code>tanh</code> del JSFX (<code>1 − 2/(e^{{2x}} + 1)</code>) y el de Rust.</p>
<p>Con FB TIME modulado, la equivalencia es exacta solo si los pasos del patrón son representables en f32: la versión
VST3/CLAP guarda el patrón en f32 (desde la 0.5) y el JSFX en doble precisión, así que los valores de los carriles
difieren en ~10<sup>−8</sup>. En otros destinos esa diferencia no es visible, pero un delay modulado la convierte en
~10<sup>−7</sup> de salida, y una cadena caótica dentro del bucle (FOLDBACK a ganancia alta) la amplifica. Con el patrón
redondeado a f32 en el JSFX, ambos coinciden muestra a muestra; la cadena caótica queda dentro de 8.5·10<sup>−5</sup>, la
misma sensibilidad que muestra el núcleo Rust frente a una perturbación de 10<sup>−15</sup> en FB TIME. La diferencia
afecta solo al patrón inicial antes de guardar: al guardar, el JSFX escribe el patrón con <code>@serialize</code> en 32
bits y lo restaura así. Se conserva para que los proyectos JSFX anteriores sigan siendo idénticos.</p>
<p><b>Presets de demostración (0.8).</b> Los cuatro presets (carpeta <code>presets</code> de la distribución del efecto)
se renderizaron con el JSFX y se compararon con el núcleo Rust con bloques de 512 muestras: FB Metal y Spread Wide
coinciden dentro de 4.8·10<sup>−7</sup>; Scream Siren, dentro de 6·10<sup>−4</sup> (la autooscilación acumula en su fase la
diferencia de redondeo); Env Follow es caótico: el seguidor modula un RING MOD cuya salida vuelve al propio seguidor, y el
núcleo Rust frente a sí mismo con ENV GAIN alterado en 10<sup>−12</sup> dB ya difiere en 0.29, por lo que su comparación
muestra a muestra no es significativa (el nivel RMS coincide dentro del 0.1 %). Los archivos <code>.vstpreset</code> se
validaron como estados de NIH-plug (73 parámetros con su tipo, patrón incluido).</p>
<h3>17.4 Interfaz</h3>
<p>La interfaz se verificó por captura del ejecutable standalone (<code>--backend dummy</code>) y, en el JSFX, de la
ventana de REAPER.</p>
<h3>17.5 Rendimiento</h3>
<p>Tiempo de CPU por bloque de 512 muestras a 48 kHz (presupuesto de tiempo real: 10 667 µs), con señal de entrada y
transporte en reproducción: mínimo de 7 × 2000 bloques, repetido 5 veces, con el proceso de prioridad alta fijado a un
núcleo, en tres corridas. Entre corridas la medición varía en torno a ±10 %. <i>default</i> es la configuración inicial;
<i>heavy</i> usa TRIGGER S/H, RING MOD, FOLDBACK y SLEW en los cuatro slots, ruido rosa, filtro LP con resonancia 0.8,
gate y tres carriles modulando.</p>
{table(["Configuración", "0.7 (µs/bloque)", "0.8 (µs/bloque)", "% del tiempo real (0.8)"], [
    ["default", "109–120", "122–136", "≈ 1.2 %"],
    ["heavy", "199–202", "202–228", "≈ 2.0 %"],
    ["heavy + realimentación (AMOUNT 0.9, TIME 7 ms, modulado)", "—", "232–266", "≈ 2.4 %"],
    ["heavy + SCREAM (RESO 1)", "—", "244–275", "≈ 2.4 %"],
    ["heavy + ENV (PRE-FILTER, SRC ENV y SEQ×ENV)", "—", "201–226", "≈ 2.0 %"],
    ["heavy + SPREAD 0.8", "—", "208–240", "≈ 2.1 %"],
    ["todo activo", "—", "267–298", "≈ 2.7 %"],
], ncols=(0,))}
<p>Con las funciones nuevas apagadas, el costo agregado es el suavizado de AMOUNT, TIME y SPREAD y el detector de
envolvente, que siempre corre (≈ 4 µs por bloque). La ruta del canal R con SPREAD está fuera de línea para no penalizar
el caso SPREAD = 0.</p>
"""


def s_fb_env(gen):
    tap = ("salida de S4 (PRE-FILTER) o v de la voz (VOICE, 12.2)" if gen
           else "entrada seca x (INPUT) o salida de S4 (PRE-FILTER)")
    return f"""
<h2 id="i20">20. Realimentación y seguidor de envolvente</h2>
<p>Funciones agregadas en la 0.8. Con sus valores por defecto (FB AMOUNT = 0 y SRC = SEQ en los tres carriles) la
salida es idéntica a la de la 0.7 bit a bit.</p>
<h3>20.1 Bucle de realimentación global</h3>
<p>La salida de la cadena de slots (o del filtro principal) vuelve a la entrada del slot 1 a través de un delay
fraccional, un filtro de un polo (TONE), un bloqueador de DC, la ganancia y una saturación tanh. AMOUNT y TIME se
suavizan como los demás parámetros (4 ms) y TIME no recibe otro suavizado: modularlo mueve el punto de lectura del delay
en cada muestra y produce el desplazamiento de altura propio de un delay modulado.</p>
{eq('''AMT_sm ← AMT_sm + (AMT_obj − AMT_sm)·k(4 ms);   si AMT_obj = 0 ∧ AMT_sm < 10⁻⁶: AMT_sm = 0
TIME_sm ← TIME_sm + (TIME_obj − TIME_sm)·k(4 ms)                                     (ms)
conectado = AMT_obj > 0 ∨ AMT_sm > 0;    al pasar a conectado: delay, tono y DC ← 0''')}
{eq('''por canal c, si el bucle está conectado:
d_c  = clamp(TIME_sm · (c = R ? 1 + 0.3·SPREAD_sm : 1) · fs/1000, 1, 32766)        (muestras)
i = floor(d_c);  f = d_c − i
r_c  = buf_c[w − i] + (buf_c[w − i − 1] − buf_c[w − i])·f                    (índices mod 32768)
t_c ← t_c + (r_c − t_c)·a,     a = 1 − exp(−2π·min(TONE, 0.45·fs)/fs)
z_c  = t_c − x1_c + R·y1_c;   x1_c = t_c;   y1_c = z_c                       (R = 1 − 2π·10/fs)
fb_c = tanh(clamp(AMT_sm·z_c, −20, 20))''')}
{eq('''entrada del slot 1:  wet_c ← wet_c · drive_sm + fb_c
toma:   tap_c = salida de S4 (FB TAP = PRE FILTER) o salida del filtro post (POST FILTER)
escritura:  si |tap_L| < 10³⁰ ∧ |tap_R| < 10³⁰:  buf_c[w] = tap_c;  w ← (w + 1) mod 32768
            si no (NaN, ±∞ o desbocado): delay, tono y DC ← 0''')}
{table(["Control", "Rango", "Defecto", "Notas"], [
    ["FB AMOUNT", "0–150 %", "0", "sobre 100 % el bucle es inestable a propósito y lo contiene el tanh; destino de modulación 19 (<code>clamp(AMT + 1.5·m, 0, 1.5)</code>)"],
    ["FB TIME", "0.05–50 ms, logarítmico", "5 ms", "mínimo 1 muestra; destino 20 (modulación en la escala logarítmica)"],
    ["FB TONE", "50 Hz–20 kHz, logarítmico", "6 kHz", "corte del filtro de un polo dentro del bucle"],
    ["FB TAP", "PRE FILTER, POST FILTER", "PRE FILTER", "si el filtro principal está en PRE CHAIN u OFF, ambas tomas coinciden"],
], ncols=(0, 1, 2))}
<p>Con FB AMOUNT = 0 el bucle no se ejecuta: no se lee, no se suma (ni siquiera un 0, que podría cambiar el signo de un
cero) y no se escribe. Como |fb| ≤ 1, la entrada del slot 1 está acotada aunque el bucle se desboque. El delay tiene
32768 muestras por canal, reservadas al crear el motor: alcanzan para 65 ms (50 ms × 1.3 de SPREAD) a 384 kHz; por encima
de esa frecuencia, el tiempo máximo queda recortado por el límite de <code>d</code>. La resincronización (9.7) vacía el
bucle. El bucle no tiene sobremuestreo (18).</p>
<h3>20.2 Seguidor de envolvente (ENV)</h3>
<p>Detector de pico con ataque y caída. Corre en cada muestra, al final de la muestra; su valor se usa como fuente de
modulación a la tasa de la matriz (sección 10, cada 16 muestras).</p>
{eq('''nivel = max(|s_L|, |s_R|) · dB(GAIN),    s = {tap}
si nivel < 10³⁰:  e ← e + (nivel − e) · (nivel > e ? k(ATK) : k(REL))
si no:            e ← 0
ENV = min(e, 1)''')}
{table(["Control", "Rango", "Defecto", "Notas"], [
    ["ENV ATK", "0.1–100 ms", "5 ms", "constante de subida"],
    ["ENV REL", "1–2000 ms", "150 ms", "constante de caída"],
    ["ENV GAIN", "−24 a +24 dB", "0 dB", "ganancia antes del detector; ENV satura en 1"],
    ["ENV TAP", ("VOICE, PRE-FILTER" if gen else "INPUT, PRE-FILTER"), ("PRE-FILTER" if gen else "INPUT"),
     ("el instrumento no tiene entrada de audio: su opción 0 es la envolvente de la voz" if gen
      else "PRE-FILTER sigue la salida de los slots: el carril que modula la cadena con ENV crea automodulación (intencional)")],
    ["SRC (por carril MOD)", "SEQ, ENV, SEQ×ENV", "SEQ", "SEQ = valor del paso con glide (comportamiento anterior)"],
], ncols=(0, 1, 2))}
<p>El seguidor se reinicia a 0 en la resincronización (9.7). La automodulación con PRE-FILTER pasa por la matriz, que
acota cada destino a su rango, de modo que no puede desbordarse.</p>
"""


def s_limit(gen):
    items = [
        "Sin sobremuestreo: aliasing en módulos discontinuos (intencional).",
        ("La realimentación no tiene sobremuestreo: con FB AMOUNT alto y FB TIME corto, el bucle (y su saturación tanh) "
         "genera componentes por encima de fs/2 que se reflejan (aliasing). Es intencional."),
        ("Los destinos de modulación pasaron de 19 a 22 opciones: una automatización de «Mod N Destination» grabada por el "
         "host como valor normalizado en la 0.7 apunta a otro destino en la 0.8. Los proyectos y presets guardan el valor "
         "plano y no se ven afectados."),
        "Las ediciones del patrón en VST3/CLAP no generan historial de deshacer.",
        ("La mutación (MUT &gt; 0) no es determinista. Con reloj libre no hay resincronización y la fase del ruido "
         "depende de las muestras procesadas desde la inicialización (13.4)."),
        ("Cada instancia del plugin mantiene un único juego de estado de procesamiento (ruido, slots, filtro, "
         "secuenciador y envolvente), compartido por todas las notas: no existe estado independiente por nota, "
         "por lo que el instrumento no puede ser polifónico." if gen else
         "Cada instancia del plugin mantiene un único juego de estado de procesamiento (ruido, slots, filtro y "
         "secuenciador) para su par de canales: no hay procesamiento por voz ni por nota."),
    ]
    if gen:
        items += ["Monofónico, prioridad a la última nota.", "PITCH del LFSR limitado a f0 ≤ 4·fs/93."]
    refs = """
<ol class="refs">
  <li>Simper, A. (2013). <i>Linear Trapezoidal Integrated State Variable Filter With Low Noise Optimisation</i>. Cytomic.</li>
  <li>Kellet, P. (2000). <i>Filter to make pink noise from white</i>. Music-DSP Source Code Archive.</li>
  <li>Marsaglia, G. (2003). Xorshift RNGs. <i>Journal of Statistical Software</i>, 8(14).</li>
  <li>Knuth, D. E. (1997). <i>The Art of Computer Programming, Vol. 2</i>. Addison-Wesley.</li>
  <li>Toussaint, G. (2005). The Euclidean Algorithm Generates Traditional Musical Rhythms. <i>BRIDGES</i>.</li>
  <li>NESdev Wiki. <i>APU Noise</i>.</li>
  <li>Zölzer, U. (Ed.) (2011). <i>DAFX: Digital Audio Effects</i>. Wiley.</li>
  <li>Teschner, M. et al. (2003). Optimized Spatial Hashing for Collision Detection of Deformable Objects. <i>VMV</i>. (Constantes del hash.)</li>
  <li>CLAP: <i>CLever Audio Plug-in API</i>, free-audio. · Steinberg: <i>VST 3 SDK</i> 3.8 (MIT).</li>
  <li>NIH-plug, R. van Dijk. ISC.</li>
</ol>"""
    return f"""
<h2 id="i18">18. Limitaciones</h2>
<ul>{"".join("<li>" + i + "</li>" for i in items)}</ul>
<h2 id="i19">19. Referencias</h2>
{refs}
"""


def documento(gen):
    _eq[0] = 0
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    titulo = 'NOISEGEK <span class="gen">GEN</span>' if gen else "NOISEGEK"
    partes = [
        '<div class="part">PARTE I · SISTEMA</div>', s_ident(gen), s_arquitectura(gen), s_numeros(),
        '<div class="part">PARTE II · PROCESAMIENTO DE SEÑAL</div>', s_rng(), s_ruido(gen), s_modulos(), s_filtro(),
        s_salida(gen), s_seq(gen), s_mod(gen), s_orden(gen), (s_voz() if gen else ""), s_fb_env(gen),
        '<div class="part">PARTE III · PLUGIN E INTERFAZ</div>', s_params(gen), s_ui(gen),
        ("" if gen else s_jsfx()), s_build(gen), s_verif(gen), s_limit(gen),
        '<div class="part">APÉNDICE A · CÓDIGO FUENTE DEL NÚCLEO</div>',
        "<p>Código normativo del procesamiento de señal (crate <code>noisegek-dsp</code>), tal como se compila.</p>",
        dsp_source_appendix(),
    ]
    toc = re.findall(r'<h2 id="(i\d+)">([^<]+)</h2>', "".join(partes))
    toc_html = "".join(f'<li><a href="#{a}">{esc(b)}</a></li>' for a, b in toc)
    return f"""<!doctype html>
<html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="author" content="{AUTOR}"><title>{nombre} · Especificación técnica</title>
<style>{CSS}{CSS_EXTRA}</style></head>
<body><div class="wrap">
<header><h1>{titulo}</h1>
<div class="meta">ESPECIFICACIÓN TÉCNICA · v{VERSION} · {FECHA} · {AUTOR}</div></header>
<div class="abstract"><b>ALCANCE</b><p>Documentación de ingeniería completa de {nombre}: sistema, procesamiento de señal
con todas sus ecuaciones, constantes y semillas, orden de ejecución, parámetros y su normalización, formato del estado,
concurrencia, interfaz, {"" if gen else "implementación JSFX, "}construcción, parches y verificación. Incluye el código
fuente del núcleo como apéndice normativo.</p></div>
<p class="note">Numeración: ambos documentos (NOISEGEK y NOISEGEK GEN) comparten la numeración de secciones; la 12 (Voz) es exclusiva de GEN y la 15 (JSFX) es exclusiva del efecto. La 20 (Realimentación y seguidor de envolvente, desde la 0.8) se ubica en la Parte II, después de la 12, para no renumerar las demás.</p>
<nav class="toc"><ol>{toc_html}<li><a href="#apA">Apéndice A · Código fuente</a></li></ol></nav>
{"".join(partes).replace('<div class="part">APÉNDICE A', '<div class="part" id="apA">APÉNDICE A')}
<footer>{nombre} · Especificación técnica · v{VERSION}<br>{COPY_SPEC}</footer>
</div></body></html>
"""


if __name__ == "__main__":
    os.makedirs(os.path.join(ROOT, "dist"), exist_ok=True)
    for gen, fname in (((True, "NOISEGEK GEN - Especificacion tecnica.html"),) if True else
                       ((False, "NOISEGEK - Especificacion tecnica.html"),)):
        path = os.path.join(ROOT, "dist", fname)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(documento(gen))
        print("ok:", path)
