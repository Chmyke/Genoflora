# CLAUDE.md, Genoflora

Simulation d'une plante qui pousse en 3D à partir de cellules portant toutes le même pseudo ADN
(gènes, hormones, auxine en tête). Délire numérique plus que recherche : ni jeu, ni fond d'écran.
La forme n'est écrite nulle part, elle émerge des règles locales des cellules (l'opposé des
L-systems et du procédural à graine).

## Méthode

Le socle est chargé par le fichier global. Lire `docs/contexte/INDEX.md`, puis le domaine
concerné, avant de proposer ou modifier ; tenue de la mémoire : `Méthode/noyau/11`.
Modules de ce projet, à lire avant d'écrire du code : `Méthode/noyau/07`, `Méthode/modules/rust.md`.
Vérification : `powershell -NoProfile -ExecutionPolicy Bypass -File check.ps1` (format, clippy sans
avertissement, tests). Tout contrôle neuf s'y rattache. `cargo` se lance depuis PowerShell.

## Structure

| Crate | Rôle | Dépendances externes |
|---|---|---|
| `geno_sim` | Cellules, génome, mitose ; toute règle de simulation | aucune |

Le rendu 3D viendra dans un crate séparé ; la simulation n'en dépendra jamais.

## Règles du code

- Une règle biologique naît dans `geno_sim`, avec son test chiffré (jamais un critère « à l'œil »).
- Simulation déterministe : mêmes entrées, mêmes bits. Pas de `HashMap`/`HashSet` (clippy les
  refuse), pas d'horloge, tout aléa vient d'une graine.
- `unsafe` interdit, pas d'`unwrap` en logique (lints du workspace).
- Aucun comportement visible n'est codé (spirale des feuilles, circumnutation, tropismes) : il
  émerge des règles locales des cellules ; les tests mesurent le résultat, jamais la règle.
- Ne pas figer le modèle sur « seule la pointe vit » : des cellules anciennes peuvent grandir à leur
  rythme (épaississement du tronc).
