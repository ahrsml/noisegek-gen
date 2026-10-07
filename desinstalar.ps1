# NOISEGEK · por @ahrsml
# Quita NOISEGEK GEN de las carpetas del sistema (lo que puso
# instalar.ps1). No toca esta carpeta ni tus proyectos.
param([switch]$Elevado)

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$($MyInvocation.MyCommand.Path)`" -Elevado"
    exit
}
foreach ($name in @("NOISEGEK GEN")) {
    foreach ($p in "C:\Program Files\Common Files\VST3\$name.vst3", "C:\Program Files\Common Files\CLAP\$name.clap") {
        if (Test-Path $p) { Remove-Item -Recurse -Force $p; Write-Host "Borrado: $p" }
    }
}
if ($Elevado) { Start-Sleep -Seconds 3 }
