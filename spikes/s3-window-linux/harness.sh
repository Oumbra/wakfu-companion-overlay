#!/usr/bin/env bash
# Harnais du spike S3 (docs/plan-architecture.md §17.2) — orchestre un scénario complet, rejouable
# et NON interactif : Xvfb + WM EWMH (openbox) + fenêtre de jeu factice (xterm titré) + overlay réel
# (`s3-window-linux`), pilotage par `xdotool`, vérifications programmatiques par `probe` (jamais une
# déduction depuis une capture d'écran), artefact final pour revue humaine.
#
# Prérequis système (voir README.md § « Prérequis système ») : xvfb, openbox, xdotool, xterm,
# picom (optionnel, scénario compositeur), imagemagick (`import`), ffmpeg (optionnel, vidéo).
#
# Usage : ./harness.sh [--with-compositor] [--video]
#   --with-compositor   lance picom (backend xrender) — scénario secondaire de transparence ARGB32
#                        (voir §17.2 du plan) ; SANS cette option (par défaut), le scénario est
#                        « sans compositeur », qui doit rester le cas nominal du harnais.
#   --video              capture une courte vidéo (ffmpeg -x11grab) au lieu d'une image fixe.
#
# Sortie : `artifact.png` (ou `artifact.mp4`) dans ce dossier, plus le code de sortie du script
# (0 = toutes les assertions passées).
set -euo pipefail
cd "$(dirname "$0")"

WITH_COMPOSITOR=0
CAPTURE_VIDEO=0
for arg in "$@"; do
  case "$arg" in
    --with-compositor) WITH_COMPOSITOR=1 ;;
    --video) CAPTURE_VIDEO=1 ;;
    *) echo "argument inconnu : $arg" >&2; exit 2 ;;
  esac
done

# Display dédié à CE harnais — jamais celui d'une session interactive en cours (évite tout
# télescopage avec un `:99` déjà utilisé à la main pendant le développement de ce spike).
DISPLAY_NUM="${S3_HARNESS_DISPLAY:-:98}"
export DISPLAY="$DISPLAY_NUM"
export XDG_RUNTIME_DIR="$(mktemp -d)"
chmod 700 "$XDG_RUNTIME_DIR"

GAME_TITLE="Sagittarius Caecus - WAKFU"
OTHER_TITLE="Autre appli non jeu"

# --- Cycle de vie des process : guard qui les termine TOUS, même en cas d'échec d'assertion en
# cours de route (réserve de l'expert X11, §17.6 du plan — un test précédent en échec ne doit
# jamais laisser un display corrompu qui ferait échouer le suivant sans rapport avec le bug réel).
PIDS=()
cleanup() {
  local status=$?
  for pid in "${PIDS[@]:-}"; do
    kill -9 "$pid" >/dev/null 2>&1 || true
  done
  wait >/dev/null 2>&1 || true
  rm -rf "$XDG_RUNTIME_DIR"
  exit "$status"
}
trap cleanup EXIT INT TERM

spawn() {
  "$@" &
  PIDS+=("$!")
}

assert_eq() {
  local label="$1" expected="$2" actual="$3"
  if [[ "$actual" != "$expected" ]]; then
    echo "ÉCHEC : $label — attendu '$expected', obtenu '$actual'" >&2
    exit 1
  fi
  echo "OK : $label = $actual"
}

probe_field() {
  # $1 = xid, $2 = nom du champ (clé=valeur, une par ligne — voir probe.rs)
  ./target/debug/probe "$1" | sed -n "s/^$2=//p"
}

echo "=== Build ==="
cargo build --quiet

echo "=== Xvfb ($DISPLAY_NUM) ==="
spawn Xvfb "$DISPLAY_NUM" -screen 0 1280x800x24 +extension RENDER +extension COMPOSITE +extension XTEST
sleep 1

echo "=== Gestionnaire de fenêtres EWMH (openbox) ==="
# Sans lui, `_NET_CLIENT_LIST`/`_NET_CLIENT_LIST_STACKING`/`_NET_WM_STATE_ABOVE` n'existent pas —
# voir README.md, prérequis n°2. Xvfb seul n'en fournit aucun.
spawn openbox
sleep 1

if [[ "$WITH_COMPOSITOR" -eq 1 ]]; then
  echo "=== Compositeur (picom --backend xrender) — scénario secondaire transparence ARGB32 ==="
  spawn picom --backend xrender --config /dev/null
  sleep 1
else
  echo "=== Scénario par défaut : SANS compositeur (repli opaque attendu, §6.4 du plan) ==="
fi

echo "=== Fenêtre de jeu factice (xterm) ==="
spawn xterm -T "$GAME_TITLE" -geometry 80x24+150+150 -e sleep 3600
sleep 1
GAME_XID=$(xdotool search --name "$GAME_TITLE" | head -1)
[[ -n "$GAME_XID" ]] || { echo "ÉCHEC : fenêtre de jeu factice introuvable" >&2; exit 1; }
echo "fenêtre de jeu factice : XID=$GAME_XID"

