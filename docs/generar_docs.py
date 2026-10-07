# -*- coding: utf-8 -*-
"""
NOISEGEK · por @ahrsml
Genera la documentación de la carpeta de distribución
    dist/NOISEGEK GEN (instrumento)/
con MANUAL.html (referencia técnica), README.md (técnico),
CHANGELOG.md, licencias/ y plugin/ (copias de vst/dist).

Uso:  python generar_docs.py        (lo llama build.ps1 al final)
Las carpetas generadas son autocontenidas: no deben mencionar el código fuente.
Fuente única: si cambia el comportamiento del DSP, se actualiza aquí y
ambos manuales quedan consistentes.
"""
import html
import os
import shutil

VERSION = "0.8.1"
FECHA = "octubre de 2026"
AUTOR = "Adolfo Rosas (@ahrsml)"
VENDOR = "@ahrsml"   # nombre de fabricante que muestra el host
COPY_DOC = "© 2026 Adolfo Rosas (@ahrsml). Todos los derechos reservados."

HERE = os.path.dirname(os.path.abspath(__file__))
VST = os.path.dirname(HERE)
ROOT = VST
DIR_FX = os.path.join(VST, "dist", "NOISEGEK (efecto)")
DIR_GEN = os.path.join(VST, "dist", "NOISEGEK GEN (instrumento)")

# -----------------------------------------------------------------------------
# Hoja de estilo (paleta NOISEGEK, tipografía legible para lectura larga)
# -----------------------------------------------------------------------------
CSS = r"""
:root { --void:#0A0A0A; --iron:#1A1A1A; --white:#E6E6E6; --red:#FF0033; --shadow:#4D4D4D; --dim:#A8A8A8; }
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body { margin:0; background:var(--void); color:var(--white);
       font:18px/1.7 "Segoe UI", system-ui, -apple-system, sans-serif; }
a { color:var(--white); text-decoration-color:var(--red); text-underline-offset:3px; }
a:hover { color:var(--red); }
.wrap { max-width:960px; margin:0 auto; padding:40px 24px 120px; }
h1,h2,h3 { font-family:Impact,"Anton","Arial Black",sans-serif; font-weight:normal; letter-spacing:.02em; line-height:1.15; }
h1 { font-size:58px; margin:0; }
h1 .gen { color:var(--red); }
h2 { font-size:32px; margin:64px 0 12px; padding-top:14px; border-top:2px solid var(--shadow); position:relative; }
h2::before { content:""; position:absolute; left:0; top:-2px; width:64px; height:2px; background:var(--red); }
h3 { font-size:23px; margin:34px 0 8px; }
h4 { font-size:18px; margin:24px 0 6px; font-family:"Courier New",monospace; }
p { margin:10px 0; text-align:justify; hyphens:auto; }
code, .mono { font-family:"Courier New","Courier Prime",monospace; font-weight:bold; }
code { background:var(--iron); padding:1px 6px; border:1px solid var(--shadow); font-size:.9em; }
.meta { font-family:"Courier New",monospace; font-weight:bold; color:var(--dim); margin-top:8px; }
.abstract { background:var(--iron); border-left:3px solid var(--red); padding:16px 22px; margin:28px 0; }
.abstract b { font-family:Impact,"Anton",sans-serif; font-weight:normal; font-size:20px; letter-spacing:.04em; }
nav.toc { background:var(--iron); padding:16px 24px; margin:28px 0; }
nav.toc ol { margin:0; padding-left:24px; columns:2; column-gap:32px; }
nav.toc li { break-inside:avoid; }
.eq { font-family:"Courier New",monospace; font-weight:bold; background:var(--iron); border-left:3px solid var(--shadow);
      padding:12px 18px; margin:14px 0; white-space:pre; overflow-x:auto; font-size:16px; line-height:1.55; }
.eq .n { float:right; color:var(--dim); font-weight:normal; }
.fig { font-family:"Courier New",monospace; font-weight:bold; background:var(--iron); padding:16px 20px; margin:16px 0;
       white-space:pre; overflow-x:auto; font-size:15px; line-height:1.5; border-left:3px solid var(--red); }
.cap { color:var(--dim); font-size:15px; margin:-8px 0 18px; }
table { width:100%; border-collapse:collapse; margin:16px 0; font-size:16px; }
th, td { text-align:left; vertical-align:top; padding:8px 11px; border-bottom:1px solid var(--shadow); }
th { background:var(--iron); font-family:"Courier New",monospace; font-weight:bold; }
td.n, th.n { white-space:nowrap; }
.note { border-left:3px solid var(--shadow); padding:4px 16px; color:var(--dim); margin:12px 0; }
ol.refs li { margin:8px 0; }
footer { margin-top:80px; padding-top:16px; border-top:2px solid var(--shadow); color:var(--dim); font-size:15px; }
@media (max-width:700px) { body{font-size:17px} h1{font-size:42px} nav.toc ol{columns:1} p{text-align:left} }
@media print { body{background:#fff;color:#000} a{color:#000} .abstract,nav.toc,.eq,.fig,code,th{background:#f2f2f2;color:#000} }
"""

# -----------------------------------------------------------------------------
# Utilidades
# -----------------------------------------------------------------------------
_eq_counter = [0]


def eq(text):
    """Ecuación numerada (texto monoespaciado)."""
    _eq_counter[0] += 1
    return f'<div class="eq"><span class="n">({_eq_counter[0]})</span>{html.escape(text.strip(chr(10)))}</div>'


def fig(text, caption):
    return f'<div class="fig">{html.escape(text.strip(chr(10)))}</div><p class="cap">{caption}</p>'


def table(headers, rows, ncols=()):
    h = "".join(f'<th{" class=n" if i in ncols else ""}>{c}</th>' for i, c in enumerate(headers))
    body = ""
    for r in rows:
        body += "<tr>" + "".join(f'<td{" class=n" if i in ncols else ""}>{c}</td>' for i, c in enumerate(r)) + "</tr>"
    return f"<table><tr>{h}</tr>{body}</table>"


# -----------------------------------------------------------------------------
# Secciones del manual (gen = True para el instrumento)
# -----------------------------------------------------------------------------

def sec_intro(gen):
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    tipo = "instrumento virtual (generador)" if gen else "procesador de efectos de audio"
    s = f"""
<h2 id="s1">1. Introducción</h2>
<h3>1.1 Objeto del documento</h3>
<p>El presente documento constituye la referencia técnica de <b>{nombre}</b> versión {VERSION}, un {tipo}
desarrollado por {AUTOR}. Describe de forma exhaustiva la arquitectura de procesamiento de señal, la definición
matemática de cada módulo, el comportamiento del secuenciador, el rango y la escala de cada parámetro, y las
convenciones de la interfaz. El documento es descriptivo: no propone usos musicales.</p>
<h3>1.2 Alcance</h3>
<p>Se documentan los formatos VST3 y CLAP.
{"" if gen else "El efecto se incluye además como JSFX para REAPER (<code>plugin/NOISEGEK.jsfx</code>), cuyo comportamiento de señal es idéntico; las diferencias de interfaz se indican en la sección 11.3."}
</p>
<h3>1.3 Convenciones y notación</h3>
{table(["Símbolo", "Significado"], [
    ["<code>fs</code>", "frecuencia de muestreo del host, en Hz"],
    ["<code>x[n]</code>, <code>y[n]</code>", "muestra de entrada y de salida en el instante <code>n</code>"],
    ["<code>A</code>, <code>B</code>", "parámetros normalizados de un módulo, en el intervalo [0, 1]"],
    ["<code>clamp(v, a, b)</code>", "<code>min(max(v, a), b)</code>"],
    ["<code>dB → lineal</code>", "<code>g = 10^(dB/20)</code>"],
    ["<code>U(a, b)</code>", "variable aleatoria uniforme en [a, b)"],
    ["negra (QN)", "unidad de tiempo musical: una negra (quarter note)"],
    ["<code>k(τ)</code>", "coeficiente de un filtro de un polo con constante de tiempo τ: <code>k = 1 − exp(−1/(τ·fs))</code>"],
], ncols=(0,))}
<p>Las magnitudes internas se calculan en coma flotante de 64 bits; la entrada y la salida del plugin son de 32 bits.
Los nombres de los controles se escriben en <b>MAYÚSCULAS</b>, tal como aparecen en la interfaz.</p>
"""
    return s


def sec_formatos(gen):
    if gen:
        io = table(["Propiedad", "Valor"], [
            ["Tipo", "Instrumento (VST3: <code>Instrument|Synth</code>; CLAP: <code>instrument, synthesizer, stereo</code>)"],
            ["Entrada de audio", "ninguna"],
            ["Salida de audio", "estéreo (2 canales)"],
            ["Entrada de eventos", "notas MIDI (note on / note off / choke), con precisión de muestra"],
            ["Latencia", "0 muestras"],
            ["Frecuencia de muestreo", "cualquiera admitida por el host"],
            ["Plataforma", "Windows x86-64"],
            ["Identificadores", "CLAP <code>com.ahrsml.noisegek-gen</code> · VST3 <code>NOISEGEKGENahrsm</code>"],
        ], ncols=(0,))
    else:
        io = table(["Propiedad", "Valor"], [
            ["Tipo", "Efecto (VST3: <code>Fx|Distortion|Filter</code>; CLAP: <code>audio-effect, stereo, distortion, filter</code>)"],
            ["Entrada / salida", "estéreo → estéreo, o mono → mono"],
            ["Latencia", "0 muestras"],
            ["Frecuencia de muestreo", "cualquiera admitida por el host"],
            ["Plataforma", "Windows x86-64 (VST3, CLAP); JSFX: REAPER"],
            ["Identificadores", "CLAP <code>com.ahrsml.noisegek</code> · VST3 <code>NOISEGEKahrsml01</code>"],
        ], ncols=(0,))
    nota = ("Al no tener entrada de audio, el instrumento solo produce señal mientras la envolvente de voz es distinta de cero "
            "(sección 10). El generador de ruido es la única fuente.") if gen else (
            "El plugin informa al host que debe seguir procesando aunque la entrada sea silencio "
            "(<i>keep alive</i>), porque el generador de ruido interno puede producir señal sin entrada.")
    return f"""
<h2 id="s2">2. Formatos y especificaciones</h2>
{io}
<p>{nota}</p>
"""


