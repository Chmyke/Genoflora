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
- Génome (2026-11-24) : un programme qui mute, inspiré de vrais réseaux de gènes. Pas le génome
  entier (Arabidopsis : environ 27 000 gènes) mais ses modules connus : WUS/CLV3 (pointe),
  PIN/auxine (forme, gravitropisme), phytochromes (lumière), gibbérellines/DELLA (allongement),
  CONSTANS/FT (floraison selon la durée du jour).
- Environnement (2026-11-24) : une plante seule, le plus réel possible : lumière, gravité, eau et
  hygrométrie, photosynthèse, cycle jour-nuit, saisons, vent.
- Temps multi-échelle (conséquence) : une division cellulaire prend des heures, une saison des
  mois. Le direct montre les heures ; jours et saisons passent en accéléré, ce qui impose le modèle
  hybride de la note (cellules fines aux pointes, organes figés ailleurs).

## Ouvert

- Rendu 3D : Bevy, wgpu nu ou autre. À trancher avec un tableau comparatif avant d'en avoir besoin.
- Parallélisme : trois essais décidés (CPU, GPU, CPU + GPU), voir `Parallélisme.md`.
- Équations du premier modèle d'auxine : partir de Jönsson 2006 ou Smith 2006.
- Questions de fond restantes : rendu (cellules visibles ou surface lisse), actions de
  l'utilisateur pendant la pousse, code ouvert ou privé.

## Prochaine étape

1. Les trois essais de calcul (CPU, GPU, CPU + GPU) : suivre `docs/Plan des essais.md`, section 7.
   Squelette prêt dans `Spikes/`, rien d'implémenté.
2. Puis jalon 2 de la note : phyllotaxie sur une surface, angle moyen 137,5° ± 5°, sans spirale codée.

## Ordre proposé des facteurs d'environnement (non validé)

Chacun arrive avec son module de gènes et un critère chiffré tiré d'observations réelles.

1. Gravité : la tige monte, la racine descend (auxine redistribuée).
2. Lumière et phototropisme : la tige se courbe vers une lumière latérale.
3. Photosynthèse et sucre : la croissance dépend de la lumière reçue.
4. Jour-nuit : croissance et ouverture des stomates qui suivent le cycle.
5. Eau et hygrométrie : racines, transport, flétrissement à sec.
6. Saisons : floraison déclenchée par la durée du jour (CONSTANS/FT).
7. Vent : la tige plie, et une plante secouée pousse plus courte et plus épaisse
   (thigmomorphogenèse, gènes TCH) ; critère : rapport hauteur/diamètre plus bas sous vent.
