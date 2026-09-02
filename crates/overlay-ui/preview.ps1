<#
.SYNOPSIS
    Build + lance overlay-ui pour prévisualiser l'overlay à l'instant T sur un vrai wakfu.log.

.DESCRIPTION
    Commande unique, même esprit que spikes/s1-window-windows/preview.ps1 :
      1. Prépare vendor/wgpu-hal-30.0.1 (patch DirectComposition, voir patches/setup-vendor.sh à
         la racine du dépôt) s'il est absent.
      2. `cargo run -p overlay-ui` (release par défaut) : compile puis lance directement la
         fenêtre overlay, câblée sur overlay-ingest + overlay-engine.

    La fenêtre est transparente, toujours au-dessus, sans bordure. Ctrl+Alt+W bascule interactif /
    clic-traversant (hotkey global). Ctrl+Alt+R force un rafraîchissement (overlay bloqué, mal
    positionné ou mal dimensionné) sans relancer tout le processus. Échap (fenêtre focalisée) ou
    Ctrl+C (dans cette console) pour quitter.

.PARAMETER LogPath
    Chemin explicite vers un wakfu.log (utile pour rejouer un fichier plutôt que suivre le vrai
    log du jeu). Sans ce paramètre : découverte automatique (overlay_ingest::discovery).

.PARAMETER Debug
    Build debug au lieu de release : compilation plus rapide, utile pour itérer sur le code.

.EXAMPLE
    .\preview.ps1
    Build release + lance l'overlay sur le wakfu.log découvert automatiquement.

.EXAMPLE
    .\preview.ps1 -LogPath C:\chemin\vers\un\autre\wakfu.log -Debug
#>
param(
    [string]$LogPath,
    [switch]$Debug
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $repoRoot

$vendorDir = Join-Path $repoRoot "vendor\wgpu-hal-30.0.1"
if (-not (Test-Path $vendorDir)) {
    Write-Host "vendor/ absent — préparation via patches/setup-vendor.sh (patch DirectComposition)..." -ForegroundColor Yellow
    & bash patches/setup-vendor.sh
    if ($LASTEXITCODE -ne 0) {
        throw "patches/setup-vendor.sh a échoué (exit $LASTEXITCODE) — voir la sortie ci-dessus."
    }
}

Write-Host ""
Write-Host "=== overlay-ui — prévisualisation ===" -ForegroundColor Cyan
Write-Host "Ctrl+Alt+W = bascule interactif / clic-traversant  |  Ctrl+Alt+R = rafraîchir  |  Échap ou Ctrl+C = quitter" -ForegroundColor Cyan
Write-Host ""

$cargoArgs = @("run", "-p", "overlay-ui")
if (-not $Debug) { $cargoArgs += "--release" }
if ($LogPath) { $cargoArgs += @("--", $LogPath) }

& cargo @cargoArgs