def sec_instalacion(gen):
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    bs = chr(92)  # barra invertida (Python 3.6 no la admite dentro de f-strings)
    ruta = bs.join(["C:", "Program Files", "Common Files"])
    tabla_rutas = table(["Formato", "Ruta estándar del sistema"], [
        ["VST3", "<code>" + ruta + bs + "VST3" + bs + nombre + ".vst3</code>"],
        ["CLAP", "<code>" + ruta + bs + "CLAP" + bs + nombre + ".clap</code>"],
    ], ncols=(0,))
    jsfx = "" if gen else (
        "<h3>3.2 JSFX (REAPER)</h3><p>Copiar <code>plugin" + bs + "NOISEGEK.jsfx</code> a la carpeta <code>Effects</code> de "
        "REAPER (<i>Options → Show REAPER resource path</i>) y actualizar la lista de efectos (F5 en el explorador de FX).</p>")
    return f"""
<h2 id="s3">3. Instalación</h2>
<h3>3.1 VST3 y CLAP</h3>
<p>Los archivos del plugin se encuentran en la subcarpeta <code>plugin\\</code> de este directorio
(<code>{nombre}.vst3</code>, que es una carpeta con formato de paquete, y <code>{nombre}.clap</code>).
La instalación consiste en copiarlos a las rutas estándar del sistema:</p>
{tabla_rutas}
<p><code>INSTALAR.bat</code> realiza esa copia (Windows solicita permiso de administrador) y <code>DESINSTALAR.bat</code>
elimina los archivos instalados. Tras la instalación, el host debe volver a escanear sus plugins.</p>
{jsfx}
<p class="note">FL Studio 20 solo detecta plugins VST3 en <code>C:\\Program Files\\Common Files\\VST3\\</code>; las rutas
de búsqueda personalizadas de ese host se aplican únicamente a VST2. FL Studio 20 no admite CLAP.
{"En FL Studio el instrumento se carga en el <i>Channel Rack</i>." if gen else "En FL Studio el efecto se carga en un slot del <i>Mixer</i>."}</p>
"""


def sec_arquitectura(gen):
    if gen:
        diagrama = """
                ruido ──×env(PRE)── [GATE pre] ── [FILTER pre] ── DRIVE ── S1 ── S2 ── S3 ── S4 ── [FILTER post] ── WET
                                                                                                             │
   WET·MIX ── [GATE post] ── ×env(POST) ── PAN ── OUT ── DC blocker ── limitador ── salida
"""
        texto = ("En el instrumento la señal seca es nula (no hay entrada), de modo que con MIX &lt; 100 % el nivel "
                 "de salida se reduce proporcionalmente. La envolvente de voz (sección 10) se aplica antes de la cadena "
                 "(<code>PRE FX</code>) o después de la mezcla (<code>POST OUT</code>), según el parámetro ENV.")
    else:
        diagrama = """
  entrada ─┬──────────────────────────────────────────────────────────────────────────────── DRY ──┐
           └─(+ ruido)─ [GATE pre] ─ [FILTER pre] ─ DRIVE ─ S1 ─ S2 ─ S3 ─ S4 ─ [FILTER post] ─ WET ──┤
                                                                                                  MIX
           [GATE post] ─ PAN ─ OUT ─ DC blocker ─ limitador −0.3 dBFS ─ salida
"""
        texto = ("La rama seca (DRY) contiene exclusivamente la señal de entrada; el ruido interno y el gate en "
                 "posición PRE solo afectan a la rama procesada (WET).")
    return f"""
<h2 id="s4">4. Arquitectura de la señal</h2>
<p>El procesamiento es muestra a muestra, con estado independiente por canal. El orden de las etapas es fijo; las
etapas entre corchetes ocupan una de dos posiciones posibles según su selector POS.</p>
{fig(diagrama, "Figura 1. Diagrama de flujo de la señal.")}
<p>{texto}</p>
<p>Para cada canal, la mezcla y las etapas finales se definen como:</p>
{eq('''wet = S4(S3(S2(S1( drive · filtro_pre( gate_pre( x + ruido ) ) + fb ))))
y   = dry + (filtro_post(wet) − dry) · MIX''')}
<p><code>fb</code> es la señal del bucle de realimentación (sección 6.5); con FB AMOUNT = 0 el bucle está
desconectado y <code>fb</code> no se suma.</p>
{eq('''DC blocker (fc ≈ 10 Hz):   y[n] = x[n] − x[n−1] + R · y[n−1],   R = 1 − 2π·10/fs''')}
{eq('''limitador:   y = clamp(y, −L, L),   L = 10^(−0.3/20) ≈ 0.9661''')}
<p>El limitador es un recorte duro de seguridad: ninguna muestra de salida supera −0.3 dBFS.</p>
<h3>4.1 Suavizado y transiciones</h3>
<p>Todos los parámetros continuos se suavizan con un filtro de un polo de constante de tiempo τ = 4 ms. La
modulación del secuenciador se recalcula cada 16 muestras (tasa de control) y atraviesa el mismo suavizado.
El cambio de módulo de un slot y su encendido o apagado se realizan con un fundido lineal de 10 ms; en el cambio de
módulo el slot se funde a paso directo, se reinicia el estado interno del nuevo módulo y se funde de vuelta.</p>
{eq('''p[n] = p[n−1] + (p_objetivo − p[n−1]) · k(4 ms)''')}
"""


def sec_source(gen):
    return f"""
<h2 id="s5">5. SOURCE: generador de ruido</h2>
<p>El generador produce una señal estéreo que se suma a la entrada antes de la cadena. Su nivel es
<code>LEVEL</code> (−60 a 0 dB; el valor −60 dB equivale a silencio). El parámetro <code>CHARACTER</code>
(0–1) tiene una función distinta según el tipo.</p>
<h3>5.1 Fuente uniforme</h3>
<p>Cada canal dispone de un generador congruencial lineal independiente; de él derivan todos los tipos:</p>
{eq('''s[n] = (1664525 · s[n−1] + 1013904223) mod 2^32
w[n] = 2 · s[n] / 2^32 − 1          ∈ [−1, 1)''')}
<h3>5.2 Tipos</h3>
{table(["Tipo", "Definición", "Función de CHARACTER (c)"], [
    ["OFF", "sin ruido", "—"],
    ["WHITE", "<code>w[n]</code>; densidad espectral plana", "ancho estéreo: <code>R = L + (R<sub>indep</sub> − L)·c</code>"],
    ["PINK", "filtro de Paul Kellet (versión refinada, 7 polos) sobre <code>w</code>, escala 0.11; pendiente −3 dB/octava", "ancho estéreo (igual que WHITE)"],
    ["BROWN", "integrador con fuga: <code>y[n] = (1 − λ)·y[n−1] + g·w[n]</code>; pendiente −6 dB/octava por encima de la frecuencia de fuga",
     "<code>λ = 0.0002 + 0.05·(1 − c)²</code>, <code>g = √(0.6·λ)</code> (mantiene RMS ≈ 0.3)"],
    ["DUST", "proceso de impulsos de Bernoulli: con probabilidad <code>p</code> por muestra se emite un impulso de amplitud <code>U(−1, 1)</code>; entre impulsos <code>y[n] = 0.45·y[n−1]</code>",
     "densidad: <code>p = (2 + 20000·c³) / fs</code> (2 a 20002 impulsos/s)"],
    ["LFSR", "registro de desplazamiento con realimentación lineal de 15 bits; realimentación <code>b0 ⊕ b1</code> (período 32767); salida ±0.7",
     "frecuencia de reloj: <code>f = 50 · 2^(9.5·c)</code> Hz (≈ 50 Hz a 36 kHz; máx. 4 pasos por muestra)"],
], ncols=(0,))}
{"<p>En el instrumento, el modo PITCH reemplaza el reloj y la toma del LFSR (sección 10.5).</p>" if gen else ""}
"""


MODULOS = [
    ["OFF", "<code>y = x</code>", "—", "—"],
    ["HARD CLIP", "<code>y = clamp(x, −t, t) / t</code>", "<code>t = max(A, 0.01)</code>", "sin uso"],
    ["SOFT CLIP", "<code>y = tanh(d·x) / tanh(d)</code>", "<code>d = 1 + 19A</code>", "sin uso"],
    ["FOLDBACK", "mientras <code>|x| &gt; t</code> y queden pliegues: <code>x ← ±2t − x</code>; luego <code>y = clamp(x, −t, t) / t</code>",
     "<code>t = max(A, 0.02)</code>", "pliegues máximos <code>N = round(1 + 15B)</code>"],
    ["BITCRUSH", "<code>y = ⌊x/q + 0.5⌋ · q</code>, <code>q = 2^−(b−1)</code>", "<code>b = 1 + 15A²</code> bits", "sin uso"],
    ["DOWNSAMPLE", "retención de orden cero: toma una muestra cada <code>N</code> y la mantiene", "<code>N = round(1 + 63A²)</code>", "sin uso"],
    ["RECTIFIER", "<code>r = max(x, 0) + m·max(−x, 0)</code>; <code>y = r + (x − r)·B</code>", "<code>m = 1</code> si <code>A ≥ 0.5</code> (onda completa), si no 0 (media onda)", "proporción de señal original"],
    ["TABLE SHAPER", "<code>s = T<sub>c</sub>(x) + (T<sub>s</sub>(x) − T<sub>c</sub>(x))·B</code>; <code>y = x + (s − x)·A</code>", "intensidad", "suavizado de la curva"],
    ["TRIGGER S/H", "disparo si <code>sgn</code> cambia o si <code>|x − h| &gt; θ</code>; en el disparo, <code>h ← x</code> con probabilidad <code>1 − B</code>; <code>y = h</code>",
     "<code>θ = A²</code>", "probabilidad de ignorar el disparo"],
    ["RING MOD", "<code>y = x · o</code>, <code>o = s + (sgn(s) − s)·B</code>, <code>s = sin(2π·φ)</code>, <code>φ ← φ + f/fs</code>", "<code>f = 20 · 250^A</code> Hz (20 Hz–5 kHz)", "forma: seno (0) a cuadrada (1)"],
    ["SLEW LIMITER", "<code>y[n] = y[n−1] + clamp(x − y[n−1], −δ<sub>b</sub>, δ<sub>s</sub>)</code>",
     "<code>r = 5 · 10^(4A) / fs</code> por muestra", "asimetría: <code>B &lt; 0.5</code> reduce δ<sub>s</sub>, <code>B &gt; 0.5</code> reduce δ<sub>b</sub>, hasta ×0.01"],
]


