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

Germination, premier résultat (2026-11-25). Quatre capacités de cellule, validées par Mickaël, et
rien d'autre : sentir le bas, fabriquer l'auxine et la passer plutôt à la voisine du dessous,
grandir selon l'auxine contenue, se diviser dans une direction aléatoire.

Ce qui a émergé, mesuré (`geno_sim::checks`, images dans `docs/images/`) :

- Une colonne qui descend depuis la graine, avec l'auxine concentrée à sa pointe : une racine.
  Allongement de 2,7 à 7,3 selon le génome, 100 % des cellules sous la graine.
- La forme dépend du génome : très sensible à l'auxine, la graine reste une boule ; moins
  sensible, elle devient une colonne fine, parfois avec des départs latéraux.
- La colonne suit la gravité quelle que soit sa direction ; sans perception du bas, la graine
  reste une boule.

Ce qui n'a pas émergé : rien ne monte. Pas de tige.

Essai d'une cinquième capacité, la cytokinine (2026-11-25, validée par Mickaël) : fabriquée là où
il y a de l'auxine, elle diffuse sans direction, et fait grandir les cellules où l'auxine est
rare (rapport cytokinine/auxine, Skoog et Miller 1957, revérifié). Résultat mesuré sur sept
génomes : aucun second pôle. Faible, elle épaissit la racine ; forte, elle redonne une boule
(allongement 1,15). Dans tous les cas la plante ne dépasse pas 10 rayons au-dessus de la graine.
Raison probable : dans une vraie plante la cytokinine monte par la sève brute, aspirée par
l'évaporation des feuilles ; ici elle ne fait que diffuser et reste près de la racine. La capacité
reste dans le code, éteinte dans le génome par défaut (`ck_production = 0`).

Cellules allongées (2026-11-25, piste A1 validée par Mickaël, la piste A2 « sol et air » viendra
après, pas en même temps). Une cellule est un ellipsoïde de volume constant : elle s'étire selon
l'auxine qu'elle contient (un peu l'aide, trop la freine), son grand axe tourne vers la ligne du
flux d'auxine qui la traverse, et elle se divise plutôt en travers de sa longueur. Rien n'impose
de direction.

Mesuré (`docs/images/Cellules allongées*.png`) :

- Les cellules s'étirent (trois fois plus longues que larges) et s'alignent sur le flux : 0,90
  d'alignement avec la verticale sur trois graines, contre 0,26 à 0,55 pour un témoin dont les
  axes ne tournent pas (10 000 cellules sur GPU). Trop de rotation dégrade (0,77).
- La racine devient plus fine et plus longue (allongement 4,9 à 5,2 contre 2,7 à 3,6).
- Vus de près, des tissus de cellules longues et parallèles, comme dans une vraie tige.
- Non mesuré, à suivre : de petits foyers d'auxine apparaissent sur les flancs.

Toujours rien ne monte (0 à 5 % des cellules au-dessus de la graine). Leçon : sans sol, la graine
n'est accrochée à rien. Les cellules ne font que se pousser entre elles, donc « monter » n'a pas
de sens physique : la colonne s'étire autour de son propre centre, et seul l'endroit où les
cellules se divisent décide de quel côté de la graine elle s'étend. Le sol (piste A2) n'est donc
pas un décor : c'est ce qui donne un point d'appui pour qu'une partie puisse pousser vers le haut.

Biais corrigé en route : l'axe de la cellule graine était tiré une fois pour toutes et toutes les
cellules en héritaient. Il est maintenant tiré de la graine numérique.

Audit des mécaniques (2026-11-25, demandé par Mickaël : se rapprocher du réel sans ajouter de
capacité). Corrigé :

1. La rotation tournait à pleine vitesse au moindre flux (direction seule). Elle est maintenant
   proportionnelle à la force du flux.
2. La diffusion suivait les quantités : une grosse cellule fuyait vers une petite aussi chargée.
   Elle suit les concentrations, sous forme d'un échange qui rapproche deux cellules de leur
   équilibre (stable, la masse est conservée).
3. Une cellule s'allongeait à volume constant, comme une pâte. Une vraie cellule s'allonge en
   prenant de l'eau : l'allongement ajoute du volume, la largeur ne change pas.
4. La cellule entière pivotait. Elle ne pivote plus : seule la direction de croissance de sa
   paroi tourne, et sa forme se construit par ses croissances successives (ellipsoïde quelconque,
   `geno_sim::shape`). Une cellule se coupe en deux moitiés en travers de sa longueur. Une
   croissance égale partout la rearrondit (l'eau pousse une paroi uniforme vers la sphère) ; sans
   cela les cellules fraîchement divisées restaient des disques plats.
6. Les divisions tombaient toutes ensemble tous les 20 pas. Chaque cellule se divise maintenant
   dès qu'elle est prête (par sa taille, ou par sa longueur).

Remesuré après correction (16 000 cellules sur GPU, `docs/images/Cellules allongées*.png`) :
alignement 0,84 à 0,86 sur trois graines contre 0,32 à 0,52 pour le témoin aux axes figés ;
cellules 2,7 fois plus longues que larges ; sans allongement, cellules presque rondes (1,7) et
racine plus fine. Toujours rien ne monte.

Point 5 de l'audit, corrigé ensuite (2026-11-25, validé par Mickaël : « le plus fidèle au
réel »). Le sens du transport ne vient plus de la gravité : chaque cellule porte une polarité de
ses pompes PIN (un vecteur de longueur au plus 1, nul dans la graine), et pompe son auxine vers
les voisines qui font face à ce côté. Les pompes se déplacent lentement (la cellule s'en
souvient) vers une cible faite de trois tirages, tous à l'œuvre dans chaque cellule (Bayer et
al. 2009) :

