# Feuille de route

## Décidé

- Langage : Rust, confirmé le 2026-11-24 après comparaison avec C, C++ et assembleur (le langage pèse ≈ ×1 sur la performance ; Rust interdit les erreurs de concurrence). Pas de variante SIMD dans les essais.
- Dépôt GitHub public `Genoflora`, open source (MIT), à vocation scientifique (2026-11-24).
- Finalité (2026-11-24, précisée) : un simulateur de timelapse de plante crédible. La croissance doit
  bouger comme dans les timelapses réels (racines qui explorent, bourgeons qui se forment, tige qui
  tourne, cycle de vie complet), sans reproduire une espèce réelle. Génome simplifié : quelques
  modules inspirés du réel suffisent. La science sert la crédibilité du mouvement, pas l'exactitude.
- Temps (2026-11-24, précisé) : toujours en accéléré, comme un timelapse (une seconde affichée =
  des minutes à des heures de plante). Le temps réel n'a pas d'intérêt. Conséquence : il faut
  plusieurs pas de simulation par image, donc le débit (pas par seconde) compte encore plus ; le
  précalcul puis replay reste un repli naturel.
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
  hybride de la note (cellules fines aux pointes, tissu ancien mis à jour plus rarement).
- Interaction (2026-11-24) : aucune. On regarde la plante pousser, on n'agit pas (couper, tailler,
  c'est le domaine d'un autre projet, le jeu de bonsaï).
- Rendu (2026-11-24) : cellulaire, jamais une surface lisse posée par-dessus. On zoome de la
  plante entière jusqu'à la cellule, et l'on voit la vie interne : ressources qui circulent,
  photosynthèse, stomates qui s'ouvrent, eau aspirée, cellules qui meurent. Les cellules ne sont
  pas des boules : elles s'étirent et prennent des formes.
- Conséquence sur le modèle hybride : le tissu « figé » garde ses cellules (on doit pouvoir y
  zoomer) ; il est seulement mis à jour moins souvent, pas remplacé par de la géométrie.
- Type de plante (2026-11-24) : aucun n'est fixé, le génome décide (herbe, buisson ou arbre).
- Échelle (2026-11-24) : vraies tailles de cellules, donc une petite plante (quelques centimètres
  pour environ un million de cellules). Le zoom montre de vraies cellules. Conséquence : un génome
  d'arbre ne montrera qu'un très jeune plant, jamais un arbre adulte.
- Départ (2026-11-24) : la graine, germination comprise ; la plante va jusqu'à produire ses
  graines. Porte ouverte vers les générations et les mutations.
- Après les graines (2026-11-24) : au choix, on continue avec la même plante ou l'on repart d'une
  de ses graines. But lointain, design pas encore mûr : la graine doit elle-même émerger des
  cellules (piste : un programme de floraison fait apparaître des cellules germinales qui copient
  le génome avec mutation, s'entourent d'un tégument, cessent de se diviser et se détachent ; la
  « graine » est un état atteint par des cellules, pas un objet codé).
- Sol (2026-11-24) : semi-transparent, en volume. On voit les racines en 3D à travers la terre,
  pas dans une simple coupe.
- Reproductibilité (2026-11-24) : graine numérique + fichier génome = exactement la même plante sur
  la même machine ; à de petits écarts près entre machines (le GPU ne garantit pas les mêmes bits).
  Mesures exportables.
- Architecture de calcul (2026-11-25, après les trois essais) : la plante tourne à 100 % sur le
  GPU (`geno_gpu`). `geno_sim` est un juge CPU réservé à la vérification : chaque règle y naît,
  les tests comparent le moteur GPU au juge. Le juge est optionnel et n'entre pas dans le produit
  final. Détail : `docs/contexte/Parallélisme.md`.
- Principe d'émergence (2026-11-25, précisé par Mickaël) : la nature est le juge, pas le plan. Le
  point de départ est simple et en partie aléatoire ; les chiffres du réel (137,5° entre deux
  feuilles, période de circumnutation) ne sont jamais des objectifs codés ni des réglages de
  départ, seulement des mesures de vérification faites après coup.
- Ordre des jalons (2026-11-25) : la germination d'abord (une graine devient une pointe, la tige
  monte, la racine descend), les feuilles ensuite, sans angle imposé. L'ancien jalon « phyllotaxie
  à 137,5° ± 5° » est abandonné comme objectif ; l'angle reste une mesure de vérification.

## Ouvert

- Rendu 3D : Bevy, wgpu nu ou autre. À trancher avec un tableau comparatif avant d'en avoir besoin.
- Équations du premier modèle d'auxine : partir de Jönsson 2006 ou Smith 2006.
- Forme des cellules : comment elles s'étirent. Pistes : ellipsoïdes qui s'allongent selon un
  axe ; ou rendu par cellules de Voronoï autour des centres (polyèdres jointifs, comme un vrai
  tissu, sans changer la mécanique) ; ou cellules déformables (modèle à sommets, plus cher).
- Rendu 3D multi-échelle : de la plante à la cellule, avec niveaux de détail.

## Prochaine étape

Le socle est en place (2026-11-25) : juge CPU et moteur GPU font pousser le même tissu de test
(sphères, une hormone, division), comparés dans `check.ps1`.

Jalon suivant : la germination. Critères mesurés, sans chiffre du réel imposé : la tige monte,
la racine descend, la croissance se concentre sous la pointe. Règles locales à valider avant de
coder.

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

## Signature visuelle d'un timelapse (tests d'émergence, non validés)

Ce qui fait qu'un timelapse « a l'air vivant ». Règle absolue : aucun de ces comportements n'est codé.
Chacun doit émerger des règles locales des cellules ; il sert seulement de test qui mesure le
résultat (par exemple : trajectoire de la pointe périodique, sans aucun sinus dans le code).

- Circumnutation : la pointe décrit des ellipses en poussant (souvenir : période d'une à quelques
  heures selon l'espèce).
- Croissance concentrée juste sous la pointe (zone d'élongation), le reste se fige.
- Feuilles qui montent la nuit et s'abaissent le jour (nyctinastie).
- Tige qui se réoriente vers la lumière en quelques heures.
- Racines qui explorent, se ramifient, contournent les obstacles.
- Bourgeons qui gonflent puis s'ouvrent, puis fleur, graine, déclin.

Piste pour la circumnutation (souvenir, à revérifier) : une hypothèse classique la fait naître
d'une boucle de gravitropisme avec retard. La tige penche, l'auxine migre vers le côté bas avec un
délai, le côté bas pousse plus, la tige dépasse la verticale, et ainsi de suite (Israelsson et
Johnsson 1967 ; voir aussi Bastien et al. 2013, graviproprioception). Il faudrait donc dans le
modèle : un transport d'auxine qui prend du temps, et une croissance différente selon le flanc de
la tige. L'ellipse en sortirait sans être écrite.