def sec_chain(gen):
    return f"""
<h2 id="s6">6. CHAIN: cadena no lineal</h2>
<p>La cadena consta de cuatro slots en serie (S1 → S4). Cada slot aloja uno de once módulos y dos parámetros
normalizados, A y B, cuya interpretación depende del módulo. La interfaz muestra en cada caso el nombre y el valor
físico del parámetro (por ejemplo <code>THRESH</code>, <code>FOLDS</code>, <code>FREQ</code>); los parámetros que un
módulo no utiliza se representan atenuados.</p>
<p>La ganancia de entrada a la cadena es <code>DRIVE</code> (−12 a +36 dB). Ningún módulo incorpora sobremuestreo:
las no linealidades con discontinuidades en la función o en su derivada (HARD CLIP, FOLDBACK, RECTIFIER, BITCRUSH,
DOWNSAMPLE y RING MOD con forma cuadrada) generan componentes por encima de <code>fs/2</code> que se reflejan
(<i>aliasing</i>). Este comportamiento es intencional.</p>
<h3>6.1 Definición de los módulos</h3>
{table(["Módulo", "Función de transferencia", "A", "B"], MODULOS, ncols=(0,))}
<h3>6.2 Tablas del TABLE SHAPER</h3>
<p>Se generan dos tablas de 1024 puntos a partir de <code>T.SEED</code> (0–9999): una cruda
<code>T<sub>c</sub></code> con valores <code>U(−1, 1)</code> producidos por xorshift32 (semilla <code>T.SEED + 1</code>,
16 iteraciones de calentamiento) y una suavizada <code>T<sub>s</sub></code>, media móvil de radio 48 de la cruda.
Ambas se desplazan para que <code>T(0) = 0</code> y se normalizan a pico 1. La lectura usa interpolación lineal sobre
<code>x</code> acotado a [−1, 1]. La misma semilla produce siempre las mismas tablas.</p>
<h3>6.3 Estado de los módulos</h3>
<p>DOWNSAMPLE, TRIGGER S/H, RING MOD y SLEW LIMITER conservan estado por slot y canal. El TRIGGER S/H dispone de un
generador xorshift32 propio por slot y canal, con semillas fijas, por lo que su secuencia de decisiones es
determinista.</p>
<h3>6.4 Ganancias de la cadena</h3>
{table(["Control", "Rango", "Definición"], [
    ["DRIVE", "−12 a +36 dB", "ganancia aplicada a la rama WET antes de S1"],
    ["MIX", "0–100 %", "proporción WET en la mezcla (ec. 1)"],
    ["OUT", "−24 a +12 dB", "ganancia de salida, antes del DC blocker"],
    ["T.SEED", "0–9999", "semilla de las tablas del TABLE SHAPER"],
], ncols=(0, 1))}
<h3>6.5 Realimentación (FEEDBACK)</h3>
<p>La salida de la cadena vuelve a la entrada de S1 a través de un retardo fraccional (interpolación lineal, mínimo una
muestra), un filtro paso bajo de un polo, un bloqueador de continua, una ganancia y una saturación tanh:</p>
{eq('''fb[n] = tanh( AMOUNT · DC( LP_TONE( toma[n − d] ) ) ),     d = TIME · fs / 1000 muestras
LP_TONE: t[n] = t[n−1] + (x − t[n−1]) · (1 − e^(−2π·TONE/fs))''')}
{table(["Control", "Rango", "Definición"], [
    ["AMT", "0–150 %", "ganancia del bucle; por encima de 100 % el bucle es inestable y la saturación tanh lo mantiene acotado (|fb| ≤ 1). Con 0 % el bucle está desconectado"],
    ["TIME", "0.05–50 ms (escala logarítmica)", "retardo del bucle. Sus variaciones (modulación) desplazan la altura de la señal realimentada, como en un retardo modulado"],
    ["TONE", "50 Hz–20 kHz (escala logarítmica)", "corte del filtro de un polo dentro del bucle"],
    ["TAP", "PRE FILTER, POST FILTER", "punto de toma: salida de S4 o salida del filtro en posición POST CHAIN"],
], ncols=(0, 1))}
<p>AMOUNT y TIME son destinos de modulación (sección 9.9). Un valor no finito en la toma vacía el bucle. Al conectarse,
el bucle comienza vacío. El bucle no tiene sobremuestreo: con AMOUNT alto y TIME corto produce aliasing.</p>
"""


def sec_filter(gen):
    return f"""
<h2 id="s7">7. FILTER</h2>
<p>Filtro de variable de estado con integración trapezoidal (Simper, 2013), estable bajo modulación rápida de sus
coeficientes. Ocupa la posición <code>POST CHAIN</code> (entre S4 y la mezcla) o <code>PRE CHAIN</code> (antes de
DRIVE).</p>
{eq('''g  = tan(π · fc / fs),        fc ≤ 0.45 · fs
k  = 2 − 1.96 · RESO           (RESO ∈ [0, 1])
a1 = 1 / (1 + g·(g + k)),   a2 = g·a1,   a3 = g·a2''')}
{eq('''v3 = x − ic2
v1 = a1·ic1 + a2·v3
v2 = ic2 + a2·ic1 + a3·v3
ic1 ← 2·v1 − ic1,   ic2 ← 2·v2 − ic2
LOWPASS = v2,   BANDPASS = v1,   HIGHPASS = x − k·v1 − v2''')}
{table(["Control", "Rango", "Notas"], [
    ["MODE", "OFF, LOWPASS, BANDPASS, HIGHPASS", "OFF omite la etapa"],
    ["POS", "POST CHAIN, PRE CHAIN", "—"],
    ["CUTOFF", "20 Hz – 20 kHz", "escala logarítmica para la modulación (sección 9.9)"],
    ["RESO", "0–100 %", "a 100 %, <code>k = 0.04</code> (Q = 25)"],
    ["SCREAM", "OFF, ON", "ON: <code>k = 2 − 2.04·RESO</code> (k = −0.04 a 100 %) y estados saturados por tanh"],
], ncols=(0,))}
<h3>7.1 SCREAM</h3>
<p>Con SCREAM activo, la resonancia puede anular el amortiguamiento del filtro y llevarlo ligeramente por debajo de cero;
los dos integradores pasan por una saturación tanh, que limita su amplitud:</p>
{eq('''k = 2 − 2.04 · RESO
ic1 ← tanh(2·v1 − ic1),   ic2 ← tanh(2·v2 − ic2)''')}
<p>Con RESO cercano a 100 %, el filtro sostiene una oscilación cercana a CUTOFF aunque la entrada sea silencio; la
saturación la mantiene en un ciclo límite de amplitud acotada. Con SCREAM inactivo, el filtro es el descrito arriba.</p>
"""


