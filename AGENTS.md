# AGENTS.md, Genoflora

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
| `geno_sim` | Juge CPU : cellules, règles écrites en clair, référence mono-thread, contrôles chiffrés | aucune |
| `geno_gpu` | Moteur GPU (wgpu + WGSL) : c'est lui qui fait tourner la plante | `wgpu`, `pollster`, `bytemuck` |

Le juge sert à vérifier, jamais à produire : il n'entre pas dans le produit final. Le rendu 3D
viendra dans un crate séparé ; la simulation n'en dépendra jamais.

## Règles du code

- Une règle biologique naît dans `geno_sim`, avec son test chiffré (jamais un critère « à l'œil »),
  puis est recopiée dans les shaders de `geno_gpu` ; `tests/against_judge.rs` compare les deux.
- Comparaison cellule par cellule sur horizon court seulement (un tissu qui se divise est
  chaotique) ; au-delà, comparer forme, effectif et masse. Les forces restent continues.
- Simulation déterministe : mêmes entrées, mêmes bits. Pas de `HashMap`/`HashSet` (clippy les
  refuse), pas d'horloge, tout aléa vient d'une graine.
- `unsafe` interdit, pas d'`unwrap` en logique (lints du workspace).
- Aucun comportement visible n'est codé (spirale des feuilles, circumnutation, tropismes) : il
  émerge des règles locales des cellules ; les tests mesurent le résultat, jamais la règle.
- La nature sert à vérifier, jamais à construire : on code des causes locales (ce qu'une cellule
  sent et fait), jamais un résultat. Aucun chiffre observé dans la nature (angle, période, rapport)
  n'entre dans une règle ni dans le génome de départ ; il sert seulement, après coup, à mesurer si
  ce qui a émergé ressemble au réel.
- Ne pas figer le modèle sur « seule la pointe vit » : des cellules anciennes peuvent grandir à leur
  rythme (épaississement du tronc).
