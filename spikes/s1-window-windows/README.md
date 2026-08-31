# Spike S1 — fenêtre transparente, toujours au-dessus, clic-traversant (Windows)

Voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §6.2/§12 (feuille de route,
S1). Question posée : **une fenêtre transparente, toujours au-dessus, traversable par la souris,
rendue avec wgpu+egui, est-elle atteignable sous Windows sans piège de composition ?**

**État : en cours — la composition DirectComposition fonctionne (fenêtre transparente affichée),
un bug de redimensionnement au démarrage reste à corriger avant de considérer le spike terminé.**

## Méthode

- `winit` 0.30 (fenêtre transparente, sans bordure, toujours au-dessus, `WS_EX_NOACTIVATE` +
  `WS_EX_TOOLWINDOW` posés à la main sur le HWND — non exposés par winit) + `wgpu` 30 + `egui`
  0.36, backend forcé DX12.
- Rendu via `Dx12SwapchainKind::DxgiFromVisual` (voir découverte ci-dessous) : wgpu-hal crée et
  pilote lui-même le device/target/visual DirectComposition, sans code manuel de notre côté.
- Bascule interactif / clic-traversant : hotkey global (`global-hotkey`, `Ctrl+Alt+W`, indépendant
  du focus) + `window.set_cursor_hittest()`.
- Mesure RSS via `GetProcessMemoryInfo`, affichée en continu dans la console.

Build : `cd spikes/s1-window-windows && bash patches/setup-vendor.sh && cargo build --release`
(voir §"Le patch wgpu-hal" plus bas — `vendor/` n'est pas commité, seul le patch l'est).

## Découverte n°1 — pas besoin de piloter DirectComposition à la main

Le plan v2 (§6.2) prévoyait de créer `IDCompositionDevice`/`Target`/`Visual` nous-mêmes. **Faux
besoin** : le vrai code source de `wgpu-hal` 30.0.1 (`vendor/wgpu-hal-30.0.1/src/dx12/dcomp.rs`,
lu directement — docs.rs ne documente pas les items spécifiques à Windows, cible de build par
défaut Linux) montre que `wgpu::Dx12BackendOptions { presentation_system:
Dx12SwapchainKind::DxgiFromVisual, .. }` suffit : wgpu-hal crée et gère tout en interne
(`DCompState::get_or_init`, appelé automatiquement à la configuration de la surface). Simplifie
nettement le plan — voir la mise à jour de `docs/plan-architecture.md` à faire suite à ce spike.

## Découverte n°2 — deux bugs réels dans wgpu-hal 30.0.1 pour la cible composition

Reproduits et confirmés sur cette machine (RTX 3080 Ti, driver 32.0.16.1062, Windows 11), tous
deux **hors de toute ambiguïté documentaire** (vérifiés contre la documentation Microsoft citée
ci-dessous, pas contre un comportement supposé) :

1. **`DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING` posé inconditionnellement.** Microsoft ne documente ce
   flag que pour `CreateSwapChainForHwnd` ; combiné à `CreateSwapChainForComposition`, la création
   échoue avec `DXGI_ERROR_INVALID_CALL` (0x887A0001).