def sec_seq(gen):
    return f"""
<h2 id="s8">8. Etapa final: paneo</h2>
<p>El paneo no tiene control propio; solo existe como destino de modulación (<code>PAN</code>, sección 9.9). Se aplica
una ley de balance que no atenúa el centro:</p>
{eq('''L ← L · min(1, 1 − p),    R ← R · min(1, 1 + p),    p ∈ [−1, 1]''')}
<h3>8.1 SPREAD (STEREO)</h3>
<p>SPREAD (0–100 %) introduce diferencias entre los canales en el canal R; con 0 % ambos canales se procesan igual:</p>
{table(["Elemento", "Canal R con SPREAD = s"], [
    ["RING MOD", "el oscilador se lee con la fase adelantada <code>0.25·s</code> ciclos"],
    ["DOWNSAMPLE", "la muestra se retiene <code>⌊s·N/2⌋</code> muestras después que en L (hasta medio ciclo)"],
    ["FEEDBACK", "el retardo del canal R es <code>TIME · (1 + 0.3·s)</code>"],
    ["TRIGGER S/H", "sin cambio: cada canal ya tiene su propio generador aleatorio"],
], ncols=(0,))}
<p>SPREAD es un destino de modulación (sección 9.9).</p>

<h2 id="s9">9. SEQUENCER</h2>
<p>El secuenciador consta de cuatro carriles de 16 pasos: <b>GATE</b> (carril 0) y <b>MOD 1–3</b> (carriles 1–3).
Cada paso almacena un valor <code>v ∈ [0, 1]</code>. Cada carril tiene velocidad, longitud, dirección, probabilidad y
mutación propias; los carriles MOD tienen además destino, profundidad y glide.</p>

<h3>9.1 Reloj</h3>
<p>El tiempo del secuenciador es una posición <code>q</code> medida en negras.</p>
{table(["CLOCK", "Comportamiento"], [
    ["AUTO", "con el transporte en reproducción, <code>q</code> sigue la posición del host; detenido, avanza libremente al tempo del proyecto"],
    ["HOST ONLY", "sigue la posición del host; con el transporte detenido, el secuenciador se congela y la envolvente del gate tiende a 1"],
    ["FREE RUN", "avanza siempre al tempo del proyecto, ignorando la posición"],
], ncols=(0,))}
<p>En modo sincronizado, <code>q</code> se toma del host al inicio de cada bloque y avanza por muestra con un
incremento calibrado: si la reproducción es continua, se usa el avance real entre bloques; si difiere más de un 50 %
del valor nominal (salto, bucle), se usa el nominal.</p>
<p>Resincronización: con el reloj sincronizado, al iniciar la reproducción y cada vez que la posición del host salta más
de 64 muestras respecto de la prevista (búsqueda, vuelta de un bucle), el generador de ruido, los slots, el filtro y los
carriles vuelven a su estado inicial y el secuenciador se evalúa en la posición nueva. Los carriles MOD comienzan en el
valor de su primer paso y el gate parte de 0. Con FREE RUN, o con AUTO y el transporte detenido, no hay
resincronización.</p>
{eq('''Δq_nominal = BPM / (60 · fs)
Δq = (q_bloque − q_bloque_anterior) / N_anterior   si |Δq − Δq_nominal| < 0.5·Δq_nominal''')}

<h3>9.2 Velocidad (RATE)</h3>
<p>Cada carril convierte <code>q</code> en un índice de paso según su número de pasos por negra <code>spq</code>.
Los valores en compases dependen del compás del proyecto: <code>Q<sub>compás</sub> = num · 4 / den</code>.</p>
{table(["RATE", "spq", "RATE", "spq"], [
    ["8 BAR", "1 / (8·Q<sub>c</sub>)", "1/8T", "1.5"],
    ["4 BAR", "1 / (4·Q<sub>c</sub>)", "1/8", "2"],
    ["2 BAR", "1 / (2·Q<sub>c</sub>)", "1/16T", "3"],
    ["1 BAR", "1 / Q<sub>c</sub>", "1/16", "4"],
    ["1/2", "0.5", "1/32", "8"],
    ["1/4", "1", "1/64", "16"],
], ncols=(0, 2))}

<h3>9.3 Swing</h3>
<p>Los pasos se agrupan por pares; el paso impar de cada par comienza en la fracción <code>b</code> del par:</p>
{eq('''t = (q − q0) · spq,    par = ⌊t/2⌋,    φ = t − 2·par,    b = SWING / 50  ∈ [1, 1.5]
paso n = 2·par      si φ < b,   fase dentro del paso = φ / b
paso n = 2·par + 1  si φ ≥ b,   fase dentro del paso = (φ − b) / (2 − b)''')}
<p><code>q0</code> es el origen del secuenciador ({"desplazado en cada nota cuando RETRIG está activo, sección 10.6" if gen else "siempre 0 en el efecto"}).
SWING = 50 % produce pasos iguales; 66.7 % corresponde a la proporción de tresillo.</p>

<h3>9.4 Dirección</h3>
<p>Cada vez que el contador absoluto <code>n</code> cambia se determina el índice <code>i</code> del paso que suena,
con <code>L</code> = STEPS (1–16):</p>
{table(["Dirección", "Índice"], [
    ["FWD", "<code>i = n mod L</code>"],
    ["BWD", "<code>i = L − 1 − (n mod L)</code>"],
    ["PINGPONG", "<code>p = n mod (2L − 2)</code>; <code>i = p</code> si <code>p &lt; L</code>, si no <code>2L − 2 − p</code> (L = 1 ⇒ i = 0)"],
    ["RANDOM", "<code>i = ⌊h(n, carril, 1) · L⌋</code>"],
    ["DRUNK", "<code>i = (i<sub>anterior</sub> ± 1) mod L</code>, con −1 si <code>h(n, carril, 2) &lt; 0.5</code>"],
], ncols=(0,))}
<p>Los pasos con índice ≥ L no se reproducen.</p>

<h3>9.5 Función de azar determinista</h3>
<p>RANDOM, DRUNK y la probabilidad usan una función hash de la posición, no un generador con estado. Con el reloj
sincronizado, una misma posición produce siempre el mismo resultado; <code>SEED</code> (0–9999) selecciona otra
secuencia.</p>
{eq('''s = (73856093·n + 19349663·carril + 83492791·sal + 2654435·SEED + 1) mod 2^32   (s = 0 ⇒ s = 1)
s ← xorshift32(s) tres veces;      h = s / 2^32  ∈ [0, 1)''')}
{eq('''xorshift32:  s ^= s << 13;  s ^= s >> 17;  s ^= s << 5     (Marsaglia, 2003)''')}

<h3>9.6 Probabilidad (PRB)</h3>
<p>Un paso se <i>dispara</i> si <code>h(n, carril, 3) · 100 &lt; PRB</code>. Si no se dispara, el carril GATE emite
0 (silencio) y los carriles MOD conservan su valor anterior.</p>

<h3>9.7 Mutación (MUT)</h3>
<p>Al dispararse un paso, con probabilidad <code>MUT / 100</code> su valor almacenado se reemplaza por
<code>clamp(v + U(−0.3, 0.3), 0, 1)</code>. La mutación modifica el patrón de forma permanente, el cambio se guarda con el
proyecto y, a diferencia del resto del secuenciador, no es determinista.</p>

<h3>9.8 Glide (carriles MOD)</h3>
<p>El valor de un carril MOD sigue al valor del paso con un filtro de un polo:</p>
{eq('''T_paso = 60 / (BPM · spq)          τ = max(0.5 ms, GLD · T_paso / 2)''')}
<p>Con GLD = 0 el cambio es prácticamente instantáneo (τ = 0.5 ms); con GLD = 100 % alcanza ≈ 86 % del salto al final
del paso.</p>

<h3>9.9 Matriz de modulación</h3>
<p>Cada carril MOD envía <code>v</code> a un destino con profundidad <code>D ∈ [−1, 1]</code> (DEP). El selector SRC del
carril define <code>v</code>: el valor del paso con glide (SEQ), el seguidor de envolvente (ENV, sección 9.13) o su
producto (SEQ×ENV). Las contribuciones de varios carriles al mismo destino se suman. La modulación se evalúa cada 16
muestras.</p>
{table(["Destino", "Efecto"], [
    ["S1…S4 A/B", "<code>A' = clamp(A + D·v, 0, 1)</code> (ídem B); el menú muestra el nombre físico del parámetro"],
    ["DRIVE", "<code>DRIVE' = clamp(DRIVE + 48·D·v, −24, 48)</code> dB"],
    ["MIX", "<code>MIX' = clamp(MIX + D·v, 0, 1)</code>"],
    ["CUTOFF", "<code>u = ln(fc/20)/ln(1000)</code>; <code>fc' = 20 · 1000^clamp(u + D·v, 0, 1)</code>"],
    ["RESO", "<code>RESO' = clamp(RESO + D·v, 0, 1)</code>"],
    ["NOISE LVL", "<code>LEVEL' = LEVEL + 60·D·v</code> dB (≤ 0; −60 dB = silencio)"],
    ["PAN", "<code>p = clamp(D·(2v − 1), −1, 1)</code> (bipolar, centrado en v = 0.5)"],
    ["S1…S4 ON/OFF", "slot activo si <code>v ≥ 0.5</code> (D &gt; 0) o si <code>v &lt; 0.5</code> (D &lt; 0); sin efecto con D = 0"],
    ["FB AMOUNT", "<code>AMT' = clamp(AMT + 1.5·D·v, 0, 1.5)</code>"],
    ["FB TIME", "<code>u = ln(TIME/0.05)/ln(1000)</code>; <code>TIME' = 0.05 · 1000^clamp(u + D·v, 0, 1)</code> ms"],
    ["SPREAD", "<code>SPREAD' = clamp(SPREAD + D·v, 0, 1)</code>"],
], ncols=(0,))}

<h3>9.10 GATE</h3>
<p>El carril GATE controla un amplificador. Su objetivo es el valor del paso mientras el paso esté disparado y su fase
sea menor que LEN; en otro caso, 0. La envolvente sigue al objetivo con constantes de ataque y caída distintas.</p>
{eq('''objetivo = v_paso  si disparado ∧ fase < LEN,   si no 0
e[n] = e[n−1] + (objetivo − e[n−1]) · k(ATK)   si objetivo > e[n−1]
e[n] = e[n−1] + (objetivo − e[n−1]) · k(REL)   en otro caso
ganancia = 1 − DEPTH + DEPTH · e''')}
{table(["Control", "Rango", "Definición"], [
    ["DEPTH", "0–100 %", "profundidad del gate; 0 % lo anula"],
    ["LEN", "1–100 %", "fracción del paso con el gate abierto"],
    ["ATK", "0.1–100 ms", "constante de tiempo de subida"],
    ["REL", "1–2000 ms", "constante de tiempo de caída"],
    ["POS", "PRE FX, POST OUT", "PRE: sobre la rama WET antes del filtro pre y de la cadena; POST: sobre la mezcla final"],
], ncols=(0, 1))}

<h3>9.11 Operaciones de edición</h3>
{table(["Operación", "Definición"], [
    ["Dibujo", "clic izquierdo y arrastre: <code>v</code> = altura relativa del puntero; clic derecho y arrastre: <code>v = 0</code>; los pasos intermedios recorridos se rellenan"],
    ["&lt; &gt;", "rotación circular de los L primeros pasos, una posición"],
    ["RND (carril)", "GATE: <code>v = 0</code> con probabilidad 0.45, si no <code>U(0.5, 1)</code>; MOD: <code>U(0, 1)</code>"],
    ["EUC k", "ritmo euclidiano por el método de Bresenham: <code>v<sub>i</sub> = 1</code> si <code>(i·k) mod L &lt; k</code>, si no 0 (Toussaint, 2005). Clic izquierdo k+1, derecho k−1"],
    ["RND FX", "módulos uniformes en {0…10}, A y B <code>U(0, 1)</code>, DRIVE y MIX uniformes en su rango, OUT <code>U(−12, 0)</code> dB, T.SEED uniforme"],
    ["RND PAT", "aplica RND a los cuatro carriles"],
    ["RND SEQ", "GATE: RATE ∈ {1/8 … 1/16}, STEPS ∈ [5, 16], dirección FWD con probabilidad 0.7, PRB ∈ [60, 100], MUT 0 o U(0, 25); MOD: destino ∈ {1…18}, DEP, RATE, STEPS ∈ [2, 16], dirección, GLD uniformes, PRB ∈ [50, 100], MUT 0 o U(0, 30). No modifica DEPTH del gate."],
], ncols=(0,))}
<h3>9.12 Patrón inicial</h3>
<p>GATE: E(7,16) (<code>(7i) mod 16 &lt; 7</code>) con nivel 1 en los pasos múltiplos de 4 y 0.7 en el resto;
MOD 1: rampa <code>i/15</code>; MOD 2: <code>0.5 + 0.5·sin(0.9i)·cos(0.37i)</code>; MOD 3: alternancia 0.8/0.2.</p>
<h3>9.13 Seguidor de envolvente (ENV FOLLOW)</h3>
<p>Detector de pico que corre en cada muestra; la matriz lee su valor cada 16 muestras:</p>
{eq('''nivel = max(|L|, |R|) · 10^(GAIN/20)
e ← e + (nivel − e) · k(ATK)   si nivel > e;     e ← e + (nivel − e) · k(REL)   en otro caso
ENV = min(e, 1)''')}
{table(["Control", "Rango", "Definición"], [
    ["ATK", "0.1–100 ms", "constante de subida"],
    ["REL", "1–2000 ms", "constante de caída"],
    ["GAIN", "−24 a +24 dB", "ganancia antes del detector"],
    ["TAP", "VOICE, PRE-FILTER" if gen else "INPUT, PRE-FILTER",
     "VOICE: envolvente de la voz (ADSR por velocidad, sección 10); PRE-FILTER: salida de S4" if gen
     else "INPUT: señal de entrada; PRE-FILTER: salida de S4 (la modulación de la cadena por ENV realimenta al propio detector)"],
    ["SRC (carril MOD)", "SEQ, ENV, SEQ×ENV", "fuente de modulación del carril"],
], ncols=(0, 1))}
<p>El medidor rojo junto a TAP muestra el valor de ENV.</p>
"""