echo "=== Overlay (s3-window-linux) ==="
spawn env RUST_LOG=warn,wgpu_hal=info ./target/debug/s3-window-linux
sleep 1
OVERLAY_XID=$(xdotool search --name "Spike S3" | head -1)
[[ -n "$OVERLAY_XID" ]] || { echo "ÉCHEC : fenêtre overlay introuvable" >&2; exit 1; }
echo "overlay : XID=$OVERLAY_XID"
# Laisse au moins deux tours de sondage (POLL_INTERVAL = 50 ms) passer avant la première
# assertion — jamais un `sleep` fixe suivi d'une lecture unique sans marge (réserve de l'expert
# X11 sur l'asynchronisme de `xdotool`/la coalescence des événements X11).
sleep 0.3

echo "=== Assertions : ancrage ==="
GAME_GEOM=$(xdotool getwindowgeometry --shell "$GAME_XID")
GAME_X=$(sed -n 's/^X=//p' <<<"$GAME_GEOM")
OVERLAY_GEOM=$(xdotool getwindowgeometry --shell "$OVERLAY_XID")
OVERLAY_X=$(sed -n 's/^X=//p' <<<"$OVERLAY_GEOM")
# Ancré au bord gauche + marge (voir `GAME_EDGE_MARGIN_PX`, 12 px) — tolérance de quelques pixels
# pour la marge de décoration WM sur l'overlay lui-même (non décoré, mais openbox peut réserver un
# pixel de bordure selon le thème), jamais un test au pixel près.
DIFF=$(( OVERLAY_X - GAME_X ))
if (( DIFF < 8 || DIFF > 20 )); then
  echo "ÉCHEC : ancrage — overlay.x - jeu.x = $DIFF, attendu ~12" >&2
  exit 1
fi
echo "OK : ancrage — overlay.x - jeu.x = $DIFF (≈ GAME_EDGE_MARGIN_PX)"

echo "=== Assertions : découverte par titre (panneau affiche le personnage) ==="
# Vérification protocolaire indirecte : le panneau lit `found_character` en interne (pas exposé au
# protocole X) — ce que ce harnais PEUT vérifier par le protocole, c'est que l'overlay a bien migré
# vers la position de la fenêtre de jeu (assertion d'ancrage ci-dessus), preuve que `scan()` a
# trouvé la fenêtre et extrait son rectangle. Le contenu textuel du panneau (nom affiché) reste
# vérifié VISUELLEMENT sur l'artefact produit en fin de script — voir README.md pour la limite
# assumée (pas d'introspection du texte peint par egui depuis l'extérieur, sans OCR).

echo "=== Assertions : click-through (extension Shape) ==="
assert_eq "shape_input_rects (interactif, défaut)" "1" "$(probe_field "$OVERLAY_XID" shape_input_rects)"
xdotool key --clearmodifiers ctrl+alt+w
sleep 0.2
assert_eq "shape_input_rects (après 1 bascule)" "0" "$(probe_field "$OVERLAY_XID" shape_input_rects)"
xdotool key --clearmodifiers ctrl+alt+w
sleep 0.2
assert_eq "shape_input_rects (après 2 bascules)" "1" "$(probe_field "$OVERLAY_XID" shape_input_rects)"

echo "=== Assertions : topmost / focus-aware ==="
xdotool windowactivate "$GAME_XID"
sleep 0.2
assert_eq "is_active (jeu actif)" "true" "$(probe_field "$GAME_XID" is_active)"
TOTAL=$(probe_field "$OVERLAY_XID" stacking_total)
assert_eq "stacking_index (overlay au-dessus, jeu actif)" "$(( TOTAL - 1 ))" "$(probe_field "$OVERLAY_XID" stacking_index)"

echo "=== Fenêtre non-jeu + délai de grâce (démotion) ==="
spawn xterm -T "$OTHER_TITLE" -geometry 40x10+700+400 -e sleep 3600
sleep 1
OTHER_XID=$(xdotool search --name "$OTHER_TITLE" | head -1)
xdotool windowactivate "$OTHER_XID"
sleep 0.2
TOTAL=$(probe_field "$OVERLAY_XID" stacking_total)
BEFORE_GRACE=$(probe_field "$OVERLAY_XID" stacking_index)
assert_eq "pas encore démoté (avant le délai de grâce de 1,5 s)" "$(( TOTAL - 1 ))" "$BEFORE_GRACE"
sleep 2
AFTER_GRACE=$(probe_field "$OVERLAY_XID" stacking_index)
if [[ "$AFTER_GRACE" == "$(( TOTAL - 1 ))" ]]; then
  echo "ÉCHEC : toujours au sommet après le délai de grâce (démotion attendue)" >&2
  exit 1
fi
echo "OK : démoté après le délai de grâce (stacking_index=$AFTER_GRACE, était $(( TOTAL - 1 )))"

echo "=== Réaffirmation immédiate au retour de focus ==="
xdotool windowactivate "$GAME_XID"
sleep 0.2
TOTAL=$(probe_field "$OVERLAY_XID" stacking_total)
assert_eq "réaffirmé immédiatement" "$(( TOTAL - 1 ))" "$(probe_field "$OVERLAY_XID" stacking_index)"

echo "=== Artefact de revue humaine ==="
if [[ "$CAPTURE_VIDEO" -eq 1 ]]; then
  ffmpeg -y -f x11grab -video_size 1280x800 -i "$DISPLAY_NUM" -t 3 artifact.mp4 >/tmp/s3-harness-ffmpeg.log 2>&1
  echo "vidéo : $(pwd)/artifact.mp4"
else
  import -window root artifact.png
  echo "image : $(pwd)/artifact.png"
fi

echo "=== Toutes les assertions sont passées ==="
