# Feuille de route

## Décidé

- Langage : Rust (workspace, crate de simulation pur et sans dépendance).
- Dépôt GitHub privé `Genoflora`.

## Ouvert

- Rendu 3D : Bevy, wgpu nu ou autre. À trancher avec un tableau comparatif avant d'en avoir besoin.
- Parallélisme : trois essais décidés (CPU, GPU, CPU + GPU), voir `Parallélisme.md`.
- Équations du premier modèle d'auxine : partir de Jönsson 2006 ou Smith 2006.

## Prochaine étape

1. Les trois essais de calcul (CPU, GPU, CPU + GPU) : suivre `docs/Plan des essais.md`, section 7.
   Squelette prêt dans `Spikes/`, rien d'implémenté.
2. Puis jalon 2 de la note : phyllotaxie sur une surface, angle moyen 137,5° ± 5°, sans spirale codée.
