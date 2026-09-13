# Reproduit EN LOCAL l'environnement de rendu exact du CI — celui sous lequel les références de
# `crates/overlay-testkit/tests/snapshots/` sont comparées, et donc le seul dans lequel les
# régénérer.
#
# Le CI ne construit PAS cette image : son job `test-linux` tourne dans l'image Ubuntu de base
# épinglée par digest et appelle directement `scripts/setup-render-env.sh` (voir
# `.github/workflows/ci.yml`). Ce Dockerfile applique le MÊME script à la MÊME base, pour qu'un
# poste de dev — quelle que soit sa distribution, y compris SteamOS — obtienne un rendu identique
# au CI sans toucher à son système.
#
# Le plus simple est de passer par `scripts/ci-local.sh`, qui fait les deux pour vous :
#
#   bash scripts/ci-local.sh --captures-conteneur                      # vérifier
#   UPDATE_SNAPSHOTS=1 bash scripts/ci-local.sh --captures-conteneur   # régénérer les références
#
# À la main, si besoin (le dépôt monté au MÊME chemin : les empreintes de cargo contiennent des
# chemins absolus, donc `target/` est réutilisé tel quel au lieu de tout recompiler) :
#
#   docker build -t wakfu-ci-render -f .github/ci-image/Dockerfile .
#   docker run --rm -v "$PWD:$PWD" -w "$PWD" wakfu-ci-render \
#     cargo test --no-fail-fast -p overlay-testkit
#
# Le contexte de build est la RACINE du dépôt (`-f .github/ci-image/Dockerfile .`) : le script
# installé ici est celui du dépôt, jamais une copie. `.dockerignore` réduit ce contexte au seul
# script — sans lui, `docker build` enverrait aussi les gigaoctets de `target/`.

# **Le digest par défaut est CELUI du `container.image` du job `test-linux`**, pas le tag
# `ubuntu:24.04` : l'image locale est ainsi celle du CI par construction, et non « celle qu'on
# espère identique ». Les deux valeurs doivent bouger ensemble — un digest changé d'un seul côté
# fait diverger le rendu local de celui qui décide du gate.
#
# Surchargeable (`--build-arg BASE=…`) pour un miroir de registre, jamais pour « prendre la
# dernière ».
ARG BASE=ubuntu:24.04@sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254
FROM ${BASE}

# Une seule source de vérité pour les versions : le script du dépôt, celui-là même que le CI
# exécute. Copié plutôt que recopié — une liste de paquets dupliquée ici finirait par diverger.
COPY scripts/setup-render-env.sh /tmp/setup-render-env.sh
RUN set -eux; \
    bash /tmp/setup-render-env.sh; \
    rm -f /tmp/setup-render-env.sh; \
    rm -rf /var/lib/apt/lists/*

# `rustup` — absent d'une image Ubuntu nue. Aucune toolchain n'est installée volontairement : c'est
# `rust-toolchain.toml` qui décide de la version (voir sa doc), et rustup la récupère au premier
# `cargo` lancé dans le conteneur. Épingler une toolchain DANS l'image dédoublerait cette source de
# vérité, et c'est exactement la dérive que ce fichier existe pour empêcher.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --default-toolchain none; \
    chmod -R a+rwX /usr/local/rustup /usr/local/cargo; \
    rustup --version
