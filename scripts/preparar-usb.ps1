<#
.SYNOPSIS
    Arma una USB de Drivward Portable en esta compu (DIST-03).

.DESCRIPTION
    Copia drivward-portable.exe a la USB, descarga los paquetes oficiales listados en el
    manifiesto, verifica su SHA-256, los extrae en <USB>\Drivward\drivers y genera index.json.
    Los drivers no vienen en el repositorio: se descargan de la fuente oficial en tu compu.

.EXAMPLE
    .\scripts\preparar-usb.ps1 -Usb E: -Manifiesto .\library\fuentes.json
#>
param(
    [Parameter(Mandatory = $true)] [string] $Usb,
    [Parameter(Mandatory = $true)] [string] $Manifiesto,
    [string] $Binarios = (Join-Path $PSScriptRoot '..\target\release'),
    [switch] $SoloRedistribuibles
)

$ErrorActionPreference = 'Stop'
$destino = Join-Path $Usb 'Drivward'
$drivers = Join-Path $destino 'drivers'
New-Item -ItemType Directory -Force -Path $drivers | Out-Null

foreach ($exe in 'drivward-portable.exe', 'drivward-index.exe') {
    $origen = Join-Path $Binarios $exe
    if (-not (Test-Path $origen)) { throw "No se encontró $origen. Compila con: cargo build --release" }
}
Copy-Item (Join-Path $Binarios 'drivward-portable.exe') $destino -Force

$argumentos = @('prepare', $Manifiesto, $drivers)
if ($SoloRedistribuibles) { $argumentos += '--redistributable' }
& (Join-Path $Binarios 'drivward-index.exe') @argumentos
if ($LASTEXITCODE -eq 2) { throw 'El preparador terminó con error.' }
if ($LASTEXITCODE -eq 1) { Write-Warning 'Algunos paquetes se omitieron; revisa la salida anterior.' }

& (Join-Path $Binarios 'drivward-index.exe') verify $drivers
Write-Host "USB lista en $destino. Abre drivward-portable.exe en la compu formateada."
