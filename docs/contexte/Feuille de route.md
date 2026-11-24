# Feuille de route

## Décidé

- Langage : Rust, confirmé le 2026-11-24 après comparaison avec C, C++ et assembleur (le langage pèse ≈ ×1 sur la performance ; Rust interdit les erreurs de concurrence). Pas de variante SIMD dans les essais.
- Dépôt GitHub privé `Genoflora`.
- Finalité (2026-11-24) : un vrai simulateur, scientifique assumé, aussi fidèle que possible à la
  biologie, avec une belle interface. Conséquence : les modèles partent des publications (Jönsson,
  Smith, Gruel), avec leurs paramètres, et chaque jalon se compare à une observation réelle.
- Temps (2026-11-24) : en direct, accélérable, c'est la cible ; précalcul puis replay accepté en
  repli si la performance ne suit pas.
- Plateforme (2026-11-24) : PC Windows seul, jamais le web. Machine de référence : Ryzen 9 7950X3D
  (16 cœurs), RTX 4080 SUPER.
- API GPU, choisie par Claude (2026-11-24) : wgpu (DX12 ou Vulkan) pour les essais. Raisons : Rust
  natif, n'importe quelle carte, testable sans GPU (WARP). CUDA (possible : carte NVIDIA) reste le
  recours si l'essai GPU bute sur une limite de wgpu (atomiques flottants, somme préfixe).

## Ouvert

- Rendu 3D : Bevy, wgpu nu ou autre. À trancher avec un tableau comparatif avant d'en avoir besoin.
- Parallélisme : trois essais décidés (CPU, GPU, CPU + GPU), voir `Parallélisme.md`.
- Équations du premier modèle d'auxine : partir de Jönsson 2006 ou Smith 2006.
- Questions de fond à trancher avant le jalon concerné : forme du génome (réglages fixes ou
  programme qui mute), environnement (plante seule ou concurrence, sol, eau, gravité), rendu
  (cellules visibles ou surface lisse), actions de l'utilisateur pendant la pousse (couper,
  éclairer, injecter), code ouvert ou privé.

## Prochaine étape

1. Les trois essais de calcul (CPU, GPU, CPU + GPU) : suivre `docs/Plan des essais.md`, section 7.
   Squelette prêt dans `Spikes/`, rien d'implémenté.
2. Puis jalon 2 de la note : phyllotaxie sur une surface, angle moyen 137,5° ± 5°, sans spirale codée.