def sec_voice():
    return f"""
<h2 id="s10">10. VOICE: comportamiento como instrumento</h2>
<h3>10.1 Gestión de notas</h3>
<p>El instrumento es monofónico con prioridad a la última nota. Mantiene una pila de hasta 16 notas pulsadas.
Una nota nueva pasa a sonar y reinicia la envolvente desde su nivel actual (sin discontinuidad). Al soltar una nota,
si quedan otras pulsadas, la altura vuelve a la última de ellas sin reiniciar la envolvente (legato); si no queda
ninguna, comienza la fase de release. Un <i>note on</i> con velocidad 0 y un evento <i>choke</i> se tratan como
<i>note off</i>. Los eventos se procesan con precisión de muestra.</p>
<h3>10.2 Envolvente ADSR</h3>
{eq('''Ataque:    e ← e + 1/(ATK·fs)  hasta e = 1                 (lineal)
Caída:     e ← e + (SUS − e) · k(DEC)  hasta |e − SUS| < 1e−4  (exponencial)
Sostenido: e = SUS
Release:   e ← e − e · k(REL)  hasta e < 1e−5, luego e = 0    (exponencial)''')}
{table(["Control", "Rango", "Valor inicial"], [
    ["ATK", "0.1–2000 ms", "2 ms"],
    ["DEC", "1–5000 ms", "250 ms"],
    ["SUS", "0–100 %", "70 %"],
    ["REL", "1–8000 ms", "300 ms"],
], ncols=(0, 1, 2))}
<h3>10.3 Velocidad</h3>
{eq('''ganancia_voz = e · (1 − VEL + VEL · velocidad),     velocidad ∈ [0, 1]''')}
<p>VEL = 0 hace que todas las notas suenen con el mismo nivel.</p>
<h3>10.4 Posición de la envolvente (ENV)</h3>
<p><code>PRE FX</code>: la ganancia de voz multiplica la fuente (ruido) antes de la cadena; la distorsión recibe una
señal de nivel variable, por lo que el timbre cambia a lo largo de la envolvente. <code>POST OUT</code>: la ganancia
multiplica la salida mezclada; el timbre de la cadena no depende de la envolvente.</p>
<h3>10.5 Seguimiento de teclado</h3>
{eq('''f0 = 440 · 2^((nota − 69)/12)  Hz''')}
<p><b>TRACK</b> (0–100 %) desplaza la frecuencia de corte del filtro en proporción a la distancia a Do4 (nota 60),
después de aplicar la modulación del secuenciador:</p>
{eq('''fc' = clamp( fc · 2^( (nota − 60)/12 · TRACK ), 20, 20000 )''')}
<p>Con TRACK = 100 % el corte sigue una octava por octava. El filtro debe estar activo (MODE ≠ OFF) para que TRACK
tenga efecto.</p>
<p><b>PITCH</b> afina dos elementos a la nota:</p>
<ul>
  <li><b>LFSR</b>: la realimentación pasa a <code>b0 ⊕ b6</code> (modo corto), que produce una secuencia periódica de
    93 pasos, y el reloj se fija en <code>93·f0</code>; la señal resultante tiene frecuencia fundamental <code>f0</code>.
    El reloj está limitado a 4 pasos por muestra, lo que fija el máximo en <code>f0 = 4·fs/93</code> (≈ 2064 Hz a
    48 kHz). Solo tiene efecto con TYPE = LFSR.</li>
  <li><b>RING MOD</b>: en los slots con este módulo, la frecuencia del oscilador es <code>f0</code> en lugar de la dada
    por A.</li>
</ul>
<h3>10.6 Reinicio del secuenciador (RETRIG)</h3>
<p>Con RETRIG activo, cada nota nueva fija el origen <code>q0</code> del secuenciador (sección 9.3) en la posición
actual y fuerza el disparo del paso 0 en todos los carriles. Además, reinicia el generador de ruido a su estado
inicial, de modo que la fuente produce la misma secuencia en cada nota. Con RETRIG inactivo, <code>q0 = 0</code>, los
carriles siguen la posición del reloj y el ruido continúa sin reiniciarse.</p>
<h3>10.7 Valores iniciales específicos</h3>
<p>El instrumento se inicia con TYPE = PINK y LEVEL = −6 dB, para que una nota produzca señal sin configuración
previa. El resto de los valores iniciales coincide con el efecto.</p>
"""


def sec_ui(gen):
    n = 11 if gen else 10
    zonas = [
        ["Cabecera", "botones RND FX, RND PAT, RND SEQ; versión" + ("; indicador GEN" if gen else "") + "; LED y estado del reloj (SYNC, FREE, STOP) y tempo"],
        ["SOURCE", "TYPE, LEVEL, CHARACTER y descripción del tipo"],
        ["CHAIN", "cuatro slots (módulo, A, B) y DRIVE, MIX, OUT, T.SEED"],
        ["FILTER", "MODE, POS, CUTOFF, RESO, SCREAM"],
        ["SEQUENCER", "CLOCK, SWING, SEED y los controles del GATE"],
    ]
    fila = ["FEEDBACK · ENV FOLLOW · STEREO", "AMT, TIME, TONE, TAP; ATK, REL, GAIN, TAP y medidor del seguidor; SPREAD"]
    if gen:
        zonas.append(["VOICE", "ATK, DEC, SUS, REL, VEL, TRACK, PITCH, RETRIG, ENV; nota actual y medidor de la envolvente"])
    zonas.append(fila)
    zonas += [
        ["Carriles", "GATE y MOD 1–3: configuración a la izquierda (en los MOD, SRC en la cabecera), 16 pasos a la derecha"],
        ["Barra de ayuda", "descripción del control situado bajo el puntero"],
    ]
    return f"""
<h2 id="s{n}">{n}. Interfaz</h2>
<h3>{n}.1 Zonas</h3>
{table(["Zona", "Contenido"], zonas, ncols=(0,))}
<h3>{n}.2 Interacción</h3>
{table(["Elemento", "Acción", "Resultado"], [
    ["Barra", "arrastre horizontal o vertical", "variación relativa del valor (250 unidades de diseño = rango completo)"],
    ["", "Shift + arrastre", "variación ×0.1"],
    ["", "doble clic", "valor por defecto"],
    ["", "rueda", "un paso (parámetros discretos) o 2 % del rango"],
    ["Selector", "clic", "menú de opciones"],
    ["", "rueda", "opción anterior o siguiente"],
    ["Interruptor", "clic", "alterna ON / OFF"],
    ["Pasos", "clic izquierdo / derecho con arrastre", "dibujo / borrado"],
    ["Ventana", "esquina inferior derecha", "cambio de tamaño; el contenido se escala proporcionalmente"],
], ncols=(0,))}
<h3>{n}.3 Codificación cromática</h3>
{table(["Color", "Uso"], [
    ["#FF0033", "elemento activo: paso en reproducción, valor modulado dentro de una barra, LED de reloj y de slot, medidores de envolvente, SEQ OFF, botón bajo el puntero"],
    ["#E6E6E6", "texto, valores, pasos del patrón"],
    ["#4D4D4D", "relleno de barras, elementos inactivos, pasos fuera de la longitud del carril"],
    ["#1A1A1A / #0A0A0A", "paneles / fondo"],
], ncols=(0,))}
<p>Tipografías incluidas en el plugin: Anton (títulos) y Courier Prime (texto), ambas bajo SIL Open Font License 1.1.</p>
"""