2. **`SwapEffect` codé en dur à `DXGI_SWAP_EFFECT_FLIP_DISCARD`.** [La documentation
   `CreateSwapChainForComposition`](https://learn.microsoft.com/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgifactory2-createswapchainforcomposition)
   est explicite : *« You must specify the DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL value in the
   SwapEffect member ... because CreateSwapChainForComposition supports only flip presentation
   model »*. `FLIP_DISCARD` fonctionne pour `CreateSwapChainForHwnd`, pas pour la composition.

Les deux sont corrigés dans `vendor/wgpu-hal-30.0.1/src/dx12/mod.rs` (voir
`patches/wgpu-hal-30.0.1-directcomposition.patch`), conditionnés sur
`is_composition_target`. Une troisième piste (`DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT`
également incompatible) a été explorée par prudence mais **n'a finalement pas été nécessaire** —
voir la découverte n°3.

Ces deux bugs sont probablement à remonter en amont sur
[`gfx-rs/wgpu`](https://github.com/gfx-rs/wgpu/issues) — pas encore fait à ce stade.

## Découverte n°3 — la vraie cause restante : `AlphaMode`, pas un bug wgpu-hal

Avec les deux corrections ci-dessus, `CreateSwapChainForComposition` échouait **encore** avec le
même code d'erreur, alors que chaque champ du descripteur respectait déjà toutes les exigences
documentées. Plutôt que de continuer à empiler des hypothèses sur wgpu-hal, un repro Win32/DXGI
**brut**, indépendant de `wgpu`/`wgpu-hal` (`src/raw_repro.rs`), a permis de trancher
définitivement :

| Test (`cargo run --release --bin raw-repro`) | AlphaMode | Résultat |
| --- | --- | --- |
| `CreateSwapChainForHwnd` (contrôle) | `IGNORE` | ✅ OK |
| `CreateSwapChainForComposition` | `STRAIGHT` (= `wgpu::CompositeAlphaMode::PostMultiplied`) | ❌ `DXGI_ERROR_INVALID_CALL` |
| `CreateSwapChainForComposition` | `PREMULTIPLIED` | ✅ OK |

**`DXGI_ALPHA_MODE_STRAIGHT` est rejeté par ce pilote pour une swapchain composition** — non
documenté comme tel par Microsoft (les deux valeurs sont présentées comme également valides), mais
reproductible et net. Notre choix initial (`CompositeAlphaMode::PostMultiplied`, dans `main.rs`)
était donc la vraie cause de l'échec observé — **pas un bug wgpu-hal**. Corrigé en passant à
`CompositeAlphaMode::PreMultiplied`, qui correspond de toute façon à ce qu'`egui_wgpu::Renderer`
produit déjà en interne (blending alpha prémultiplié) : aucune conversion de couleur nécessaire
côté rendu.

**Méthode à retenir** pour la suite du projet : quand une erreur DXGI générique résiste à plusieurs
corrections plausibles, écrire un repro minimal hors de toute abstraction (`wgpu`/`wgpu-hal`) est
allé plus vite que d'empiler des hypothèses sur le code emprunté — le repro (`raw_repro.rs`) a
tranché en un seul run là où trois itérations sur wgpu-hal n'avaient fait que déplacer le symptôme.

## Diagnostic tenté et abandonné : couche de debug D3D12/DXGI

`WGPU_DEBUG=1`/`WGPU_VALIDATION=1` et une tentative directe via `IDXGIInfoQueue`
(`install_dxgi_debug_panic_hook` dans `main.rs`) n'ont produit **aucun message détaillé** (0
message stocké) : la fonctionnalité facultative Windows « Outils graphiques » (couche de debug
D3D12) n'est pas installée sur cette machine, et son installation nécessite une élévation non
tentée dans cette session. C'est ce qui a motivé le repro brut de la découverte n°3 plutôt que
d'attendre cette installation.

## État actuel — CE QUI RESTE À FAIRE

Avec les trois corrections ci-dessus, la swapchain composition est créée avec succès et la fenêtre
transparente s'affiche (`AlphaMode(1)` = `PREMULTIPLIED` confirmé dans les logs). Un nouveau
problème, **non résolu**, apparaît juste après :

```
wgpu error: Validation Error
  In Surface::configure
    `Surface` width and height must be within the maximum supported texture size.
    Requested was (3824, 984), maximum extent for either dimension is 2048.
```

Un `WindowEvent::Resized` arrive avec une taille incohérente (3824×984, ratio ~3.9:1, sans rapport
avec les 420×220 logiques demandés ni avec une mise à l'échelle DPI simple). Hypothèses non
encore testées : storm de redimensionnement transitoire lié à `with_no_redirection_bitmap(true)`
+ `WS_EX_TOOLWINDOW` à la création, ou bug d'interaction winit/DirectComposition sur le tout
premier `WM_SIZE`. **Prochaine étape** : logger `size` et `scale_factor` à chaque
`WindowEvent::Resized`, et clamper défensivement `config.width/height` à la capacité annoncée par
`surface.get_capabilities()` avant `configure()` — ce qui aurait aussi évité le crash ici (pattern
à adopter dans l'implémentation finale de toute façon, un redimensionnement excessif ne doit
jamais faire planter l'overlay).

## Prochaines étapes (une fois le redimensionnement résolu)

1. Confirmer visuellement le rendu (capture d'écran) par-dessus une fenêtre quelconque en mode
   fenêtré sans bordure.
2. Mesurer le RSS en continu (déjà instrumenté, `print_rss()`).
3. Vérifier le clic-traversant réel (`Ctrl+Alt+W`) avec une autre fenêtre focalisée dessous.
4. Retester si l'exclusion de `FRAME_LATENCY_WAITABLE_OBJECT` pour la composition (découverte
   n°2, note dans le patch) est réellement nécessaire maintenant que la vraie cause (AlphaMode)
   est connue — elle a été laissée par prudence sans être re-vérifiée isolément.
5. Ouvrir les deux issues amont sur `gfx-rs/wgpu` (ALLOW_TEARING + SwapEffect pour la cible
   composition).
6. Nettoyer les instrumentations de diagnostic (`install_dxgi_debug_panic_hook`, dump `DIAG desc`
   dans le patch) une fois le spike conclu — volontairement conservées pour l'instant, utiles à
   qui reprend l'investigation du redimensionnement.

## Le patch wgpu-hal — pourquoi il n'est pas commité tel quel

`vendor/wgpu-hal-30.0.1/` (copie complète du crate, ~2,4 Mo, un seul fichier réellement modifié)
n'est **pas** versionné : voir `.gitignore` (`spikes/*/vendor/`). Seul
[`patches/wgpu-hal-30.0.1-directcomposition.patch`](patches/wgpu-hal-30.0.1-directcomposition.patch)
l'est. `patches/setup-vendor.sh` télécharge la version pristine depuis crates.io et applique le
patch — à lancer après un clone, avant tout `cargo build` (le `[patch.crates-io]` de `Cargo.toml`
pointe sur `vendor/wgpu-hal-30.0.1`, qui n'existe pas tant que le script n'a pas tourné).
