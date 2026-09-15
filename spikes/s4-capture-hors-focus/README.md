# Spike S4 — capturer une fenêtre Wakfu sans focus, voire occultée (Windows)

Voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §9.1 decies et §14 point 7.
Question posée : **une API de capture obtient-elle un contenu VIVANT d'une fenêtre Wakfu
(Java/JOGL) qui n'a pas le focus et qu'une autre fenêtre recouvre entièrement ?**

Ce que la vidéo du 2026-09-14 a déjà établi : le client continue de rendre en arrière-plan (chrono
du widget « Fin du tour » identique à la seconde près, focus ou non). Ce qu'elle ne pouvait pas
dire : si une capture *programmatique* — pas une capture d'écran de ce que DWM affiche — voit ce
contenu quand la fenêtre est recouverte.

**État : validé sous Windows (2026-09-14, deux passages sur la machine du mainteneur).** Les deux
API voient un contenu **vivant** d'une fenêtre Wakfu recouverte à 100 % — par l'autre client
comme par une application tierce maximisée. Une fenêtre **minimisée** est hors d'atteinte des
deux, et chacune le signale proprement. Réponse à la question posée : **oui.** Détail en §« Résultats ».

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

Machine du mainteneur, Windows 11 Pro 22631, deux clients Wakfu en combat (Pugio Letalis, Oumbra),
chacun plein écran fenêtré sur son moniteur (2560 × 1392 de zone client). Deux passages de 90 s,
un tick par seconde, les deux méthodes à chaque tick.

**Passage 1** — les deux fenêtres côte à côte, focus alterné à la main. Recoupe la vidéo du même
jour : sans premier plan mais visible, le widget change à 40 paires sur 47 (PrintWindow) et 46 sur
47 (WGC), chrono décompté seconde par seconde. WGC livre 2 images par tick, en continu.

**Passage 2** — les trois gestes du protocole, l'occultation mesurée à chaque tick par la grille
`WindowFromPoint`. Différence moyenne par canal entre deux recadrages « widget » consécutifs
(seuil de changement : 0,5) :

| Situation | PrintWindow | WGC |
| --- | --- | --- |
| Recouverte ≥ 90 % par l'autre client Wakfu (Pugio, t = 6–18 et 23–30) | vivant, chrono « Prêt » puis « Fin du tour » | vivant, 2 images/tick |
| Recouverte ≥ 90 % par une **application tierce maximisée**, les deux fenêtres, aucune au premier plan (t = 37–42) | **96 → 95 → 94 → 93 → 92 → 91 s**, 5/5 paires changent, les deux fenêtres | **97 → 92 s**, 5/5, les deux fenêtres |
| Minimisée (Pugio t = 45–50, Oumbra t = 53–57) | **aucune image** — `PrintWindow` renvoie faux | image **figée** sur la dernière reçue, **0 image** livrée par DWM pendant la minimisation |

Totaux sous occultation (minimisée exclue) : Oumbra 13/13 paires changent (PrintWindow), 12/13
(WGC) ; Pugio 15/25 et 14/25 — les paires immobiles de Pugio sont les ticks 8–16, phase de
placement où le widget est un fond quasi noir avec un seul petit chiffre qui bouge, sous le seuil ;
la planche des captures le montre vivant.

Ce qu'on retient pour l'implémentation :

- **Les deux API conviennent ; `PrintWindow` est retenue pour la v1.** Synchrone, à la demande,
  aucune session à tenir par fenêtre, aucun liseré, et l'image de l'instant même — WGC, dans ce
  spike, est en retard d'environ une seconde (dernière image du pool, drainée à 1 Hz), et sous
  Windows 11 peut dessiner un liseré jaune. WGC reste le repli documenté si `PrintWindow` échouait
  sur une autre configuration graphique, avec un atout propre : « 0 image reçue » dit à lui seul
  que la fenêtre ne vit plus.
- **Minimisée = non couvert**, à annoncer tel quel : `PrintWindow` échoue (à traiter comme « pas de
  lecture ce tick »), et `IsIconic` le dit avant même d'essayer.
- **Coût** : `PrintWindow` rend toute la fenêtre (14 Mo de DIB en 2560 × 1392), mais `GetDIBits`
  sait ne copier que les dernières lignes (`startScan`/`cLines`) — le widget vit en bas, une bande
  de 200 lignes suffit, soit 2 Mo par lecture. À 2 Hz sur deux fenêtres, négligeable.
- Un tick de ce spike dure ~1,5 s au lieu de 1 : c'est l'écriture de huit PNG plein cadre par tick,
  pas la capture.

**Hors périmètre de ce spike** : Linux / X11 (`XCompositeNameWindowPixmap`), à valider séparément
sur une machine Linux — le mainteneur teste sous SteamOS (voir `scripts/ci-local.sh`).
