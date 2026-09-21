# Sons d'alerte

Embarqués par `crates/overlay-ui/src/alert_sound.rs` (`include_bytes!`). Ce sont les fichiers de
l'application web (`public/assets/sounds/` du dépôt `Oumbra/wakfu-companion`), recopiés sous un nom
sans hash. **Aucun d'eux ne provient du client Wakfu** — ce sont les seuls sons de l'overlay, et la
question de la provenance se pose pour chacun (`docs/analyse-cgu.md` §3.4).

| Fichier | Rôle | Provenance (commentaire de source du dépôt web) |
| --- | --- | --- |
| `loot.mp3` | ramassage d'un objet suivi avec son activé | `AlertSound6.mp3` de https://www.filterblade.xyz/assets/sounds/ — une bibliothèque de sons de filtre pour *Path of Exile* ; les sons d'alerte natifs de ce jeu appartiennent à Grinding Gear Games, **licence non établie** |
| `countdown.mp3` | décompte à zéro d'un objet suivi | « fourni par l'utilisateur », origine non documentée |
| `chat-filter.mp3` | correspondance d'un filtre de chat | « fourni par l'utilisateur », origine non documentée |
| `turn.mp3` | notification de tour (Windows) | copie provisoire de `countdown.mp3`, à remplacer par le son que l'utilisateur choisira |

À faire par le mainteneur : documenter ici l'origine et la licence de `countdown.mp3` et
`chat-filter.mp3`, et remplacer `loot.mp3` par un son dont la licence est connue (ou en établir
une). La licence MIT du dépôt ne couvre pas ces fichiers.
