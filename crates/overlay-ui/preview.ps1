<#
.SYNOPSIS
    Build + lance overlay-ui pour prévisualiser l'overlay à l'instant T sur un vrai wakfu.log.

.DESCRIPTION
    Commande unique, même esprit que spikes/s1-window-windows/preview.ps1 :
      1. Prépare vendor/wgpu-hal-30.0.1 (patch DirectComposition, voir patches/setup-vendor.sh à
         la racine du dépôt) s'il est absent.
      2. `cargo run -p overlay-ui --profile preview` : compile puis lance directement la
         fenêtre overlay, câblée sur overlay-ingest + overlay-engine. Le profil `preview`
         (Cargo.toml racine) optimise comme `release` mais sans LTO fat ni `codegen-units = 1`,
         qui ne servent qu'au binaire livré et coûtaient 5 à 10 min par relance.

    La fenêtre est transparente, toujours au-dessus, sans bordure. Ctrl+Shift+W bascule interactif /
    clic-traversant (hotkey global). Ctrl+Shift+R force un rafraîchissement (overlay bloqué, mal
    positionné/dimensionné, ou Suivi resté vide — redemande aussi les réglages de compte) sans
    relancer tout le processus. Ctrl+Shift+Q (hotkey global, fonctionne sans focus) ou Ctrl+C (dans
    cette console) pour quitter — les fenêtres overlay ne peuvent jamais recevoir le focus clavier
    (WS_EX_NOACTIVATE), Échap ne fonctionne donc pas en pratique.

.PARAMETER LogPath
    Chemin explicite vers un wakfu.log (utile pour rejouer un fichier plutôt que suivre le vrai
    log du jeu). Sans ce paramètre : découverte automatique (overlay_ingest::discovery).

.PARAMETER Debug
    Build debug au lieu du profil preview : sans optimisation, utile pour un débogueur.

.PARAMETER Release
    Le VRAI profil release (LTO fat, strip) : le binaire exactement tel que le livrerait
    `.github/workflows/release.yml`. Long à compiler — réservé à une vérification finale.

.EXAMPLE
    .\preview.ps1
    Build preview + lance l'overlay sur le wakfu.log découvert automatiquement.

.EXAMPLE
    .\preview.ps1 -LogPath C:\chemin\vers\un\autre\wakfu.log -Debug
#>
param(
    [string]$LogPath,
    [switch]$Debug,
    [switch]$Release
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

# Deux binaires coexistent dans le crate (overlay-ui pour Windows, overlay-ui-x11 pour
# Linux/X11) — cargo ne peut pas choisir seul, on détermine ici lequel lancer selon l'OS courant
# pour que l'utilisateur n'ait jamais à s'en soucier. $IsWindows n'existe pas sous Windows
# PowerShell 5.1 (toujours Windows dans ce cas) mais existe sous pwsh (Core, cross-plateforme).
$isWindowsHost = $true
if (Test-Path variable:IsWindows) { $isWindowsHost = $IsWindows }
$binName = if ($isWindowsHost) { "overlay-ui" } else { "overlay-ui-x11" }

Write-Host ""
Write-Host "=== overlay-ui — prévisualisation ($binName) ===" -ForegroundColor Cyan
Write-Host "Ctrl+Shift+W = bascule interactif / clic-traversant  |  Ctrl+Shift+R = rafraîchir  |  Ctrl+Shift+Q ou Ctrl+C = quitter" -ForegroundColor Cyan
Write-Host ""

# Le binaire vise la PROD par défaut (`overlay_sync::client::DEFAULT_BASE_URL`, décision du
# 2026-09-15) ; la prévisualisation locale reste le seul usage du déploiement dev. Une valeur déjà
# posée dans l'environnement (ex. `wrangler pages dev` local) est respectée.
if (-not $env:WAKFU_COMPANION_API_URL) {
    $env:WAKFU_COMPANION_API_URL = "https://claude-dev.wakfu-companion.com"
}
Write-Host "API : $env:WAKFU_COMPANION_API_URL" -ForegroundColor DarkGray
Write-Host ""

$cargoArgs = @("run", "-p", "overlay-ui", "--bin", $binName)
if ($Release) { $cargoArgs += "--release" }
elseif (-not $Debug) { $cargoArgs += @("--profile", "preview") }
if ($LogPath) { $cargoArgs += @("--", $LogPath) }

& cargo @cargoArgs