- vers les voisines les plus riches en auxine (Jönsson et al. 2006), qui l'emporte quand la
  cellule en contient peu ;
- dans le sens du flux qui la traverse déjà (canalisation, Mitchison 1980), qui l'emporte quand
  elle en contient beaucoup ;
- vers le bas, tiré par les grains lourds de la cellule (statolithes), avec une force du génome
  (`gravisense`, 0,3 par défaut, choisi par Mickaël).

Les équations exactes de Bayer 2009 sont dans un supplément non lu : la bascule entre les deux
premiers tirages (selon l'auxine de la cellule) est la forme la plus simple de ce que dit
l'article, à revérifier. Les pompes sont représentées par un seul vecteur par cellule (pas une
quantité par paroi) : approximation au premier ordre de la répartition de Jönsson. L'ancienne
règle est le cas particulier où les pompes seraient fixées vers le bas ; elle redonne les mêmes
mesures.

Mesuré (16 000 cellules sur GPU, génome qui allonge les cellules, trois graines) :

| Force de la gravité sur les pompes | Alignement (témoin) | Allongement de la racine |
|---|---|---|
| Ancienne règle | 0,86 à 0,90 (0,28 à 0,53) | 3,6 à 4,0 |
| 1 | 0,84 à 0,90 (0,28 à 0,53) | 3,3 à 3,5 |
| 0,3 (défaut) | 0,74 à 0,80 (0,32 à 0,53) | 1,9 |
| 0,1 ou 0 (juge, 1 024 cellules) | non mesuré | boule (1,1 à 1,3) |

Constat important : sans gravité, les pompes se polarisent (0,45 à 0,89) mais rien ne
s'organise, quel que soit le poids donné au flux. Toutes les cellules fabriquent l'auxine au même
rythme et aucune ne la fait disparaître : sans source ni puits, la canalisation n'a rien à
relier. L'ancienne règle fabriquait ce puits de force en bas. Dans une vraie graine, l'axe haut-bas
existe déjà dans l'embryon (hérité de la mère) ; il ne naît pas de rien. Un test l'enregistre
(`pumps_blind_to_gravity_make_no_column_yet`) et doit casser le jour où une source et un puits
émergeront.

Vérification : la comparaison GPU contre juge couvre les pompes (exacte sur 60 pas, germination
exacte sur WARP). L'alignement ne se voit pas sur les quelques centaines de cellules du juge : il
est mesuré sur la carte (`geno_gpu/tests/emergence.rs`, sauté sans carte graphique).

Effets sur la vérification : divisions à chaque pas, donc un tissu chaotique plus tôt. La
comparaison exacte GPU contre juge tient 60 pas sur un tissu, au-delà elle est statistique
(effectif à 1 %, forme, masse) ; la germination exacte n'est comparée que sur WARP (la carte
arrondit un peu moins finement). La longueur d'une cellule est calculée par rotations de Jacobi :
la formule directe était fausse de 0,2 % pour les cellules plates, et décidait des divisions.

Limites : pas de sol ni de lumière ; le juge compare toutes les paires (petits tissus seulement) ;
la grille GPU grossit avec l'étirement maximal, donc le calcul ralentit.

Suite proposée par Claude (à trancher au début du prochain lot) : la piste A2 « sol et air »
avant une source et un puits d'auxine. Raison : décider sans environnement où l'auxine se
fabrique demanderait une règle interne arbitraire, proche de coder le résultat ; le sol et l'air
donnent à chaque cellule une vraie information locale (dans la terre ou dehors, à la lumière ou
non), d'où une source et un puits pourront découler (par exemple : une cellule exposée fabrique
plus d'auxine).

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
