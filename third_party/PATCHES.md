# Parches sobre nih-plug

Copia de `robbert-vdh/nih-plug` en el commit `de421011f41a6d10fc8c7a6084e4f4dee0143683`
(licencia ISC, ver `nih-plug/LICENSE`), sin su historial git, con tres cambios
marcados con `NOISEGEK patch` (los dos primeros en `src/wrapper/clap/wrapper.rs`):

1. **Estado corrupto**: `ext_state_load` reservaba memoria con el largo leído del
   stream sin validarlo; un estado basura abortaba el proceso del DAW. Ahora se
   rechaza cualquier estado de más de 64 MB.
2. **Rescan tras cargar estado**: después de cargar un estado se avisa al host
   (`CLAP_PARAM_RESCAN_VALUES`), como pide la especificación de CLAP.

Ambos fallos los detecta `clap-validator` (tests `state-invalid-random` y
`state-reproducibility-*`).

3. **Rango logarítmico** (0.8, `src/params/range.rs` y `src/params/float.rs`):
   `FloatRange::Logarithmic { min, max }`, con `normalizado = ln(v/min) / ln(max/min)`.
   NIH-plug solo trae rangos lineales y con potencia (`Skewed`), que no reproducen una
   escala logarítmica; lo usa FB TIME (0.05–50 ms).