def sec_estado(gen):
    n = 12 if gen else 11
    jsfx = "" if gen else f"""
<h3>{n}.3 Diferencias de la versión JSFX</h3>
<p>El procesamiento de señal es idéntico. Diferencias: la versión JSFX utiliza las fuentes del sistema Impact y Courier
New; sus sliders están ocultos pero disponibles para automatización; la edición de pasos crea puntos de deshacer en
REAPER; el tamaño de la ventana lo determina REAPER.</p>
"""
    return f"""
<h2 id="s{n}">{n}. Estado, automatización y determinismo</h2>
<h3>{n}.1 Estado guardado</h3>
<p>El estado del plugin incluye todos los parámetros, el patrón de 4 × 16 pasos, los valores de EUC de cada carril y
el tamaño de la ventana. Los identificadores de parámetro son estables entre versiones (véase <code>README.md</code>).
Las ediciones del patrón se guardan con el proyecto pero no generan entradas en el historial de deshacer del host.</p>
<h3>{n}.2 Determinismo</h3>
<p>Con el reloj sincronizado al host, todas las decisiones aleatorias del secuenciador (dirección RANDOM y DRUNK,
probabilidad) dependen solo de la posición, del carril y de SEED, de modo que renderizar dos veces el mismo pasaje
produce el mismo resultado. Desde la versión 0.7, además, cada resincronización (sección 9.1) restablece el ruido, los
slots y el filtro, de modo que el resultado tampoco depende de cuánto audio se procesó antes de reproducir ni de lo que
sonó antes de un salto. La única excepción es la mutación, cuyo efecto acumulado depende del historial de reproducción.
Con el reloj libre no hay resincronización y la fase del ruido depende del momento en que el host inicializa el
plugin.</p>
{jsfx}
"""


def sec_limitaciones(gen):
    n = 13 if gen else 12
    items = [
        "Sin sobremuestreo: los módulos con discontinuidades producen aliasing.",
        "La realimentación no tiene sobremuestreo: con FB AMOUNT alto y FB TIME corto produce aliasing (intencional).",
        "Las ediciones del patrón no generan historial de deshacer en el host.",
        "La mutación no es determinista.",
        "FL Studio 20 requiere la instalación en la carpeta VST3 estándar (sección 3).",
    ]
    if gen:
        items += [
            "Instrumento monofónico: no admite acordes; las notas simultáneas se gestionan por prioridad a la última.",
            "El modo PITCH del LFSR está limitado a f0 ≤ 4·fs/93.",
            "Una única instancia de la cadena procesa la voz: no hay estado independiente por nota.",
        ]
    else:
        items.append("Los estados de las versiones JSFX y VST/CLAP no son intercambiables.")
    return f"""
<h2 id="s{n}">{n}. Limitaciones conocidas</h2>
<ul>{"".join(f"<li>{i}</li>" for i in items)}</ul>
"""


def sec_refs(gen):
    n = 14 if gen else 13
    return f"""
<h2 id="s{n}">{n}. Referencias</h2>
<ol class="refs">
  <li>Simper, A. (2013). <i>Linear Trapezoidal Integrated State Variable Filter With Low Noise Optimisation</i>. Cytomic.</li>
  <li>Kellet, P. (2000). <i>Filter to make pink noise from white</i> (versión refinada). Music-DSP Source Code Archive.</li>
  <li>Marsaglia, G. (2003). Xorshift RNGs. <i>Journal of Statistical Software</i>, 8(14), 1–6.</li>
  <li>Toussaint, G. (2005). The Euclidean Algorithm Generates Traditional Musical Rhythms. <i>Proceedings of BRIDGES</i>, 47–56.</li>
  <li>Knuth, D. E. (1997). <i>The Art of Computer Programming, Vol. 2: Seminumerical Algorithms</i> (3.ª ed.). Addison-Wesley. (Generadores congruenciales lineales.)</li>
  <li>NESdev Wiki. <i>APU Noise</i>. (Registro de desplazamiento de 15 bits y modo de período corto.)</li>
  <li>Zölzer, U. (Ed.) (2011). <i>DAFX: Digital Audio Effects</i> (2.ª ed.). Wiley. (Distorsión, waveshaping y efectos no lineales.)</li>
  <li>Välimäki, V. y Huovilainen, A. (2007). Antialiasing Oscillators in Subtractive Synthesis. <i>IEEE Signal Processing Magazine</i>, 24(2). (Aliasing en formas de onda discontinuas.)</li>
</ol>
"""


def manual(gen):
    _eq_counter[0] = 0
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    titulo_html = 'NOISEGEK <span class="gen">GEN</span>' if gen else "NOISEGEK"
    resumen = (
        "NOISEGEK GEN es un instrumento virtual monofónico cuya fuente es un generador de ruido de seis tipos "
        "(blanco, rosa, browniano, impulsos y registro de desplazamiento de 15 bits). La señal atraviesa una cadena "
        "configurable de cuatro módulos no lineales, un filtro de variable de estado y un amplificador controlado por "
        "una envolvente ADSR disparada por notas MIDI, con seguimiento de teclado del filtro y afinación del LFSR y del "
        "modulador en anillo. Un secuenciador de cuatro carriles, sincronizable con el host y reiniciable por nota, "
        "modula los parámetros mediante una matriz de destinos."
        if gen else
        "NOISEGEK es un procesador de efectos que combina un generador de ruido de seis tipos, una cadena configurable "
        "de cuatro módulos no lineales (recorte, saturación, plegado, cuantización, diezmado, rectificación, "
        "waveshaping por tabla, muestreo y retención disparado, modulación en anillo y limitación de pendiente), un "
        "filtro de variable de estado y un secuenciador de cuatro carriles sincronizable con el host. El secuenciador "
        "controla un amplificador (gate) y modula los parámetros de la cadena mediante una matriz de destinos."
    )
    toc = [("s1", "Introducción"), ("s2", "Formatos y especificaciones"), ("s3", "Instalación"),
           ("s4", "Arquitectura de la señal"), ("s5", "SOURCE: generador de ruido"), ("s6", "CHAIN: cadena no lineal"),
           ("s7", "FILTER"), ("s8", "Etapa final: paneo"), ("s9", "SEQUENCER")]
    if gen:
        toc += [("s10", "VOICE: comportamiento como instrumento"), ("s11", "Interfaz"),
                ("s12", "Estado, automatización y determinismo"), ("s13", "Limitaciones conocidas"), ("s14", "Referencias")]
    else:
        toc += [("s10", "Interfaz"), ("s11", "Estado, automatización y determinismo"),
                ("s12", "Limitaciones conocidas"), ("s13", "Referencias")]
    toc_html = "".join(f'<li><a href="#{i}">{t}</a></li>' for i, t in toc)

    body = (sec_intro(gen) + sec_formatos(gen) + sec_instalacion(gen) + sec_arquitectura(gen) + sec_source(gen)
            + sec_chain(gen) + sec_filter(gen) + sec_seq(gen) + (sec_voice() if gen else "")
            + sec_ui(gen) + sec_estado(gen) + sec_limitaciones(gen) + sec_refs(gen))
    return f"""<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="author" content="{AUTOR}">
<title>{nombre} · Manual de referencia</title>
<style>{CSS}</style>
</head>
<body><div class="wrap">
<header>
  <h1>{titulo_html}</h1>
  <div class="meta">MANUAL DE REFERENCIA TÉCNICA · v{VERSION} · {FECHA} · {AUTOR}</div>
</header>
<div class="abstract"><b>RESUMEN</b><p>{resumen}</p></div>
<nav class="toc"><ol>{toc_html}</ol></nav>
{body}
<footer>{nombre} · Manual de referencia técnica · v{VERSION}<br>{COPY_DOC}</footer>
</div></body></html>
"""


# -----------------------------------------------------------------------------
# README técnico (tabla de parámetros con IDs estables)
# -----------------------------------------------------------------------------

