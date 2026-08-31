<#
.SYNOPSIS
    Build + lance le spike S1 pour prévisualiser l'overlay transparent Windows à l'instant T.

.DESCRIPTION
    Commande unique pour voir le résultat du spike sans se souvenir des étapes :
      1. Prépare vendor/wgpu-hal-30.0.1 (patch DirectComposition) s'il est absent —
         voir README.md, §"Le patch wgpu-hal".
      2. `cargo run` (release par défaut) : compile puis lance directement la fenêtre overlay.

    La fenêtre est transparente, toujours au-dessus, sans bordure, positionnée en haut à gauche
    de l'écran principal. Ctrl+Alt+W bascule interactif / clic-traversant (hotkey global, marche
    même sans focus). Échap (fenêtre focalisée) ou Ctrl+C (dans cette console) pour quitter.

.PARAMETER Debug
    Build debug au lieu de release : compilation plus rapide, RSS affiché plus élevé (non
    représentatif) — utile pour itérer sur le code, pas pour mesurer la mémoire.

.EXAMPLE
    .\preview.ps1
    Build release + lance l'overlay (quelques secondes après la première compilation).

.EXAMPLE
    .\preview.ps1 -Debug
    Idem, en build debug (recompilation plus rapide entre deux essais).
#>
param(
    [switch]$Debug
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$vendorDir = Join-Path $PSScriptRoot "vendor\wgpu-hal-30.0.1"
if (-not (Test-Path $vendorDir)) {
    Write-Host "vendor/ absent — préparation via patches/setup-vendor.sh (patch DirectComposition)..." -ForegroundColor Yellow
    & bash patches/setup-vendor.sh
    if ($LASTEXITCODE -ne 0) {
        throw "patches/setup-vendor.sh a échoué (exit $LASTEXITCODE) — voir la sortie ci-dessus."
    }
}

Write-Host ""
Write-Host "=== Spike S1 — prévisualisation overlay ===" -ForegroundColor Cyan
Write-Host "Ctrl+Alt+W = bascule interactif / clic-traversant  |  Échap ou Ctrl+C = quitter" -ForegroundColor Cyan
Write-Host ""

if ($Debug) {
    cargo run
} else {
    cargo run --release
}
