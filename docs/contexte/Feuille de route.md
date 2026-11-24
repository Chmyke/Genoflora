# Feuille de route

## Décidé

- Langage : Rust (workspace, crate de simulation pur et sans dépendance).
- Dépôt GitHub privé `Genoflora`.

## Ouvert

- Rendu 3D : Bevy, wgpu nu ou autre. À trancher avec un tableau comparatif avant d'en avoir besoin.
- Parallélisme : trois essais décidés (CPU, GPU, CPU + GPU), voir `Parallélisme.md`.
- Équations du premier modèle d'auxine : partir de Jönsson 2006 ou Smith 2006.

## Prochaine étape

Jalon 2 de la note : phyllotaxie sur une surface. Critère : angle moyen entre primordia successifs
à 137,5° ± 5°, sans que la spirale soit codée. D'abord le plan du prototype, puis le code.