PARAMS_CORE = [
    # id, nombre host, rango, defecto (fx), unidad/nota
    ("module_1..module_4", "Slot N Module", "0–10 (OFF … SLEW LIMITER)", "2, 0, 0, 0", "selector"),
    ("a_1..a_4", "Slot N Param A", "0–1", "0.5", "normalizado"),
    ("b_1..b_4", "Slot N Param B", "0–1", "0.5", "normalizado"),
    ("drive", "Input Drive", "−12 … 36", "0", "dB"),
    ("mix", "Mix Dry/Wet", "0–1", "1", "proporción"),
    ("out", "Output Gain", "−24 … 12", "0", "dB"),
    ("tseed", "Table Seed", "0–9999", "1234", "entero"),
    ("ntype", "Noise Type", "0–5 (OFF, WHITE, PINK, BROWN, DUST, LFSR)", "0 · GEN: 2", "selector"),
    ("nlevel", "Noise Level", "−60 … 0", "−18 · GEN: −6", "dB (−60 = off)"),
    ("nchar", "Noise Character", "0–1", "0.5", "normalizado"),
    ("fmode", "Filter Mode", "0–3 (OFF, LP, BP, HP)", "0", "selector"),
    ("cutoff", "Filter Cutoff", "20 … 20000", "2000", "Hz"),
    ("reso", "Filter Resonance", "0–1", "0.2", "proporción"),
    ("fpos", "Filter Position", "0–1 (POST, PRE)", "0", "selector"),
    ("clock", "Seq Clock", "0–2 (AUTO, HOST ONLY, FREE)", "0", "selector"),
    ("swing", "Seq Swing", "50 … 75", "50", "%"),
    ("sseed", "Seq Seed", "0–9999", "0", "entero"),
    ("gdepth", "Gate Depth", "0–1", "0", "proporción"),
    ("glen", "Gate Length", "0.01–1", "0.5", "proporción"),
    ("gatk", "Gate Attack", "0.1 … 100", "1", "ms"),
    ("grel", "Gate Release", "1 … 2000", "40", "ms"),
    ("gpos", "Gate Position", "0–1 (PRE FX, POST OUT)", "0", "selector"),
    ("grate", "Gate Rate", "0–11 (8 BAR … 1/64)", "9 (1/16)", "selector"),
    ("gsteps", "Gate Steps", "1–16", "16", "entero"),
    ("gdir", "Gate Direction", "0–4 (FWD, BWD, PINGPONG, RANDOM, DRUNK)", "0", "selector"),
    ("gprob", "Gate Probability", "0 … 100", "100", "%"),
    ("gmut", "Gate Mutate", "0 … 100", "0", "%"),
    ("dest_1..dest_3", "Mod N Destination", "0–21 (19–21 desde 0.8)", "0", "selector"),
    ("depth_1..depth_3", "Mod N Depth", "−100 … 100", "50", "%"),
    ("rate_1..rate_3", "Mod N Rate", "0–11", "9, 7, 3", "selector"),
    ("steps_1..steps_3", "Mod N Steps", "1–16", "16, 12, 7", "entero"),
    ("dir_1..dir_3", "Mod N Direction", "0–4", "0, 0, 2", "selector"),
    ("glide_1..glide_3", "Mod N Glide", "0 … 100", "0, 30, 100", "%"),
    ("prob_1..prob_3", "Mod N Probability", "0 … 100", "100", "%"),
    ("mutate_1..mutate_3", "Mod N Mutate", "0 … 100", "0", "%"),
    ("fbamt", "Feedback Amount", "0–1.5", "0", "proporción (0.8)"),
    ("fbtime", "Feedback Time", "0.05 … 50 (log.)", "5", "ms (0.8)"),
    ("fbtone", "Feedback Tone", "50 … 20000 (log.)", "6000", "Hz (0.8)"),
    ("fbtap", "Feedback Tap", "0–1 (PRE FILTER, POST FILTER)", "0", "selector (0.8)"),
    ("scream", "Filter Scream", "OFF / ON", "OFF", "booleano (0.8)"),
    ("eatk", "Env Attack", "0.1 … 100", "5", "ms (0.8)"),
    ("erel", "Env Release", "1 … 2000", "150", "ms (0.8)"),
    ("egain", "Env Gain", "−24 … 24", "0", "dB (0.8)"),
    ("etap", "Env Tap", "0–1 (INPUT · GEN: VOICE, PRE-FILTER)", "0 · GEN: 1", "selector (0.8)"),
    ("spread", "Stereo Spread", "0–1", "0", "proporción (0.8)"),
    ("src_1..src_3", "Mod N Source", "0–2 (SEQ, ENV, SEQ×ENV)", "0", "selector (0.8)"),
]
PARAMS_VOICE = [
    ("v_atk", "Attack", "0.1 … 2000", "2", "ms"),
    ("v_dec", "Decay", "1 … 5000", "250", "ms"),
    ("v_sus", "Sustain", "0–1", "0.7", "proporción"),
    ("v_rel", "Release", "1 … 8000", "300", "ms"),
    ("v_vel", "Velocity Sensitivity", "0–1", "0.7", "proporción"),
    ("v_track", "Key Track", "0–1", "0", "proporción"),
    ("v_pitch", "Pitched", "off/on", "off", "booleano"),
    ("v_retrig", "Retrigger Sequencer", "off/on", "on", "booleano"),
    ("v_envpos", "Envelope Position", "0–1 (PRE FX, POST OUT)", "1", "selector"),
]


def readme(gen):
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    bs = chr(92)
    rows = PARAMS_CORE + (PARAMS_VOICE if gen else [])
    tabla = "| ID | Nombre en el host | Rango | Defecto | Unidad |\n|---|---|---|---|---|\n" + "\n".join(
        "| `" + a + "` | " + b + " | " + c + " | " + d + " | " + e + " |" for a, b, c, d, e in rows)
    tipo = "Instrumento (generador)" if gen else "Efecto"
    io = ("sin entrada de audio · salida estéreo · entrada de notas MIDI" if gen
          else "estéreo → estéreo y mono → mono")
    ids = ("CLAP `com.ahrsml.noisegek-gen` · VST3 `NOISEGEKGENahrsm`" if gen
           else "CLAP `com.ahrsml.noisegek` · VST3 `NOISEGEKahrsml01`")
    formatos = "VST3, CLAP" if gen else "VST3, CLAP, JSFX (REAPER)"
    cf = "C:" + bs + "Program Files" + bs + "Common Files" + bs
    fila_jsfx = "" if gen else ("| `plugin/NOISEGEK.jsfx` | Versión JSFX para REAPER |\n"
                                "| `presets/` | Presets de demostración de la 0.8 (VST3, proyectos de REAPER y audio); ver `presets/README.md` |\n")
    seccion_jsfx = "" if gen else (
        "\n## Versión JSFX (REAPER)\n\n"
        "`plugin/NOISEGEK.jsfx` produce la misma señal que el VST3/CLAP. Se instala copiándolo a la "
        "carpeta `Effects` de REAPER (*Options → Show REAPER resource path*). Sus parámetros conservan "
        "la numeración de sliders desde la versión 0.2. Los estados del JSFX y del VST3/CLAP no son "
        "intercambiables.\n")
    host = ("En FL Studio, el instrumento aparece en el *Channel Rack* (generadores)."
            if gen else "En FL Studio, el efecto aparece en los slots del *Mixer*.")
    return f"""# {nombre} · README técnico

**{tipo}** · versión {VERSION} · autor **{AUTOR}** · licencia GPL-3.0-or-later

Manual de referencia: [`MANUAL.html`](MANUAL.html) · historial: [`CHANGELOG.md`](CHANGELOG.md)

## Contenido de esta carpeta

| Ruta | Descripción |
|---|---|
| `MANUAL.html` | Manual de referencia técnica (se abre en un navegador) |
| `README.md` | Este documento |
| `CHANGELOG.md` | Historial de versiones |
| `INSTALAR.bat` | Instala el plugin en las carpetas estándar de Windows |
| `DESINSTALAR.bat` | Elimina los archivos instalados |
| `plugin/{nombre}.vst3/` | Plugin VST3 (formato paquete) |
| `plugin/{nombre}.clap` | Plugin CLAP |
{fila_jsfx}| `licencias/` | Licencias del plugin y de sus componentes |

## Especificaciones

| Propiedad | Valor |
|---|---|
| Formatos | {formatos} |
| Plataforma | Windows de 64 bits; no requiere instalar componentes adicionales |
| E/S | {io} |
| Latencia | 0 muestras |
| Frecuencia de muestreo | cualquiera admitida por el host |
| Identificadores del plugin | {ids} |
| Fabricante (como lo muestra el host) | `{VENDOR}` |
| Interfaz | ventana redimensionable desde la esquina inferior derecha |

## Instalación

1. Ejecutar `INSTALAR.bat` (doble clic) y aceptar el permiso de administrador que solicita Windows.
   Copia los archivos a:
   * `{cf}VST3{bs}{nombre}.vst3`
   * `{cf}CLAP{bs}{nombre}.clap`
2. En el host, volver a escanear los plugins.

La copia también puede hacerse a mano con las mismas rutas. `DESINSTALAR.bat` elimina
ambos archivos.

FL Studio 20 solo detecta plugins VST3 en `{cf}VST3`; sus rutas de búsqueda
personalizadas se aplican únicamente a VST2. FL Studio 20 no admite CLAP. {host}
{seccion_jsfx}
## Parámetros

Todos los parámetros son automatizables. Los identificadores (ID) se mantienen entre
versiones, de modo que los proyectos y presets guardados siguen siendo válidos. Los
parámetros de slots y carriles MOD llevan el sufijo `_1`, `_2`, …

{tabla}

### Destinos de modulación (`dest_N`)

`0` OFF · `1–8` S1 A, S1 B, S2 A, S2 B, S3 A, S3 B, S4 A, S4 B · `9` DRIVE ·
`10` MIX · `11` CUTOFF · `12` RESO · `13` NOISE LVL · `14` PAN ·
`15–18` S1–S4 ON/OFF · `19` FB AMOUNT · `20` FB TIME · `21` SPREAD

### Estado guardado con el proyecto

Además de los parámetros, el proyecto guarda el patrón del secuenciador (4 carriles ×
16 pasos), el valor EUC de cada carril y el tamaño de la ventana.

## Autoría y licencias

**© 2026 Adolfo Rosas (@ahrsml).** Concepto, diseño, sonido, interfaz y documentación de
{nombre} son obra de Adolfo Rosas. La documentación (este README, `MANUAL.html` y
`CHANGELOG.md`) está protegida: **todos los derechos reservados**.

Los archivos de plugin (`.vst3`, `.clap`) incluyen software de terceros licenciado bajo la
GNU GPL v3 y, por ese motivo, se distribuyen bajo la **GNU General Public License v3.0 o
posterior**; Adolfo Rosas conserva la titularidad de los derechos de autor. De acuerdo con
esa licencia, el código fuente correspondiente está disponible solicitándolo al autor.
Componentes de terceros: NIH-plug (ISC), fuentes Anton y Courier Prime (SIL Open Font
License 1.1). VST es una marca registrada de Steinberg Media Technologies GmbH. Textos
completos en `licencias/`.
"""


