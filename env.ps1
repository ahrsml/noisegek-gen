# NOISEGEK · por @ahrsml
# Activa el toolchain de Rust LOCAL de este proyecto (solo para esta ventana de
# PowerShell). No modifica el PATH del sistema ni el registro.
#   Uso:  . .\env.ps1      (con el punto delante)
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:RUSTUP_HOME = Join-Path $here ".toolchain\rustup"
$env:CARGO_HOME  = Join-Path $here ".toolchain\cargo"
$bin = Join-Path $env:CARGO_HOME "bin"
if (-not ($env:PATH -split ';' | Where-Object { $_ -eq $bin })) { $env:PATH = "$bin;$env:PATH" }
Write-Host "Rust local activo: $(& (Join-Path $bin 'rustc.exe') --version)"
