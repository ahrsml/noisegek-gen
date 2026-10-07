# NOISEGEK · por @ahrsml
# Compila NOISEGEK GEN (VST3 + CLAP) con el Rust LOCAL de esta carpeta y los
# deja en .\dist\. No instala nada en el sistema.
#   NOISEGEK GEN = instrumento (Channel Rack / piano roll)
#
#   Uso (desde esta carpeta):   powershell -ExecutionPolicy Bypass -File .\build.ps1
#   Opcional: -Test   corre también los tests del DSP antes de compilar.
param([switch]$Test)

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here
. "$here\env.ps1"

if ($Test) {
    cargo test -p noisegek-dsp --release
    if ($LASTEXITCODE -ne 0) { throw "Fallaron los tests del DSP" }
    cargo test -p noisegek-core --release
    if ($LASTEXITCODE -ne 0) { throw "Fallaron los tests de parámetros y estado" }
}

cargo xtask bundle -p noisegek-gen --release
if ($LASTEXITCODE -ne 0) { throw "Falló la compilación" }

$dist = Join-Path $here "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
foreach ($pair in @(@("noisegek-gen", "NOISEGEK GEN"))) {
    $src = $pair[0]; $name = $pair[1]
    $vst3 = Join-Path $dist "$name.vst3"
    if (Test-Path $vst3) { Remove-Item -Recurse -Force $vst3 }
    New-Item -ItemType Directory -Force "$vst3\Contents\x86_64-win" | Out-Null
    Copy-Item "$here\target\bundled\$src.vst3\Contents\x86_64-win\$src.vst3" "$vst3\Contents\x86_64-win\$name.vst3"
    Copy-Item "$here\target\bundled\$src.clap" (Join-Path $dist "$name.clap") -Force
}

# Documentación y copia del plugin en la carpeta de distribución:
#   .\dist\NOISEGEK GEN (instrumento)\
if (Get-Command python -ErrorAction SilentlyContinue) {
    python "$here\docs\generar_docs.py"
    python "$here\docs\generar_especificacion.py"   # especificación técnica en dist\
} else {
    Write-Host "Aviso: no se encontró Python; la documentación no se regeneró."
}

Write-Host ""
Write-Host "Para instalar en el sistema (necesario para FL Studio): .\instalar.ps1"
Write-Host "Listo. Plugins en: $dist"
Get-ChildItem $dist -Recurse -File | ForEach-Object { Write-Host ("  " + $_.FullName.Substring($dist.Length + 1) + "  (" + [math]::Round($_.Length / 1MB, 1) + " MB)") }
