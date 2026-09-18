# Analyse RGPD — reste à faire par le mainteneur, hors code

Reliquat de [`analyse-rgpd.md`](analyse-rgpd.md) qui ne se traite ni dans l'overlay ni dans le
site : des gestes sur l'historique git et de la documentation interne, établis le 2026-09-18. Les
autres volets : [`analyse-rgpd-overlay.md`](analyse-rgpd-overlay.md) et
[`analyse-rgpd-site.md`](analyse-rgpd-site.md).

Ces gestes sont **interdits à une session Claude** par le `CLAUDE.md` (pas de `push --force` sur
`dev`, jamais de push sur `main`) : ils sont à faire depuis le poste du mainteneur.

| Prio | Tâche | Constat |
| --- | --- | --- |
| **P0** | **Réécriture d'historique.** Au 2026-09-18, `c0168a4` contient toujours le vrai `wakfu.log` (jeton réel, liste d'amis avec identifiants de compte Ankama) et est atteignable depuis `dev`, `main` et le tag `v0.22.0`. Recommandé : `git filter-repo --blob-callback` remplaçant l'ancien blob par la fixture pseudonymisée (le fichier reste présent partout où il l'était, seul le contenu sensible devient introuvable), force-push `dev` et `main`, redéfinir le tag `v0.22.0` et la Release, puis **ticket GitHub Support** citant les anciens SHA pour déclencher le ramassage — GitHub sert les anciens objets à qui connaît le SHA tant que ce n'est pas fait. Le dépôt a **0 fork** (vérifié via l'API le 2026-09-18) : aucune copie externe à traiter, c'est le bon moment. Détail des trois options au §3.1 de l'analyse | C1 |
| **P0** | **Note d'incident interne** (art. 33.5) : date de mise en public (2026-09-15), contenu exposé (jeton du client de jeu, IP locale, nom de compte Windows, 6 personnages, 119 pseudonymes et leurs messages, 12 identifiants de compte Ankama de la liste d'amis), mesures prises le 2026-09-18 (fixture pseudonymisée, garde-fous, réécriture d'historique et sa date) | C1 |
| P1 | **Note interne de mise en balance** (art. 6.1.f, documentation du responsable de traitement) pour les pseudonymes de tiers conservés dans l'historique des comptes (participants de combat, partenaire d'échange, filtres de chat — option A du 2026-09-18) : intérêt poursuivi (historique fidèle), nécessité (pas de restitution possible sans le nom), attentes raisonnables des personnes (pseudonymes déjà visibles de tous en jeu), garanties (aucune autre donnée, visibilité limitée au titulaire du compte, retrait sur demande). Le texte publié correspondant est en place dans la politique de confidentialité du site, section 1.3 (`50adcf8`, 2026-09-18) ; cette note est son pendant interne, à conserver avec le registre des traitements | C3 |

## Conséquences de la réécriture à anticiper

- Les SHA de 934 commits changent ; les liens vers d'anciens commits et les signatures des commits
  réécrits (non re-signables par `filter-repo`) sont perdus.
- Tout clone existant doit être re-cloné (les sessions Claude locales comprises).
- Le hook `post-commit` ne bumpe pas pendant une manipulation d'historique ; rien à rattraper ici
  puisque le contenu des commits ne change pas.