def changelog(gen):
    if gen:
        return f"""# NOISEGEK GEN · Historial

## 0.8.1 · {FECHA}
* Interfaz más compacta: la ventana inicial (960 × 659) cabe en pantallas de 768 px de alto.
  Se agranda desde la esquina inferior derecha.
* Los proyectos guardados con versiones anteriores abren con el tamaño nuevo.
* Sin cambios en el sonido.

## 0.8 · {FECHA}
* Realimentación global (FEEDBACK: AMT, TIME, TONE, TAP), con FB AMOUNT y FB TIME como
  destinos de modulación.
* SCREAM: el filtro puede autooscilar en el corte.
* Seguidor de envolvente (ENV FOLLOW) como fuente de los carriles MOD (SRC: SEQ, ENV o
  SEQ×ENV). En el instrumento sigue la salida de los slots o la envolvente de la voz (VOICE).
* SPREAD: diferencias entre L y R; también es destino de modulación.
* Interfaz con una fila más (FEEDBACK · ENV FOLLOW · STEREO).
* Con los valores por defecto el sonido es idéntico al de la 0.7; los proyectos y presets
  anteriores se cargan con los parámetros nuevos en su valor por defecto.

## 0.7 · {FECHA}
* Resincronización con el transporte (igual que NOISEGEK 0.7): al iniciar la reproducción
  o al saltar la posición más de 64 muestras, el ruido, los slots, el filtro y los carriles
  vuelven a su estado inicial. Con reloj sincronizado, dos renders del mismo proyecto son
  idénticos aunque difiera el audio procesado antes de reproducir (salvo con MUT > 0).
* Los carriles MOD comienzan en el valor de su primer paso; el gate parte de 0.
* RETRIG reinicia también el generador de ruido: la fuente produce la misma secuencia en
  cada nota.

## 0.6 · {FECHA}
* Primera versión del instrumento. Mismo procesamiento de señal que NOISEGEK 0.6.
* Notas MIDI con precisión de muestra; monofónico, prioridad a la última nota, legato.
* Envolvente ADSR, sensibilidad a la velocidad, posición PRE/POST.
* Seguimiento de teclado del filtro (TRACK).
* PITCH: LFSR en modo corto (93 pasos) y RING MOD afinados a la nota.
* RETRIG: cada nota reinicia el secuenciador.
* Valores iniciales: ruido rosa a −6 dB.
"""
    return f"""# NOISEGEK · Historial

## 0.8.1 · {FECHA}
* Interfaz más compacta: la ventana inicial (1000 × 632) cabe en pantallas de 768 px de alto.
  SCREAM pasa junto a RESO. Se agranda desde la esquina inferior derecha.
* Los proyectos guardados con versiones anteriores abren con el tamaño nuevo.
* Sin cambios en el sonido. JSFX: mismo cambio de interfaz.

## 0.8 · {FECHA}
* Realimentación global (FEEDBACK: AMT, TIME, TONE, TAP), con FB AMOUNT y FB TIME como
  destinos de modulación.
* SCREAM: el filtro puede autooscilar en el corte.
* Seguidor de envolvente (ENV FOLLOW) como fuente de los carriles MOD (SRC: SEQ, ENV o
  SEQ×ENV), tomado de la entrada o de la salida de los slots.
* SPREAD: diferencias entre L y R; también es destino de modulación.
* Interfaz con una fila más (FEEDBACK · ENV FOLLOW · STEREO).
* Con los valores por defecto el sonido es idéntico al de la 0.7; los proyectos y presets
  anteriores (incluidos los del JSFX) se cargan con los parámetros nuevos en su valor por defecto.
* JSFX: mismos cambios (sliders 61–73).

## 0.7 · {FECHA}
* Resincronización con el transporte: al iniciar la reproducción o al saltar la posición
  más de 64 muestras (búsqueda, vuelta de bucle), el ruido, los slots, el filtro y los
  carriles vuelven a su estado inicial. Con reloj sincronizado, dos renders del mismo
  proyecto son idénticos aunque difiera el audio procesado antes de reproducir (salvo con
  MUT > 0). Antes, la fase del ruido, del LFSR, del RING MOD y del TRIGGER S/H dependía del
  momento en que se inicializó el plugin.
* Los carriles MOD comienzan en el valor de su primer paso; el gate parte de 0.
* Con FREE RUN el comportamiento no cambia.
* JSFX: mismo cambio (versión 0.7).

## 0.6 · {FECHA}
* Se publica en paralelo el instrumento NOISEGEK GEN.
* Sin cambios en el procesamiento de señal del efecto.

## 0.5 · {FECHA}
* Primera versión VST3/CLAP. Interfaz escalable con fuentes propias.
* JSFX: tipografía mayor, ventana inicial 1300×845, atribución.
* Un estado de proyecto dañado se rechaza sin cerrar el host.
* El host recibe aviso de los nuevos valores tras cargar un preset.

## 0.4
* Paleta (#0A0A0A, #1A1A1A, #E6E6E6, #FF0033, #4D4D4D) y tipografía Impact/Courier.

## 0.3
* Interfaz propia con paneles y barra de ayuda; sliders ocultos.
* Corrección de un error que podía dibujar el panel CHAIN con valores incorrectos.

## 0.2
* Generador de ruido, filtro SVF y secuenciador de cuatro carriles.

## 0.1
* Cadena de cuatro slots con diez módulos de distorsión.
"""


def bat_instalar(nombre):
    # Archivo .bat autocontenido: se eleva a administrador y copia desde plugin\.
    return "\r\n".join([
        "@echo off",
        "chcp 65001 >nul",
        "rem " + nombre + " - instalador (" + AUTOR + ")",
        "net session >nul 2>&1",
        "if errorlevel 1 (",
        "  powershell -NoProfile -Command \"Start-Process -FilePath '%~f0' -Verb RunAs\"",
        "  exit /b",
        ")",
        "set \"SRC=%~dp0plugin\"",
        "set \"VST3=%CommonProgramW6432%\\VST3\"",
        "set \"CLAP=%CommonProgramW6432%\\CLAP\"",
        "if not exist \"%VST3%\" mkdir \"%VST3%\"",
        "if not exist \"%CLAP%\" mkdir \"%CLAP%\"",
        "if exist \"%VST3%\\" + nombre + ".vst3\" rmdir /S /Q \"%VST3%\\" + nombre + ".vst3\"",
        "xcopy \"%SRC%\\" + nombre + ".vst3\" \"%VST3%\\" + nombre + ".vst3\\\" /E /I /Y /Q >nul",
        "copy /Y \"%SRC%\\" + nombre + ".clap\" \"%CLAP%\\\" >nul",
        "echo.",
        "echo   " + nombre + " instalado:",
        "echo     %VST3%\\" + nombre + ".vst3",
        "echo     %CLAP%\\" + nombre + ".clap",
        "echo.",
        "echo   Vuelve a escanear los plugins en tu DAW.",
        "echo.",
        "pause",
        "",
    ])


def bat_desinstalar(nombre):
    return "\r\n".join([
        "@echo off",
        "chcp 65001 >nul",
        "rem " + nombre + " - desinstalador (" + AUTOR + ")",
        "net session >nul 2>&1",
        "if errorlevel 1 (",
        "  powershell -NoProfile -Command \"Start-Process -FilePath '%~f0' -Verb RunAs\"",
        "  exit /b",
        ")",
        "set \"VST3=%CommonProgramW6432%\\VST3\"",
        "set \"CLAP=%CommonProgramW6432%\\CLAP\"",
        "if exist \"%VST3%\\" + nombre + ".vst3\" rmdir /S /Q \"%VST3%\\" + nombre + ".vst3\"",
        "if exist \"%CLAP%\\" + nombre + ".clap\" del /Q \"%CLAP%\\" + nombre + ".clap\"",
        "echo.",
        "echo   " + nombre + " desinstalado.",
        "echo.",
        "pause",
        "",
    ])


# -----------------------------------------------------------------------------

def escribir(carpeta, gen):
    nombre = "NOISEGEK GEN" if gen else "NOISEGEK"
    os.makedirs(os.path.join(carpeta, "licencias"), exist_ok=True)
    os.makedirs(os.path.join(carpeta, "plugin"), exist_ok=True)
    with open(os.path.join(carpeta, "MANUAL.html"), "w", encoding="utf-8", newline="\n") as f:
        f.write(manual(gen))
    with open(os.path.join(carpeta, "README.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(readme(gen))
    with open(os.path.join(carpeta, "CHANGELOG.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write(changelog(gen))
    with open(os.path.join(carpeta, "INSTALAR.bat"), "w", encoding="utf-8", newline="") as f:
        f.write(bat_instalar(nombre))
    with open(os.path.join(carpeta, "DESINSTALAR.bat"), "w", encoding="utf-8", newline="") as f:
        f.write(bat_desinstalar(nombre))
    lic = os.path.join(carpeta, "licencias")
    shutil.copy(os.path.join(VST, "LICENSE-GPL-3.0.txt"), os.path.join(lic, "GPL-3.0.txt"))
    shutil.copy(os.path.join(VST, "third_party", "nih-plug", "LICENSE"), os.path.join(lic, "NIH-plug-ISC.txt"))
    fonts = os.path.join(VST, "noisegek-core", "assets", "fonts")
    shutil.copy(os.path.join(fonts, "Anton-OFL.txt"), os.path.join(lic, "Anton-OFL-1.1.txt"))
    shutil.copy(os.path.join(fonts, "CourierPrime-OFL.txt"), os.path.join(lic, "CourierPrime-OFL-1.1.txt"))
    # binarios
    dist = os.path.join(VST, "dist")
    src3 = os.path.join(dist, nombre + ".vst3")
    dst3 = os.path.join(carpeta, "plugin", nombre + ".vst3")
    if os.path.isdir(src3):
        if os.path.isdir(dst3):
            shutil.rmtree(dst3)
        shutil.copytree(src3, dst3)
        shutil.copy(os.path.join(dist, nombre + ".clap"), os.path.join(carpeta, "plugin", nombre + ".clap"))
    if not gen:
        jsfx = os.path.join(ROOT, "jsfx", "NOISEGEK.jsfx")
        if os.path.isfile(jsfx):
            shutil.copy(jsfx, os.path.join(carpeta, "plugin", "NOISEGEK.jsfx"))
    print("ok:", carpeta)


if __name__ == "__main__":
    escribir(DIR_GEN, True)
