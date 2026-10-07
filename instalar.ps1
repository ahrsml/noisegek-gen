# NOISEGEK · por @ahrsml
# Instala NOISEGEK GEN en las carpetas
# ESTÁNDAR del sistema, que es donde los buscan todos los DAWs (FL Studio 20
# solo busca VST3 ahí). Pide permiso de administrador (Windows lo exige para
# escribir en Program Files).
#
#   powershell -ExecutionPolicy Bypass -File .\instalar.ps1
#   Para quitarlos: .\desinstalar.ps1
param([switch]$Elevado)

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$($MyInvocation.MyCommand.Path)`" -Elevado"
    exit
}

$installed = @()
foreach ($name in @("NOISEGEK GEN")) {
    $src3 = Join-Path $here "dist\$name.vst3"
    $srcC = Join-Path $here "dist\$name.clap"
    if (-not (Test-Path $src3)) { Write-Host "No existe $src3. Compila primero con build.ps1"; if ($Elevado) { Read-Host "Enter para cerrar" }; exit 1 }
    $dst3 = "C:\Program Files\Common Files\VST3\$name.vst3"
    $dstC = "C:\Program Files\Common Files\CLAP\$name.clap"
    if (Test-Path $dst3) { Remove-Item -Recurse -Force $dst3 }
    Copy-Item -Recurse $src3 $dst3
    New-Item -ItemType Directory -Force (Split-Path $dstC) | Out-Null
    Copy-Item -Force $srcC $dstC
    $installed += $dst3, $dstC
}

Write-Host "Instalado:"
$installed | ForEach-Object { Write-Host "  $_" }
Write-Host ""
Write-Host "En FL Studio: Options > Manage plugins > Find installed plugins, y marca la estrella"
Write-Host "de NOISEGEK GEN."
if ($Elevado) { Start-Sleep -Seconds 4 }
