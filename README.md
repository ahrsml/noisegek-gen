# NOISEGEK GEN · código fuente (Rust) · por @ahrsml

Plugin VST3 / CLAP: instrumento (Channel Rack / piano roll).

El motor de sonido (`noisegek-dsp`) y los parámetros y la interfaz
(`noisegek-core`) son los mismos que usa [NOISEGEK](https://github.com/ahrsml/noisegek), el efecto; cada repositorio los
trae completos para compilar solo.

`build.ps1` deja el plugin en `dist\` y genera la carpeta de distribución
`dist\NOISEGEK GEN (instrumento)\` (`MANUAL.html`, `README.md` con parámetros e IDs,
`CHANGELOG.md`, `plugin/` y `licencias/`) con `docs/generar_docs.py`, la fuente
única del manual.

## Compilar

```powershell
powershell -ExecutionPolicy Bypass -File .\build.ps1 -Test   # tests + VST3/CLAP + docs
powershell -ExecutionPolicy Bypass -File .\instalar.ps1      # copia a Common Files (admin)
powershell -ExecutionPolicy Bypass -File .\desinstalar.ps1
powershell -ExecutionPolicy Bypass -File .\limpiar.ps1       # borra target\ (~2 GB)
```

FL Studio 20 solo encuentra VST3 en `C:\Program Files\Common Files\VST3`: sus rutas
personalizadas son solo para VST2.

Todo es local: `.toolchain\` contiene Rust estable (`x86_64-pc-windows-gnu`), la
caché de cargo y `clap-validator`. Compilar no modifica el PATH, el registro ni
Program Files. Para usar cargo a mano: `. .\env.ps1`. En un clon nuevo esa carpeta
no existe (no se sube al repositorio): instala Rust estable con rustup usando
`RUSTUP_HOME=.toolchain\rustup` y `CARGO_HOME=.toolchain\cargo`.

## Estructura

```
noisegek-dsp/    DSP puro, sin dependencias, 24 tests (cargo test -p noisegek-dsp)
  engine.rs      motor: reloj, gate, matriz de modulación, cadena, voz (GEN)
  modules.rs     10 módulos + mapeo A/B + tablas del waveshaper
  noise.rs       white / pink / brown / dust / LFSR (modo largo y corto)
  filter.rs      SVF de Simper
  seq.rs         patrón, carriles, rates, euclidiano, rotación
  rng.rs         xorshift32, hash determinista, LCG
noisegek-core/   parámetros (IDs estables), UI egui, fuentes OFL (compartido con el otro plugin)
noisegek-gen/    exporta el instrumento  (+ bin standalone)
xtask/           empaquetado (cargo xtask bundle -p noisegek-gen --release)
third_party/     NIH-plug con 2 parches (third_party/PATCHES.md)
docs/            generador de la documentación de distribución
```

El efecto y el instrumento son plugins (y repositorios) separados a propósito: FL Studio no
verifica por defecto archivos con varios plugins dentro ("shell plugins").

## Verificación

* `cargo test -p noisegek-dsp --release`: 24 tests.
* `.toolchain\tools\bin\clap-validator.exe validate "dist\NOISEGEK GEN.clap"` (35/0/9): superados / fallidos / no aplicables.
* UI sin DAW: `cargo run -p noisegek-gen --features standalone --release -- --backend dummy`.

## Reglas para no romper proyectos

* No cambiar los `#[id = "..."]` (`noisegek-core/src/params.rs`) ni las claves
  `#[persist = "..."]`.
* No cambiar `VST3_CLASS_ID` ni `CLAP_ID` de ninguno de los dos plugins.
* Nada en `process()` debe asignar memoria ni bloquear (`try_read`, `try_write`, `try_lock`).
* Si cambia el comportamiento del DSP, actualizar `docs/generar_docs.py`.

## Licencia

GPL-3.0-or-later (por `vst3-sys`, usado por NIH-plug). Texto en `LICENSE-GPL-3.0.txt`.
