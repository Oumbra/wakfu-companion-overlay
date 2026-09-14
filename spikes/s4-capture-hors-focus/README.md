# Spike S4 — capturer une fenêtre Wakfu sans focus, voire occultée (Windows)

Voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §9.1 decies et §14 point 7.
Question posée : **une API de capture obtient-elle un contenu VIVANT d'une fenêtre Wakfu
(Java/JOGL) qui n'a pas le focus et qu'une autre fenêtre recouvre entièrement ?**

Ce que la vidéo du 2026-09-14 a déjà établi : le client continue de rendre en arrière-plan (chrono
du widget « Fin du tour » identique à la seconde près, focus ou non). Ce qu'elle ne pouvait pas
dire : si une capture *programmatique* — pas une capture d'écran de ce que DWM affiche — voit ce
contenu quand la fenêtre est recouverte.

**État : en cours de validation** (voir §« Résultats » en bas, complété au fil des essais).

## Méthode

Deux API comparées sur les mêmes fenêtres, au même instant, sans autre différence :

- **`printwindow`** — `PrintWindow(hwnd, hdc, PW_CLIENTONLY | PW_RENDERFULLCONTENT)`. GDI, une
  cinquantaine de lignes, réputée renvoyer une image noire sur certaines surfaces OpenGL/D3D.
- **`wgc`** — Windows Graphics Capture (`Windows.Graphics.Capture`, WinRT, Windows 10 1803+).
  Passe par la composition DWM : voit ce que l'application *produit*, pas ce qui est affiché.
  D3D11 + interop WinRT (`CreateDirect3D11DeviceFromDXGIDevice`, `IGraphicsCaptureItemInterop`),
  lecture par texture staging. DWM ne livre une image que quand le contenu change : le nombre
  d'images reçues entre deux ticks dit à lui seul si le client peint encore.

Le binaire énumère les fenêtres `"<Nom> - WAKFU"` (même critère que `overlay-ui::game_window`),
les capture toutes à intervalle fixe pendant N secondes, écrit chaque capture en PNG plus un
recadrage du **coin bas-droit** (là où vit le widget « Fin du tour », dont le chrono change chaque
seconde), note à chaque tick si la fenêtre avait le premier plan, et conclut par fenêtre et par
méthode :

- **VIVANT** — le recadrage du widget change d'un tick à l'autre sur les ticks *sans* premier plan ;
- **FIGÉ** — il ne change plus dès que la fenêtre perd le premier plan ;
- **NOIR** — la méthode ne voit pas cette fenêtre.

```
cargo build --release
./target/release/s4-capture-hors-focus.exe list
./target/release/s4-capture-hors-focus.exe capture --seconds 60 --interval 1000 --out captures
./target/release/s4-capture-hors-focus.exe capture --title "Visual Studio Code" --seconds 4   # mécanique seule, sans le jeu
```

## Protocole

Deux clients Wakfu **en combat** (un mannequin suffit), puis, pendant que le binaire tourne :

| Fenêtre de temps | Geste |
| --- | --- |
| 0 – 15 s | les deux fenêtres côte à côte, visibles — référence |
| 15 – 30 s | une fenêtre **entièrement par-dessus** l'autre |
| 30 – 45 s | une **troisième application** au premier plan, maximisée, recouvrant les deux |
| 45 – 60 s | une fenêtre Wakfu **minimisée** — cas attendu hors d'atteinte, on veut le voir échouer proprement |

Le rapport final se lit par ligne ; les `*_widget.png` montrent le chrono capturé à chaque tick.

## Résultats

_À compléter._
