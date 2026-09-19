# Analyse RGPD — reste à faire par le mainteneur, hors code

Reliquat de [`analyse-rgpd.md`](analyse-rgpd.md) qui ne se traite ni dans l'overlay ni dans le
site : des gestes sur l'historique git et de la documentation interne, établis le 2026-09-18. Les
autres volets : [`analyse-rgpd-overlay.md`](analyse-rgpd-overlay.md) et
[`analyse-rgpd-site.md`](https://github.com/Oumbra/wakfu-companion/blob/claude/dev/docs/analyse-rgpd-site.md) (dépôt du site).

Ces gestes sont **interdits à une session Claude** par le `CLAUDE.md` (pas de `push --force` sur
`dev`, jamais de push sur `main`) : ils sont à faire depuis le poste du mainteneur.

| Prio | Tâche | Constat |
| --- | --- | --- |
| **P0** | ✅ **Réécriture d'historique — faite le 2026-09-19.** `git filter-repo` (option « remplacer le contenu », préparée en session Claude dans un clone miroir, vérifiée, puis poussée par le mainteneur : `push --atomic --force` de `dev`, `main` et `v0.22.0`). 992 commits réécrits sur 994, racine signée conservée ; blob réel `4aff23f` remplacé par la fixture pseudonymisée, valeurs recopiées dans le code remplacées par les mêmes pseudonymes (222 blobs). Anciens SHA → nouveaux : `c0168a4` → `5db868a`, `ef8e2fb` → `2700533`, `dev` `bcf9396` → `aba8d7f`, `main` `cc3fea5` → `d7175bc`, `v0.22.0` `2c7a14e` → `46bc23d`. Bilan détaillé au §3.1 de l'analyse | C1 |
| **P0** | ⏳ **Ticket GitHub Support n° 4773188, ouvert le 2026-09-19 à 12 h 45** (portail *support.github.com*, catégorie « Fonctionnalités du dépôt » → « Question générale »), en attente de réponse — à clore quand l'URL `raw` de l'ancien `c0168a4` renvoie 404. Contexte : GitHub sert encore les anciens objets à qui connaît un ancien SHA (vérifié le 2026-09-19 : l'API répond pour l'ancien `c0168a4`), et `refs/pull/1/head` (PR #1 fusionnée, ancienne tête `a308e52`) les retient. Citer le blob `4aff23fd9794247f7896013a3a404753ea655a95`, les anciens commits `c0168a4c94b78ecd79b11055257061f6940e0b28` et `ef8e2fbe693c54afa026bb6d9611bdfcc532ee81` et les anciennes pointes `bcf9396efc5eeed22bcc00e450ed97270885c7fe` (`dev`), `cc3fea5e741b72acdcff8fcbae2e6adad65d1818` (`main`), `2c7a14ee458a4f3bdf6520c198e12f33960cf82f` (`v0.22.0`) ; demander la mise à jour ou la suppression de `refs/pull/1/head`. Texte prêt dans le dossier de réécriture du 2026-09-19 | C1 |
| **P0** | **Note d'incident interne** (art. 33.5) : date de mise en public (2026-09-15), contenu exposé (jeton du client de jeu, IP locale, nom de compte Windows, 6 personnages, 119 pseudonymes et leurs messages, 12 identifiants de compte Ankama de la liste d'amis), mesures prises le 2026-09-18 (fixture pseudonymisée, garde-fous) et le 2026-09-19 (réécriture d'historique), numéro du ticket GitHub Support — note rédigée le 2026-09-19 dans le dossier de réécriture, à conserver hors dépôt | C1 |
| P1 | **Note interne de mise en balance** (art. 6.1.f, documentation du responsable de traitement) pour les pseudonymes de tiers conservés dans l'historique des comptes (participants de combat, partenaire d'échange, filtres de chat — option A du 2026-09-18) : intérêt poursuivi (historique fidèle), nécessité (pas de restitution possible sans le nom), attentes raisonnables des personnes (pseudonymes déjà visibles de tous en jeu), garanties (aucune autre donnée, visibilité limitée au titulaire du compte, retrait sur demande). Le texte publié correspondant est en place dans la politique de confidentialité du site, section 1.3 (`50adcf8`, 2026-09-18) ; cette note est son pendant interne, à conserver avec le registre des traitements | C3 |

## Conséquences de la réécriture (constatées le 2026-09-19)

- Les SHA de 992 commits ont changé ; les liens externes vers d'anciens commits sont perdus. Les SHA
  cités dans les fichiers du dépôt ont été mis à jour d'après la table `commit-map` (37 citations,
  toutes en prose ou en commentaire). La signature du commit racine, seul commit signé, est
  conservée.
- Tout clone existant doit être resynchronisé (`fetch` puis `reset --hard`, suppression des refs
  résiduelles comme `refs/original/*`, `gc --prune=now`) ou recloné ; une session cloud repart de
  toute façon d'un clone neuf.
- Le hook `post-commit` n'est pas intervenu : le contenu des commits n'a pas changé, aucun bump à
  rattraper.
