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

- Rendu 3D (2026-11-25, tranché par Mickaël sur tableau comparatif) : wgpu nu et une fenêtre
  (winit), dans un crate séparé `geno_view`. Les cellules sont dessinées là où elles sont déjà, sur
  la carte, sans copie. Bevy écarté (grosse dépendance, copie des cellules à chaque image), la 3D
  dans une page web aussi (relecture seulement, contraire à « jamais le web »). Première version :
  la plante en direct, caméra libre, pause, mêmes couleurs que la visionneuse.

## Ouvert

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
(Réfuté le 2026-11-25 par le lot « sol et air », plus bas : l'appui ne suffit pas.)

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

Vérification : la comparaison GPU contre juge couvre les pompes (exacte sur 60 pas sur WARP, 30
sur la carte depuis le sol ; germination exacte sur WARP). L'alignement ne se voit pas sur les quelques centaines de cellules du juge : il
est mesuré sur la carte (`geno_gpu/tests/emergence.rs`, sauté sans carte graphique).

Effets sur la vérification : divisions à chaque pas, donc un tissu chaotique plus tôt. La
comparaison exacte GPU contre juge tient 60 pas sur un tissu, au-delà elle est statistique
(effectif à 1 %, forme, masse) ; la germination exacte n'est comparée que sur WARP (la carte
arrondit un peu moins finement). La longueur d'une cellule est calculée par rotations de Jacobi :
la formule directe était fausse de 0,2 % pour les cellules plates, et décidait des divisions.

Limites : le juge compare toutes les paires (petits tissus seulement) ;
la grille GPU grossit avec l'étirement maximal, donc le calcul ralentit.

Sol et air (2026-11-25, piste A2, contenu validé par Mickaël : ancrage et lumière sans capacité
nouvelle, graine juste sous la surface, frottement et dureté). Tout est de l'environnement
(`geno_sim::soil`), rien n'est une capacité de cellule :

- La terre remplit tout sous une surface plane, un rayon de division au-dessus du centre de la
  graine ; elle s'amincit sur un rayon de part et d'autre de la surface (aucun saut).
- Frottement solide : dans la terre, une cellule ne bouge que de ce que sa poussée dépasse ce
  que la terre supporte (`soil_grip`). Un premier essai en frottement visqueux (la terre prend
  une part fixe du mouvement) n'ancrait rien : la croissance est si lente que les cellules
  finissent par se pousser comme à l'air libre (graine déplacée de 1,57 contre 1,67).
- Dureté : une cellule dont les parois pressent sur la terre plutôt que sur d'autres cellules
  grandit plus lentement (`soil_hardness`). « Presser sur la terre » est estimé par le
  déséquilibre de ses voisines : entourée de tous côtés, elle n'est pas exposée ; seule, ou à la
  pointe d'une colonne, elle l'est.
- Lumière : pleine dans l'air, divisée par e à chaque `light_depth` de profondeur.

Les réglages par défaut (prise 0,02, dureté 0,3, lumière 1) sont choisis par Claude, sans chiffre
du réel.

Mesuré (juge, 1 024 cellules, trois graines) : la terre tient la graine (déplacée de 1,48 contre
2,54 à l'air libre), la dureté raccourcit la racine (22 contre 26,5 de profondeur), et **rien ne
sort de terre** : sur dix-huit essais (six sols, trois graines), au plus deux cellules dépassent
la surface. La leçon d'avant est donc réfutée. La racine ne grandit qu'à sa pointe, là où
l'auxine s'accumule ; le haut de la graine, vidé de son auxine, ne grandit pas, donc rien ne
pousse contre l'appui. Ce qui manque n'est pas un appui mais un second foyer de croissance en haut.
Un test l'enregistre (`nothing_comes_out_of_the_soil_yet`) et doit casser le jour où une tige
émergera.

Vérification : sur la carte, la dureté fait entrer une racine carrée et une division dans la
croissance ; la carte les arrondit moins finement, et une division y bascule d'un pas au pas 34
(identique sur 60 pas sans sol). La comparaison exacte tient 30 pas sur la carte, 60 sur WARP.

Lumière et auxine (2026-11-25, piste choisie par Mickaël). Une capacité de cellule : elle
fabrique de l'auxine partout, et davantage à la lumière (`light_production`, par unité de volume
en pleine lumière ; la lumière nourrit les sucres dont l'auxine est faite). Allumée dans le
génome par défaut à 0,03 (décision de Mickaël, 2026-11-25) : la plante de référence sort de terre.
Les tests du mécanisme de la racine tournent sans sol ni lumière (`on_their_own`).

Mesuré, juge (1 024 cellules, trois graines, génome à cellules rondes) :

| Gravité sur les pompes | Lumière → auxine | Cellules sorties de terre | Hauteur au-dessus du sol |
|---|---|---|---|
| 0,3 | 0 à 0,01 | 0 | aucune |
| 0,3 | 0,03 | 1 à 11 | 0,1 à 2,3 |
| 0,3 | 0,1 | 52 à 121 | 5 à 7 |
| 0,1 | 0,03 à 0,3 | 333 à 492 (un tiers à la moitié) | 10 à 12 |

Sur la carte, plus grand (`docs/images/Lumière et auxine*.png`) :

- Cellules rondes, gravité 0,1, lumière 0,03 : une masse ronde qui perce la surface (2 200 de
  28 500 cellules à l'air, jusqu'à 12,5 au-dessus, aussi large que haute), comme un tubercule.
- Cellules allongées, gravité 0,3, lumière 0,03 : une seule colonne épaisse qui traverse la
  surface, la racine en bas (auxine à sa pointe), le haut à 24 au-dessus du sol (4 600 de 27 800
  cellules à l'air). La plante grandit par ses deux bouts pour la première fois, mais en un seul
  corps épais : pas de tige distincte, pas de pointe en haut (l'auxine du haut est renvoyée vers
  le bas par les pompes : 0,3 à la pointe du haut contre 35 à celle de la racine).

Ce qui manque encore pour une tige : une pointe en haut où l'auxine reste. Les pompes, qui
penchent toutes vers le bas, vident le haut. Dans une vraie tige, l'auxine descend aussi, mais la
pointe de la tige garde son propre foyer (méristème, WUS/CLV3) et ses cellules ne réagissent pas
à l'auxine comme celles de la racine.

Vérification : la règle est comparée cellule par cellule sur le tissu de test, qui chevauche la
surface. La germination exacte sur WARP reste sans lumière : l'exponentielle de WARP diffère de
celle du processeur de quelques millionièmes, assez pour faire basculer une division au pas 1 300.

Observé sans le corriger : deux essais à grande taille sur la carte se sont arrêtés (attente du
GPU dépassée, puis soumission refusée). Cause probable : la grille qui grossit avec l'étirement
des cellules (limite déjà notée). À traiter avant de viser le million de cellules.

Mûrir (2026-11-25, capacité validée par Mickaël, signal choisi par lui : l'auxine seule, « jamais
de béquille »). Chaque cellule porte une jeunesse de 0 à 1, héritée à la division. À chaque pas
elle se rapproche lentement (`youth_rate`) de ce que demande l'auxine qu'elle contient (la moitié
pour `youth_half`) : elle mûrit là où l'auxine manque, et redevient jeune si l'auxine revient
(racines latérales, épaississement plus tard). Seule une cellule jeune (au-dessus de 0,5) se
divise ; une cellule mûre grandit et s'allonge encore. Éteinte dans le génome par défaut
(`youth_rate = 0`), en attendant la décision de Mickaël.

Mesuré sur la carte (génome à cellules allongées, lumière, sol, 20 000 à 30 000 cellules ;
`docs/images/Cellules qui mûrissent.png`) :

| Vitesse | Seuil | Profondeur de la racine | Cellules à l'air | Hauteur à l'air |
|---|---|---|---|---|
| sans maturation | | 50 | 4 600 | 24 |
| 0,001 à 0,01 | 0,3 | 48 à 50 | 3 300 à 4 000 | 22 à 23 |
| 0,001 à 0,01 | 1 | 67 à 81 | 280 à 560 | 15 à 22 |
| 0,001 à 0,01 | 3 | 112 à 137 | 15 à 30 | 4 à 6 |

- Le reste se fige : les divisions se concentrent à la pointe, où l'auxine s'accumule ; le tissu
  ancien près de la graine ne s'épaissit plus (rayon 11 à 16 contre 24 à la pointe), et la
  racine devient deux à trois fois plus longue pour le même nombre de cellules. Un test sur la
  carte l'enregistre (`maturing_cells_make_the_root_grow_from_its_tip` : 109 contre 44 à 16 000
  cellules).
- Le haut mûrit et la plante ne sort plus de terre, comme annoncé : les pompes le vident de son
  auxine. Cause manquante désignée : une pointe de tige tenue par son propre système (dans le
  réel, WUS/CLV3 et la cytokinine).
- La racine n'est pas plus fine : sa pointe jeune est aussi large qu'elle, parce que le maximum
  d'auxine couvre toute la pointe. Dans une vraie racine il est étroit, grâce au retour de
  l'auxine vers le haut par l'épiderme puis vers le centre (« fontaine inversée »). Deuxième cause
  manquante désignée.

Cytokinine de lumière (2026-11-25, choix laissé à Claude par Mickaël : « fais ce qui te semble le
plus pertinent »). Correction d'abord : Claude avait dit que la lumière ne garde pas la pointe de
tige jeune dans le réel. C'est inexact : dans le noir la pointe de tige reste endormie, et la
lumière la réveille, mais par la cytokinine (Pfeiffer et al. 2016, eLife, vérifié). D'où deux
ajouts, sur la cytokinine déjà présente :

- une cellule éclairée fabrique de la cytokinine (`ck_light`, par unité de volume en pleine
  lumière) ;
- la cytokinine garde une cellule jeune là où l'auxine est rare (même réponse que pour la
  croissance : `ck_half`, `ck_block`). Les deux hormones échangent donc leurs rôles entre la
  pointe de racine (auxine) et la pointe de tige (cytokinine), comme dans le réel.

Éteinte dans le génome par défaut (`ck_light = 0`).

Mesuré sur la carte (génome à cellules allongées, maturation 0,01 / 3, 25 000 à 30 000 cellules ;
`docs/images/Cytokinine de lumière, un cal.png`) : c'est tout ou rien.

| Cytokinine de lumière | Auxine qui la bloque | Cellules à l'air | Forme |
|---|---|---|---|
| 0,001 à 0,03 | 0,1 | 3 à 45 | le haut mûrit, rien ne sort |
| 0,001 à 0,003 | 1 | 3 à 122 | idem |
| 0,01 | 0,3 | 46 | idem |
| 0,01 | 1 | 18 500 sur 30 000 | une boule (hauteur 31, aussi large que haute) |

Quand elle est assez forte, toutes les cellules éclairées restent jeunes et le haut jaillit en
boule : un cal, exactement ce que donnent les cultures de Skoog et Miller avec beaucoup de
cytokinine. Ce n'est pas une tige : rien ne réserve la jeunesse à une pointe. Cause manquante
désignée : dans une vraie pointe de tige, les cellules souches retiennent leurs voisines (elles
émettent CLV3, qui éteint WUS autour) ; le foyer reste petit et le reste mûrit en tige. Autre
écart noté : toutes les cellules à l'air reçoivent la pleine lumière, sans ombre des cellules
du dessus. Un test enregistre la boule
(`light_made_cytokinin_wakes_the_top_into_a_ball_not_a_shoot`) et doit casser le jour où une
pointe se tiendra.

Sonde : un essai qui grossit très vite dépasse la capacité entre deux relevés et la carte
abandonne (« soumission refusée ») ; des relevés plus rapprochés l'évitent. La cause exacte dans
le moteur reste à diagnostiquer.

Ombre : une tige sort de terre (2026-11-25). Avant d'ajouter un signal de retenue, l'écart
d'environnement noté plus haut a été corrigé : une cellule placée au-dessus d'une autre lui fait
de l'ombre (`shade` : la lumière est divisée par e pour chaque unité de couverture ; la couverture
additionne, sur les voisines directes, à quel point chacune est à l'aplomb). C'est de
l'environnement, pas une capacité de cellule.

Mesuré sur la carte (cellules allongées, maturation 0,01 / 3, auxine bloquant la cytokinine à 1 ;
`docs/images/Une tige sort de terre.png`) :

| Cytokinine de lumière | Ombre | Cellules à l'air | Hauteur | Écart au centre |
|---|---|---|---|---|
| 0,01 | 0,3 à 3 | 2 à 26 | rien ne sort | |
| 0,03 | 1 | 20 100 sur 28 200 | 48 | 14 (masse) |
| 0,03 | 3 | 2 700 sur 26 100 | 61 | 8 (tige) |

Avec une ombre forte, seules les cellules du dessus restent éclairées, fabriquent de la
cytokinine et restent jeunes ; celles qu'elles laissent derrière elles, à l'ombre, mûrissent. Le
sommet jeune grimpe sur ce qu'il a déposé : un col étroit sort de terre, puis une couronne qui se
divise en lobes, pendant que la racine descend (85 à 106 de profondeur). Trois graines sur trois
(à 16 000 cellules : 375 à 1 800 cellules à l'air, 24 à 52 de haut pour 6 à 10 d'écart au centre ;
test `shaded_cells_mature_and_a_stem_rises_from_the_lit_top`). Première plante avec une tige et
une racine, sans qu'aucune règle ne dise « monte ».

Le signal de retenue (WUS/CLV3) n'a donc pas été nécessaire pour obtenir une tige : l'ombre joue
le rôle de repère spatial. Il reste la bonne piste pour la suite : la couronne s'élargit et se
lobe sans contrôle, et rien ne fixe encore la taille du foyer. Non mesuré : les lobes
(ramifications ? ébauches de feuilles ?), et la tenue à plus grande taille.

Plante de référence (2026-11-25, décision de Mickaël) : maturation (0,01 / 3), cytokinine de
lumière (0,03, bloquée par l'auxine à 1) et ombre (3) sont allumées dans le génome par défaut.
Constat en les allumant : avec l'ancien génome par défaut (peu exigeant en auxine, transport
lent), rien ne sortait de terre ; le génome par défaut reprend donc aussi les deux réglages avec
lesquels tout a été mesuré depuis la germination (`half_sat = 10`, `polar = 0.1`, choix de Claude).
Mesuré sur la carte, cellules rondes, 20 000 cellules : tige de 42 de haut pour 6 d'écart au
centre (2 200 cellules à l'air), racine de 92. L'allongement des cellules reste éteint par défaut.
Les tests de la racine tournent sur le génome d'avant la maturation (`frugal`).

Visionneuse (2026-11-25, demandée par Mickaël : « visualiser un maximum de choses »). Une page HTML
autonome montre la plante cellule par cellule, colorée au choix par l'auxine, la cytokinine, la
jeunesse, la lumière reçue (donc l'ombre), le sens et la force des pompes, la taille ; en entier
ou en coupe, relevé par relevé, avec la lecture d'une cellule au survol. Deux commandes :

    cargo run --release -p geno_gpu --example germinate -- sorties/releves 21000 14 max=20000
    python tools/viewer.py sorties/releves "sorties/Plante de référence.html"

Le dossier `sorties/` n'est pas versionné. C'est une sonde, pas le rendu 3D final (toujours
ouvert) : dessin en 2D, ronds à la place des ellipsoïdes, quelques dizaines de milliers de cellules.
La lumière reçue de chaque cellule est recalculée à l'export (`checks::light_on`, toutes les paires).

Vue 3D, première version (2026-11-25, `geno_view`). La plante pousse en direct dans une fenêtre :
chaque cellule est dessinée comme son vrai ellipsoïde (un carré face à la caméra par cellule, le
calcul du rayon dans le fragment : forme exacte, sans maillage), directement depuis les tampons
de la simulation, sans copie. Le sol est une nappe translucide à la surface ; les cellules sous
terre sont teintées.

- Lancer : `cargo run --release -p geno_view` (pour un essai par l'IA : le binaire construit,
  par `ecran-de-test.ps1`).
- Touches : espace met en pause ; 1 à 8 choisissent la couleur (lumière reçue, jeunesse, auxine,
  cytokinine, sens des pompes, force des pompes) ; C coupe la plante en deux pour voir dedans ;
  flèches haut et bas changent la vitesse ; F recadre et suit la plante.
- Souris : glisser à gauche tourne, glisser à droite monte et descend, la molette zoome.
- Le titre de la fenêtre donne la couleur affichée, le nombre de cellules et le pas.
- Sans fenêtre : `geno_view --capture image.png [steps=21000] [mode=1..8] [cut] [paramètre=valeur]`
  (`docs/images/Vue 3D, jeunesse.png`).

Pour l'affichage, la lumière reçue par chaque cellule est gardée dans la quatrième case de la
polarité des pompes (aucune règle ne la lit). `GpuSim` peut se construire sur un adaptateur
choisi par l'appelant et prête ses tampons (`on_adapter`, `drawn`).

Vérifié : hors écran sur WARP, une cellule connue apparaît à la bonne taille (au pixel près), de
la bonne forme une fois allongée, de la bonne couleur, et disparaît en coupe. Essai en fenêtre
sur l'écran de droite : 968 puis 1 834 cellules en quatre secondes, environ 480 pas par seconde.
Non vérifié par l'IA : l'image de la fenêtre elle-même (pas de capture de l'écran) et le confort
de la caméra.

Limites de cette version : pas de texte ni de légende dans la fenêtre (tout est dans le titre),
échelles d'auxine et de cytokinine fixes, pas d'ombres portées ni de niveaux de détail, capacité
de 131 072 cellules, une lecture des cellules par seconde pour le compte et le cadrage.

La carte qui abandonne : diagnostic (2026-11-25). Ce n'était ni la capacité ni la grille (un tissu
entièrement hors de la grille reste exact, test `matches_the_judge_cell_by_cell_far_out_of_the_grid`).
La chaîne, mesurée relevé par relevé sur la plante de référence puis reproduite à volonté sur une
boule de cytokinine :

1. des cellules s'entassent : 40 à 55 voisines au lieu d'une douzaine, enfoncées de plus d'un
   rayon les unes dans les autres ;
2. la diffusion ferme à chaque pas une part fixe de l'écart avec chaque voisine (`ck_diffusion`
   0,05) : au-delà de vingt voisines elle dépasse l'équilibre, au-delà de quarante elle diverge.
   La cytokinine devient négative (−14, puis −2 130, puis −3·10⁸ en 300 pas) ;
3. toutes les cellules touchées deviennent « jeunes » et se divisent en chaîne (41 500 cellules
   devenues 106 600 en 1 500 pas), jusqu'à des nombres infinis ; toutes les cellules tombent alors
   dans la même case de la grille et la carte décroche (« soumission refusée »).

Deux entassements, deux causes :

- Sous terre, le sol retenait toutes les cellules, y compris celles de l'intérieur de la racine,
  qui ne touchent aucune terre. Ne pouvant faire de place à leur croissance, elles s'empilaient
  (recouvrement de 1,1 rayon sur l'axe de la racine, 0,46 au bord, 0,16 à l'air). Corrigé : le sol
  ne retient une cellule qu'à la mesure de ce qu'elle touche de terre (la même estimation que pour
  la dureté). Remesuré : douze voisines partout dans la racine, 18 au plus sur la plante entière,
  et la plante sort toujours de terre. La racine s'élargit au lieu de s'empiler.
- Dans un tissu où tout se divise (la boule), une cellule écrasée par ses voisines continuait de
  grossir. Cause manquante désignée et validée par Mickaël : la pression des voisines freine la
  croissance (lot suivant).

Effets de la correction du sol sur les mesures d'avant : le seuil de la boule de cytokinine sans
ombre passe de 0,01 à 0,03 ; la dureté du sol par défaut (0,3) ne ralentit presque plus la racine
(24,2 de profondeur contre 24,5 en sol meuble, au lieu de 22,2 contre 26,5 : une partie de l'effet
venait de l'entassement) ; il faut une dureté de 3 pour 20,8. Un contrôle garde la plante de
référence (`the_reference_plant_does_not_pile_up` : aucune hormone négative, pas plus de voisines
que la diffusion n'en supporte ; mesure `checks::packing`).

La sonde `germinate` accepte `capacity=` et `first=` (premier relevé écrit), qui ont servi à
cerner le pas où tout bascule.

La couronne, mesurée (2026-11-25, trois graines, relevés tous les 500 pas, foyers de cellules
jeunes suivis d'un relevé à l'autre ; mesure `checks::foci`). Trois sortes de foyers à l'air :

- Une pointe : un foyer reste tout en haut (à moins de 2 du sommet) et monte régulièrement
  (1,3 à 1,4 par 500 pas). Elle vit de cytokinine (1,17 contre 0,17 d'auxine), en pleine lumière.
  Mais rien ne fixe sa taille : elle enfle et se vide, de moins de 10 à plus de 130 cellules.
- Des ramifications, pas des ébauches de feuilles : des morceaux de la pointe restent sur les
  épaules éclairées de la couronne, hors de l'axe (à 10 à 20 de l'axe pour une tige de 7 à 13 de
  rayon), y vivent 2 500 à 6 500 pas et montent à leur tour, plus lentement (0,7 à 1,1 par 500
  pas). Mêmes hormones que la pointe, jamais aplatis, jamais arrêtés, sans espacement régulier :
  ce sont des pointes secondaires. De 2 à 14 en même temps selon la graine.
- Inattendu : des foyers tenus par l'auxine (3 à 3,8, à l'ombre) au bas de la tige, qui
  descendent vers le sol (0,7 à 0,9 par 500 pas) en s'écartant de l'axe. L'auxine faite en haut
  s'accumule au collet : cela ressemble à des départs de racines adventives. Non étudié.

Rien ne distingue donc une feuille dans ce modèle : pas de tissu qui s'aplatit ni qui s'arrête.
Un test enregistre la taille qui erre (`the_tip_of_the_stem_has_no_steady_size_yet`) et doit
casser le jour où la pointe tiendra sa taille.

La pression freine la croissance (2026-11-25, capacité validée par Mickaël). Une cellule sent à
quel point ses voisines s'enfoncent en elle (la somme des recouvrements) et grandit d'autant
moins ; elle cesse de grandir quand cette somme atteint `1 / squeeze_hold` de son rayon. C'est la
loi de Lockhart (1965) réduite à sa conséquence : une cellule pousse sa paroi avec sa pression
interne, et ce qui appuie en face la retient. Allumée dans le génome par défaut à 1 (choix de
Claude, sans chiffre du réel).

Mesuré sur la carte :

| Frein de pression | Plante de référence à 24 000 pas | Boule (sans ombre) à 9 000 pas |
|---|---|---|
| 0 | 16 400 cellules, 1 600 à l'air | diverge, la carte décroche |
| 0,5 | 18 300 cellules, 3 100 à l'air | 101 600 cellules, 22 voisines au plus |
| 1 (défaut) | 11 300 cellules, 3 500 à l'air | 31 600 cellules, 18 voisines au plus |
| 2 | 5 500 cellules, 3 400 à l'air | 4 500 cellules, 17 voisines au plus |
| 4 | la graine reste à 4 cellules | |

- La boule ne s'emballe plus : elle grossit au rythme où sa surface peut reculer. Le contrôle
  d'entassement couvre aussi ce cas.
- La résistance du sol émerge : le sol retient les cellules du bord de la racine, qui compriment
  celles de l'intérieur, qui grandissent moins. La racine ralentit et la tige en profite. Plante
  de référence à 36 000 pas, trois graines : 24 000 à 31 000 cellules, tige de 76 à 78 de haut
  (7 600 à 12 000 cellules à l'air), racine de 87 de profondeur. Avant, au même effectif : tige
  de 65, racine de 164. Le réglage `soil_hardness` fait maintenant en partie double emploi.
- Trop fort (4), deux cellules filles nées l'une dans l'autre ne se séparent plus dans le sol et
  la graine ne démarre pas.
- La pointe de tige erre toujours (de 3 à 147 cellules) et laisse toujours des ramifications
  (4 à 22 foyers à l'air) : le frein de pression n'y change rien.

Vérification : la règle est comparée cellule par cellule sur les tissus de test. Sur 600 pas,
l'écart statistique entre la carte et le juge passe de moins de 1 % à 2,4 % en effectif (une
cellule un rien plus pressée se divise plus tard, donc les deux tissus se séparent plus tôt) ; la
tolérance du test long est portée à 3 %. Les tests de la racine et des cellules allongées
tournent sans frein de pression (génomes d'avant).

Recherche documentaire (2026-11-25, demandée par Mickaël) : `docs/Modèles scientifiques de
plantes.md`. Elle nuance la suite : dans tous les modèles publiés, la taille de la pointe de tige
tient à un repère de surface et à un relais qui monte (WUS), pas à un signal de retenue seul ; et
le maximum étroit de la racine y est obtenu en imposant le sens des pompes par tissu.

Le signal de retenue seul, mesuré (2026-11-25). Troisième substance du modèle (après l'auxine et
la cytokinine) : une cellule assez jeune pour se diviser l'émet, elle diffuse et s'efface, et elle
atténue ce que la cytokinine réclame pour garder une cellule jeune (la moitié CLV3 de la boucle
WUS / CLV3, sans relais). Elle n'agit pas sur la jeunesse tenue par l'auxine (la racine). Règle
dans le juge, recopiée sur la carte et comparée cellule par cellule ; éteinte dans le génome par
défaut à ce stade (`restraint_made` 0), parce que seule elle ne donne pas le résultat.

Soixante réglages (force et portée), puis trois graines sur les meilleurs :

| Signal | Ce que fait la pointe |
|---|---|
| Faible | rien ne change : le plus gros foyer erre toujours de 3 à plus de 100 cellules |
| Meilleur réglage trouvé (`restraint_made` 0,05, `restraint_decay` 0,1) | 9 à 35 cellules sur une graine, 4 à 64 et 6 à 140 sur les deux autres |
| Trois fois plus fort | aucune tige ne sort (une cellule à l'air après 40 000 pas) |

La fenêtre entre « sans effet » et « plus de tige » tient dans un facteur trois, et elle ne tient
pas d'une graine à l'autre. Le signal seul ne fixe donc pas la taille de la pointe.

Pourquoi, et ce n'est pas le cas prévu (« le centre du foyer mûrit avant ses bords ») : il n'y a
ni centre ni bord. Mesuré cellule par cellule, toute la tige a une jeunesse entre 0,3 et 0,6, et
92 à 99 % des cellules éclairées sont entre 0,4 et 0,6, collées au seuil de division (0,5). La
cytokinine est à peu près aussi abondante sur tout le sommet éclairé (1,2 à 1,5 sur huit rayons
autour du foyer) et ce qu'elle réclame vaut presque exactement le seuil. Un « foyer » est donc
l'ensemble des cellules qui se trouvent un cheveu au-dessus de 0,5 : il enfle et se vide au gré de
variations de 0,02. Le signal de retenue, mesuré, vaut partout pareil (0,070 à 0,073 du centre au
bord) : il déplace tout le monde ensemble par rapport au seuil, d'où le tout ou rien.

Première forme essayée et écartée : émission proportionnelle à la jeunesse. Comme toutes les
jeunesses valent environ 0,5, les cellules mûres émettaient autant que les jeunes.

Cause manquante désignée par la mesure : rien, dans la tige, ne sépare nettement une cellule
jeune d'une cellule mûre, ni ne désigne un endroit. C'est ce que la recherche attribue au relais
de type WUS et au repère de surface (section 2 et 5 de `docs/Modèles scientifiques de plantes.md`).

Outils : la sonde `report` (décrite plus bas) a servi à ces mesures. Les sondes acceptent tous
les paramètres du génome par leur nom (`Params::set`).

Le relais et le repère de surface (2026-11-25, cause validée par Mickaël sur proposition). La
boucle WUS / CLV3 entière, écrite en causes locales, quatre substances en tout :

- La cytokinine ne garde plus une cellule jeune par elle-même. Là où elle appelle (cytokinine
  abondante, auxine rare), la cellule fabrique un relais, qui passe de cellule en cellule, et
  c'est le relais qui tient les cellules jeunes (WUS). La racine n'est pas touchée : l'auxine y
  tient la jeunesse directement.
- Le signal de retenue est émis par les cellules de la peau qui reçoivent du relais, et il
  atténue l'appel de la cytokinine, donc la fabrication du relais (CLV3). Une cellule est « de la
  peau » dans la mesure où ses voisines sont toutes du même côté : la même estimation locale que
  pour ce qu'elle touche de terre. Aucune lumière n'entre dans ce repère.
- La cytokinine s'efface dix fois plus vite (`ck_decay` 0,01, `ck_light` 0,3) : elle ne porte
  plus qu'à quelques cellules de la peau éclairée qui la fabrique, et dit donc à une cellule à
  quelle profondeur elle se trouve.

Mesuré, dans l'ordre :

| Génome | Pointe de tige |
|---|---|
| Relais seul, cytokinine d'avant (longue portée) | une boule : 12 900 cellules jeunes d'un bloc |
| Relais seul, cytokinine courte | une pointe nette sur l'axe, qui enfle sans fin (1 400 cellules et ça continue) |
| Avec signal de retenue 0,015 | 500 à 700 cellules |
| Avec signal de retenue 0,03 (génome de référence) | 200 à 450 cellules |
| Avec signal de retenue 0,06 | 40 à 130 cellules |
| Dix fois le réglage de référence | aucune tige |

Le signal règle donc la taille de façon graduée, et il la tient : sur trois graines et une
plante de 65 000 cellules, 180 à 450 cellules pendant 30 000 pas (avant : de 3 à 147, sans
plancher ni plafond). La plante de référence à 16 000 cellules, trois graines : 3 900 à 4 200
cellules à l'air, tige de 74 à 76 de haut pour 10 à 12 de large, racine de 81 à 82 de profondeur.
Elle monte bien plus vite qu'avant (76 de haut à 16 000 cellules, contre 24 à 52).

Ce qui reste vrai et n'est pas réglé :

- La pointe varie encore d'un facteur 2 à 3 : elle se dédouble quand un morceau part sur le côté,
  puis regrossit.
- Les ramifications se multiplient : 1 foyer au départ, 39 à 54 à 35 000 cellules à l'air. Rien
  ne retient un bourgeon (dans le réel : l'auxine de la tige et le sucre, section 2 de la
  recherche). La pointe principale s'écarte de l'axe (8 à 12).
- À la peau éclairée, le relais se cale juste au seuil (0,9 pour un seuil de 1) : 86 à 96 % des
  cellules éclairées restent à moins de 0,1 du seuil de division. Le contraste jeune / mûr existe
  maintenant entre la pointe et le reste de la tige, pas à l'intérieur de la peau.
- Sans ombre, c'est toujours une boule.

Tests : `the_tip_of_the_stem_holds_its_size` (remplace le constat d'errance, cassé comme prévu)
et `without_the_restraint_signal_the_tip_keeps_swelling`. Le constat « le signal seul ne tient
pas la pointe » n'a plus de test : sans relais, ce génome n'existe plus (la cytokinine n'agit que
par lui) ; il reste écrit ci-dessus.

Relevés et visionneuse : `germinate` écrit deux colonnes de plus (signal de retenue, relais,
après la cytokinine) et la visionneuse les affiche ; les relevés d'avant ne se relisent plus. La
vue 3D les montre aussi (touches 7 et 8).

Outils pour juger seul (2026-11-25, demande de Mickaël : que l'IA se donne les moyens de voir
elle-même les problèmes, le but étant une plante visuellement convaincante) :

- La planche : `cargo run --release -p geno_view -- --sheet sorties/planches/nom.png [steps=24000]
  [shots=3] [paramètre=valeur]`. Une image, sans fenêtre : la plante en coupe, une ligne par âge,
  une colonne par couleur (lumière reçue, jeunesse, auxine, cytokinine, relais, signal de
  retenue). Les échelles des substances se règlent seules (98 cellules sur 100 dessous) et sont
  imprimées. Les images ne sont pas compressées (17 Mo) : elles vont dans `sorties/`, hors du
  dépôt.
- Le bilan : `cargo run --release -p geno_gpu --example report -- <pas> <relevés> [capacity=..]
  [profile=1] [paramètre=valeur]`. Une ligne par relevé : hauteur et largeur de la tige,
  profondeur et largeur de la racine, nombre de foyers jeunes à l'air, taille du plus gros et du
  plus haut, écart du plus haut à l'axe, jeunesse moyenne des cellules jeunes et des autres ;
  puis le profil du centre au bord du foyer le plus haut (mesure `checks::profile`), la coupe
  sous le sommet et trois coupes en travers de la tige (`checks::under`, `checks::across`).
- La vue 3D et `--capture` acceptent aussi les paramètres du génome par leur nom.

Ce que la première planche a montré, et que les chiffres seuls cachaient (plante de référence,
8 000, 16 000 et 24 000 pas) :

- Pas de pointe visible. Le relais est fort dans toute la partie aérienne (0,8 partout), le
  signal de retenue aussi, et la jeunesse y est presque uniforme : 0,53 en moyenne pour les
  cellules « jeunes », 0,36 à 0,48 pour les autres. La boucle cale toute la tige au seuil, pas
  seulement la peau. La taille du foyer est tenue (le test le mesure), mais ce foyer est
  l'ensemble des cellules un peu au-dessus de 0,5, pas un méristème distinct d'une tige mûre. Le
  résultat « la pointe tient sa taille » est donc vrai en nombre et pas encore à l'œil.
- Une silhouette de tubercule : la tige fait 86 de haut pour 23 de large à 18 000 cellules, la
  racine 55 de profondeur pour 20 de large. Ni tige ni racine fines.
- Presque aucune cellule éclairée : quelques cellules au sommet seulement reçoivent la lumière,
  tout le reste est dans l'ombre des voisines.
- La racine n'est jeune que sur sa calotte du bas, sur toute sa largeur, et elle penche.

Mesure préparatoire pour la racine (une graine, 7 400 cellules, avant le relais) : le maximum
d'auxine est déjà étroit en hauteur (10 cellules sur 2 de haut tiennent plus de la moitié du pic,
139 contre 0,3 dix rayons plus haut), mais la racine est très épaisse (rayon d'environ 12), toute
sa largeur est jeune sur 8 de haut, et près de la pointe l'auxine est plus forte au bord que dans
l'axe (23 contre 8), l'inverse plus haut (0,1 contre 0,5).

Contraste de la pointe, mesuré (2026-11-26, priorité choisie par Mickaël). Mesure : jeunesse
moyenne des cellules jeunes à l'air contre celle des autres (colonnes `youth_young` et
`youth_other` de la sonde `report`). Référence : 0,53 contre 0,36 à 0,48.

| Essai | Jeunes / autres | Pointe |
|---|---|---|
| Cytokinine qui ne répond qu'à la pleine lumière (`ck_half` 3 ou 10) | 0,54 à 0,56 / 0,28 à 0,30 | 124 à 1 400 cellules selon le signal |
| Relais plus fort (`relay_made` 0,05 ou 0,1) | 0,54 à 0,64 / 0,29 à 0,36 | 850 à 7 200 cellules, tige de 27 de large |
| Signal de retenue plus fort (jusqu'à dix fois) | inchangé | plus petite, puis aucune tige |
| Réponse au relais en interrupteur (puissance 4, essai non gardé) | 0,66 à 0,88 / 0,17 à 0,27 | tout ou rien : une boule de 3 500 à 10 000 cellules, ou aucune tige |

Aucun réglage ne donne le contraste : les cellules jeunes restent juste au-dessus du seuil. Une
réponse en interrupteur le donne, mais elle détruit le réglage gradué de la taille (boule ou
rien, sur neuf réglages), donc elle n'est pas la cause manquante à elle seule et n'a pas été
ajoutée. Ce que ces mesures désignent : le champ de relais est un dégradé doux sur toute la
largeur de la tige, sans rien qui sépare en profondeur l'endroit où il est fabriqué (sous la
peau, dans le réel) de l'endroit où il agit (au sommet).

Profils sous le sommet et en travers de la tige, mesurés (2026-11-26). La sonde `report` imprime
maintenant, couche par couche (une couche = un rayon de division), la coupe sous la cellule la
plus haute et trois coupes en travers de la tige (à 4 et 12 sous le sommet, et à mi-hauteur), de
l'axe vers le bord : jeunesse, lumière reçue, appartenance à la peau, auxine, cytokinine, signal
de retenue, relais, et la fabrication du relais (part de son maximum), pour séparer l'endroit où
il est fait de celui où il se trouve. Mesures `checks::under` et `checks::across`.

Plante de référence à 16 500 cellules (tige de 86 de haut), sous le sommet :

| Profondeur | Cytokinine | Fabrication du relais | Relais | Signal de retenue | Jeunesse |
|---|---|---|---|---|---|
| 0 (la peau) | 3,1 | 0,61 | 0,90 | 0,135 | 0,51 |
| 4 | 1,4 | 0,47 | 0,83 | 0,080 | 0,50 |
| 10 | 0,6 | 0,29 | 0,65 | 0,052 | 0,47 |
| 23 | 0,25 | 0,15 | 0,38 | 0,035 | 0,38 |
| mi-hauteur (43) | 0,14 | 0,09 | 0,22 | 0,023 | 0,27 |

En travers, à 4 sous le sommet, la tige a déjà un rayon de 10 et tout y est plat de l'axe au
bord : relais 0,84 à 0,85, jeunesse 0,48 à 0,52.

Écart avec Gruel 2016 (relais absent de la peau, fabriqué dans une bande à profondeur fixe, dans
un petit domaine) :

- Le relais est fabriqué le plus fort dans la peau elle-même, et de moins en moins en descendant,
  sans bande ni bord : encore un quart du maximum à 23 couches de profondeur.
- Le relais n'a pas d'endroit à lui : il vaut partout à peu près ce qui est fabriqué sur place.
  Il ne monte de nulle part.
- Chaque maillon répond en courbe douce et écrase le contraste : la cytokinine tombe de 12 fois
  entre le sommet et 23 couches plus bas, la fabrication de 4 fois, le relais de 2,4 fois, la
  jeunesse de 1,3 fois.
- Le signal de retenue ne pèse presque rien : 0,135 au sommet pour un demi-effet à 1, soit 4 à
  12 % de fabrication en moins. Il règle la taille seulement parce que tout est au seuil.
- Il ne vient pas surtout de la peau : l'estimation « cellule de la peau » vaut 0,06 à 0,18 pour
  une cellule de l'intérieur (0,5 à 0,8 à la peau), et les cellules de l'intérieur sont bien plus
  nombreuses. Dans la coupe à 4 sous le sommet, elles émettent environ les deux tiers du signal.
  Et il porte plus loin que la cytokinine (il perd un facteur 2,7 en dix couches, elle 5,5), à
  l'inverse de Gruel où le répresseur est le signal court.
- Rien ne distingue le centre du flanc : pas de domaine au milieu du dôme.

Vérifié par les réglages seuls, sans règle nouvelle (quinze essais, bilan et planche) :

| Essai | Fabrication sous le sommet | Relais, jeunesse | Plante |
|---|---|---|---|
| Signal de retenue 3 à 30 fois plus fort, relais 1 à 10 fois | plate ou décroissante, jamais en bande | plats | aucune tige, ou une boule (35 de large pour 38 de haut) |
| Signal de retenue court et fort (efface dix fois plus vite, fabriqué 30 à 300 fois plus) | une bande apparaît : 0,10 à la peau, 0,18 à quatre couches, 0,10 à treize | toujours plats (relais 0,73 à 0,79 sur dix couches) | un dôme qui sort à peine (11 de haut), ou une boule |

Un signal court creuse donc bien le relais hors de la peau, comme chez Gruel, mais la bande fait
dix couches d'épaisseur sur toute la largeur de la tige, le relais s'y étale, et la plante reste
en tout ou rien. La bande ne suffit pas : il manque ce qui fait d'un champ doux un domaine à bords
nets et de taille propre.

Cause que l'écart désigne, proposée à Mickaël (pas ajoutée) : le relais entretient sa propre
fabrication. Dans le réel, WUS éteint les freins de la réponse à la cytokinine (les gènes ARR de
type A), donc une cellule qui tient du relais entend mieux la cytokinine et en fabrique davantage
(Leibfried 2005, Gordon 2009 ; section 2 de la recherche). Avec le signal de retenue qui porte
plus loin, c'est un couple activateur / inhibiteur : il donne une tache de taille propre au lieu
d'un dégradé (Fujita 2011). Notre boucle a l'inhibiteur, pas l'auto-entretien. La réponse en
interrupteur essayée plus haut durcissait la lecture d'un champ resté doux ; ici c'est le champ
lui-même qui se concentre. Défaut à corriger dans le même lot : le signal de retenue émis par
l'intérieur.

Décision de Mickaël (2026-11-26) : ajouter l'auto-entretien du relais, et corriger d'abord, seul,
le signal de retenue émis par l'intérieur.

La peau seule émet le signal de retenue (2026-11-26). « Être de la peau » a maintenant sa propre
estimation (`rules::skin`) : à quel point les directions des voisines s'accordent au-delà de ce
que donne le hasard (le cosinus moyen entre deux voisines). Elle vaut exactement zéro pour une
cellule entourée, même inégalement, 0,1 à 0,3 sur un flanc, 1 au bout d'une colonne. Ce que la
cellule touche de terre garde l'ancienne estimation (`rules::exposed`), choix de Claude : appliquée
au sol, la nouvelle ne retenait plus la graine (déplacée de 3,0 contre 2,5 à l'air libre), et un
dixième de prise à l'intérieur d'une racine ne fausse rien de mesuré.

Mesuré (une graine, 30 000 pas) : le signal règle toujours la taille de façon graduée.

| Signal de retenue | Pointe (cellules) | Tige à 30 000 pas |
|---|---|---|
| 0,03 (ancien réglage) | 500 à 1 000 | 116 de haut, 29 de large |
| 0,06 | 320 à 670 | 119 de haut, 24 de large |
| 0,1 (nouveau réglage de référence) | 140 à 310 | 98 de haut, 18 de large |
| 0,2 | aucune tige | |

Le contraste ne change pas (jeunes 0,52 à 0,53, autres 0,35), comme attendu : la correction ôte un
défaut, elle n'est pas la cause manquante. La tige est un peu plus fine (18 de large contre 20 à
24 au même âge).

Le relais entretient sa fabrication (2026-11-26, cause validée par Mickaël). Une cellule a des
freins sur sa réponse à la cytokinine, et le relais qu'elle tient les lève : sans relais, il lui
faut `1 + ck_brake` fois plus de cytokinine pour répondre à moitié ; la moitié des freins est
levée quand le relais atteint `relay_frees`. Réponse en courbe douce, sans interrupteur. Elle vaut
pour tout ce que la cytokinine fait à la cellule (relais et croissance). Règle dans le juge,
recopiée sur la carte, comparée cellule par cellule. Éteinte dans le génome par défaut
(`ck_brake` 0) en attendant la décision de Mickaël : elle donne la pointe visible, pas encore la
taille tenue.

Mesuré (une graine, freins à 10, relais fabriqué à 0,1 ; bilan et planche
`sorties/planches/Auto-entretien.png`) :

| | Sans auto-entretien | Avec (signal de retenue 0,45) |
|---|---|---|
| Jeunesse, 4 sous le sommet / à mi-hauteur | 0,52 / 0,31 | 0,63 / 0,19 |
| Relais, 4 sous le sommet / à mi-hauteur | 0,93 / 0,27 (3,5 fois) | 1,48 / 0,13 (11 fois) |
| Jeunes / autres cellules à l'air | 0,53 / 0,35 | 0,58 / 0,25 |
| Foyers jeunes à l'air | 4 à 11, des ramifications | 1 ou 2, le plus gros sur l'axe (écart 0,6) |

- La pointe se voit : à la planche, une calotte jeune au sommet et une tige mûre dessous, le
  relais rassemblé en haut, le signal de retenue en écorce sur la peau.
- Une seule pointe, sur l'axe : les ramifications ont disparu.
- La taille n'est plus tenue : la calotte occupe toute la largeur de la tige et grossit avec elle
  (460, 840, 1 430, 1 540 cellules de 15 000 à 30 000 pas). En travers, tout reste plat de l'axe
  au bord : rien ne distingue encore le centre du flanc.
- Le signal de retenue règle la taille sur une fenêtre étroite : 3 000 cellules à 0,3, 1 500 à
  0,45, 960 à 0,6, aucune tige à 0,8 (la jeune pousse, toute en peau, s'éteint avant de sortir).
  Un signal plus court (efface sept fois plus vite) ne change rien.
- La silhouette empire : la partie aérienne est un bulbe (23 de large pour 56 de haut), plus
  large que le collet.

Tests : `a_cell_that_holds_relay_hears_cytokinin_better` (juge) et
`a_relay_that_sustains_its_own_making_sets_a_young_tip_apart_from_the_stem` (carte).

Décision de Mickaël (2026-11-26) : l'auto-entretien reste éteint, la silhouette d'abord.

La silhouette, mesurée (2026-11-26). Le bilan imprime maintenant, à chaque relevé, la tige et la
racine en six tranches (largeur, part de cellules jeunes, rayon moyen des cellules, allongement,
orientation de leur grand axe), et la part de leurs voisines que les cellules mûres de la tige ont
gardée sur les 1 000 derniers pas (mesure `checks::kept_neighbours`).

Plante de référence, une graine :

- La pointe tient sa largeur : rayon 9 à 11 à quatre couches sous le sommet, de 16 000 à 36 000
  pas. Ce n'est donc pas la pointe qui épaissit la tige dans ce génome (avec l'auto-entretien, si :
  9 puis 14).
- Sous la pointe, la tige gonfle : rayon 14 à 16, une fois et demie la pointe. Les cellules qui
  mûrissent finissent de grossir en rond (rayon moyen 0,86 dans la tranche du sommet, 1,00
  dessous), ce qui élargit autant que cela allonge.
- Proportions : 118 de haut pour 32 de large à 36 000 pas, soit une quinzaine de cellules en
  travers. Ce nombre de cellules en travers n'est pas loin du réel (une jeune tige en compte une
  dizaine, chiffre de mémoire, non relu). Ce qui manque est la longueur.

Dans une vraie tige, la longueur vient des cellules qui s'allongent dans l'axe en quittant la
pointe (dix à cent fois leur largeur). Cette capacité existe ici (cellules allongées, éteinte par
défaut). Essayée sur la plante de référence, par réglages seuls :

| Cellules mûres | Allongement, orientation | Tige au même âge (≈ 5 000 cellules à l'air) |
|---|---|---|
| Rondes (référence) | 1,2 fois, sans orientation | 73 de haut, 17 à 20 de large |
| Allongées (plafond 2,25) | 2,8 fois plus longues que larges, verticales à 0,95 | 75 de haut, 22 de large |
| Très allongées (plafond 4) | 5,2 fois, verticales à 0,90 à 0,95 | 75 de haut, 28 de large |

Des cellules cinq fois plus longues que larges, bien verticales, ne font pas monter la tige : elles
l'élargissent. Le tissu se comporte comme un liquide. Mesure directe : les cellules mûres de la
tige perdent 5 à 22 % de leurs voisines en 1 000 pas (référence : 0,78, 0,89, 0,95 gardées selon
l'âge ; cellules allongées : 0,60, 0,87, 0,91). Dans une plante, c'est zéro : les cellules sont
collées par leurs parois et ne glissent jamais (section 4 de la recherche). Une adhérence plus
forte (réglage) ne tient les voisines qu'en étouffant la croissance : trois fois plus forte, la
plante fait 38 de haut avec 650 cellules à l'air et glisse encore (0,93) ; dix fois, rien ne sort
(4,7 de haut).

Cause que l'écart désigne, proposée à Mickaël (pas ajoutée) : les cellules ne sont pas collées.
Tant qu'elles glissent, la forme d'un organe ne peut pas venir de la croissance de ses cellules :
tout allongement s'écoule sur les côtés. C'est la cause numéro 5 de la recherche (« voisines
permanentes »), devenue la première. Elle touche le cœur du moteur (liste de voisines par cellule,
division, forces) : à décider avant d'écrire.

Les voisines permanentes, en prototype dans le juge (2026-11-26, décision de Mickaël : cette
cause, et d'abord dans le juge seul). `geno_sim/src/bonds.rs` : un lien naît à la division entre
les deux filles, n'est jamais refait, et quand une cellule liée se divise chaque lien va à la fille
qui touche la voisine (aux deux si les deux la touchent). Deux cellules liées se tirent dès qu'un
vide les sépare, sans limite de distance ; deux cellules qui se touchent sans lien ne collent
plus. Le moteur de la carte n'est pas touché (`Reference::glued`, éteint par défaut). Sonde :
`cargo run --release -p geno_sim --example glue -- <pas> <relevés> <force> [touch=] [paramètres]`,
une seconde par essai.

Verdict : tel quel, cela ne marche pas, et la mesure dit pourquoi.

| Partage des liens à la division | Liens par cellule | Liens entre cellules qui ne se touchent plus | Pression (la croissance s'arrête à 1) | Plante à 12 000 pas |
|---|---|---|---|---|
| Sans liens (référence) | 0 | | 0,37 | 2 048 cellules, tige 24 de haut |
| Fille qui touche (portée 1,2) | 4 | 2 à 6 % | 0,42 | pareille, glisse autant (0,79 à 0,96 gardées) |
| Fille proche (portée 1,5), liens forts | 9 à 10 | 26 % | 0,88 | 330 cellules, rien à l'air |
| Fille proche (portée 1,5), liens quatre fois plus faibles | 10 | 34 % | 0,61 | 430 cellules, tige 9 de haut |
| Par la coupe (la voisine que la nouvelle paroi traverse) | 10 à 12 | 35 à 50 % | 0,9 à 1,9 | 60 à 400 cellules, rien à l'air |

- Avec le seul contact, une cellule ne garde que 4 liens : des sphères ne remplissent pas la place
  de leur mère, un quart seulement des voisines touche les deux filles. Un tissu réel en a une
  douzaine. Quatre liens ne tiennent rien.
- Dès qu'on en donne une dizaine, le tissu s'écrase lui-même. Un tiers à la moitié des liens
  relient des cellules qui ne se touchent plus, une troisième s'étant glissée entre elles : ce ne
  sont plus des parois, ce sont des élastiques tendus à travers le tissu. Même avec peu de liens
  distendus (10 %, liens forts), la pression double.
- La direction est pourtant la bonne : dans le seul essai où le tissu lié pousse (cellules
  allongées, portée 1,5, liens faibles), la tige est plus fine à hauteur égale (16,5 de haut pour
  7,8 de large et 155 cellules à l'air, contre 17,8 pour 10,3 et 395 cellules sans liens).

Ce que l'écart désigne : un lien sans longueur propre n'est pas une paroi. Dans une plante, deux
voisines sont tenues à la distance où leur paroi commune a été posée, et cette paroi cède sous la
pression quand le tissu grandit (c'est par là que l'auxine fait pousser). Ici le lien veut toujours
ramener les deux cellules au contact : une dizaine de liens par cellule ne peuvent pas tous être
satisfaits à la fois dans un empilement de sphères, d'où une tension permanente que les
recouvrements compensent, et la croissance s'arrête. Il manque au lien sa longueur de repos, et à
cette longueur de céder. Pas ajouté : à décider avec Mickaël.

La paroi qui cède (2026-11-26, décision de Mickaël). Chaque lien retient maintenant l'écart où la
paroi a été posée (à la division, là où se tiennent les deux cellules), ne tire que si on l'étire
au-delà, et, étiré plus qu'il ne tient, s'allonge pour de bon (`bonds::Glue` : force, seuil
`hold`, vitesse `creep`). Une variante ne cède que dans le sens où les deux cellules poussent et
tient en travers (`across`, le rôle des fibres de cellulose). Les liens d'avant (sans longueur
propre) sont remplacés, leur verdict reste ci-dessus.

Génome à cellules allongées, 14 000 pas, plafond de 2 048 cellules, une dizaine de liens par
cellule (portée 1,5) :

| Parois | Cellules à l'air | Tige (haut, large) | Voisines gardées | Pression |
|---|---|---|---|---|
| Aucune (référence) | 1 050 | 38, 15 | 0,95 | 0,26 |
| Qui cèdent, souples | 1 320 à 1 360 | 35, 19 à 20 | 0,96 à 0,97 | 0,30 à 0,34 |
| Qui cèdent, raides | 1 250 à 1 400 | 27 à 33, 15 à 19 | 0,94 à 0,98 | 0,58 à 0,68 |
| Qui tiennent davantage (seuil 0,05), raides, cession rapide | 27 | 6, 6 | 1,00 | 0,82 |
| Qui ne cèdent que dans le sens de croissance, souples | 1 360 à 1 390 | 37 à 41, 18 à 19 | 0,96 à 0,98 | 0,30 |
| Qui ne cèdent que dans le sens de croissance, raides | 1 100 à 1 320 | 25 à 32, 14 à 19 | 0,92 à 0,96 | 0,61 à 0,66 |

- L'étouffement est levé : la plante atteint son plafond dans presque tous les réglages. C'était
  bien la longueur de repos qui manquait pour que le tissu lié puisse grandir.
- La silhouette ne change pas : la tige n'est pas plus fine, plutôt plus large à hauteur égale.
  Une paroi qui cède assez pour laisser grandir laisse aussi couler, et faire tenir le travers
  n'y change rien de mesurable.
- Les voisines gardées montent peu (0,96 à 0,98 contre 0,95) : la mesure ne voit que les
  contacts, et un tiers des liens relient toujours des cellules qui ne se touchent pas.

Limites de cet essai, à ne pas oublier : à 2 048 cellules la tige fait une dizaine de cellules de
large pour une vingtaine de haut, ce n'est pas encore une tige (sur la carte : 118 pour 32) ; et
les liens ne viennent que du lignage, avec des sphères qui ne remplissent pas la place de leur
mère. Trois formes de lien essayées (élastique, paroi qui cède, paroi qui cède dans un sens) :
aucune ne donne une tige plus fine. Ce prototype n'est donc pas à porter sur la carte en l'état.

Piège rencontré cinq fois ce jour, et d'abord mal attribué : les tests de
`geno_gpu/tests/emergence.rs` ne finissaient pas (plus d'une heure au lieu de 4 minutes), et
`check.ps1` finissait rouge sans dire pourquoi. La carte occupée par un autre programme (un
entraînement d'images, carte à 90 %) a été accusée ; le blocage est revenu carte libre. Vu de près :
sept tests finis, un seul fil tournant à vide sur un cœur, hors de la carte. La cause est dans le
test : plusieurs plantes poussaient en même temps dans le même programme, et la garde ne couvrait
que la création de l'accès à la carte. Corrigé : une plante à la fois (209 s à la suite contre
243 s côte à côte, donc sans coût). Vérification complète verte ensuite, tests d'émergence compris,
ce qui valide aussi le lot des parois. Une carte occupée ralentit toujours les mesures.

Prochaine étape, décidée par Mickaël (2026-11-26) : la pointe d'abord. Le prototype des parois
reste dans le juge, éteint. Reprendre la taille de la pointe avec l'auto-entretien (la calotte
aussi large que la tige, et qui grossit), puis la fontaine inversée de la racine. Ce travail se
mesure sur la carte : la relancer libre, et commencer par repasser `check.ps1` en entier.

### La pointe avec auto-entretien : mesurée, la boucle tient un niveau, pas une largeur (2026-11-26)

Sonde `report 24000 8 profile=1 ck_brake=10 relay_made=0.1 restraint_made=0.45` et planche
`geno_view --sheet` aux mêmes réglages. Constats chiffrés, aucun changement de code :

- La partie aérienne est un bulbe, pas une tige : largeur 10, 14, 17 puis 20 (pas 15000 à 24000),
  calotte jeune de 538, 722, 1152 puis 1418 cellules. La calotte, c'est tout ce qui se trouve à
  moins de dix couches du sommet, sur toute la largeur.
- En travers, tout est plat : relais 1,46 au centre contre 1,25 au bord, fabrication 0,17 partout.
  Rien ne distingue le flanc du centre.
- Le signal de retenue est une coquille : 0,9 sur la peau, 0,2 au centre, et le centre en reçoit de
  moins en moins à mesure que le bulbe s'élargit (0,34 puis 0,21). Sa portée est d'une à deux
  cellules.
- Portée allongée, essayée par les réglages seuls : `restraint_decay=0.003` donne soit le même
  gonflement (fabrication 0,05 : calotte 152 à 1768), soit plus de tige du tout (0,15). La portée
  n'est donc pas la cause. (`restraint_diffusion=0.5` fait tout exploser en boule, sans doute hors
  du domaine stable de la diffusion : essai sans valeur.)
- Avec les cellules qui s'allongent (génome étiré) : tige plus haute (66 pour 28 de large), calotte
  qui plafonne vers 1500 à 1600 cellules, mais la largeur monte encore (22, 25, 28).
- Les cellules du dôme pointent dans tous les sens (0,63, le hasard vaut 0,5), celles de la tige
  mûre sont dressées (0,95) ; l'auxine est basse et plate dans toute la partie aérienne (0,1 à 0,3).

Lecture : un relais qui s'entretient lui-même avance comme un front, jusqu'où la cytokinine
suffit, et la cytokinine vient de toute la peau éclairée, flancs compris. La boucle règle le
niveau du relais, pas l'étendue de la zone jeune. Dans le réel, la largeur du dôme n'est pas tenue
par cette boucle seule (Gruel 2016 : le domaine s'élargit avec le dôme) : les cellules du flanc
quittent la pointe, là où l'auxine s'accumule dans la peau. Ici, aucune auxine ne marque le flanc.
La cause à ajouter reste à choisir avec Mickaël.

Décision de Mickaël (2026-11-26) : la cause à ajouter est la sortie du flanc par l'auxine. Avant
toute règle, quatre essais par les réglages seuls (pas 18000, coupe à 4 sous le sommet) :

| Essai | Auxine au centre | Auxine sur la peau | Calotte | Largeur |
|---|---|---|---|---|
| Référence (auto-entretien) | 0,16 | 0,11 | 722 | 14 |
| Sans gravité des pompes (`gravisense=0`) | 1,4 | 1,4 | 4293 | 27 |
| Sans pompes (`polar=0`) | 1,7 | 1,9 | 3785 | 27 |
| Pompes vers le plus riche seulement (`pin_switch=1000 gravisense=0`) | 0,84 | 0,3 | 2085 | 19 |
| Dix fois plus d'auxine à la lumière (`light_production=0.3`) | 0,8 | 1,2 | 1504 | 21 |

Deux obstacles, mesurés :

1. La peau de la pointe ne garde pas son auxine. Elle la fabrique à la lumière, mais les pompes
   l'envoient vers l'intérieur puis vers le bas (une cellule de peau n'a de voisines que d'un côté,
   donc « vers les voisines les plus riches » pointe vers l'intérieur). Dans le réel, la peau
   retient l'auxine (transporteurs d'entrée AUX1/LAX de l'épiderme, Reinhardt 2003, Bainbridge 2008).
2. Quand l'auxine est là, elle fait l'inverse de ce qu'il faut : plus d'auxine donne une calotte
   plus grosse, parce que dans le modèle l'auxine garde jeune partout (la logique de la racine,
   `youth_half`). Dans le réel les rôles s'échangent : en pointe de tige l'auxine fait quitter
   l'état jeune (organes), et le relais (WUS) y émousse la réponse à l'auxine (Ma et al. 2019).
   Une cellule doit donc savoir si elle est de tige ou de racine, ce que rien ne porte aujourd'hui.

Aucune règle ajoutée : le second point touche à l'état d'une cellule, il se décide avec Mickaël.

Décision de Mickaël (2026-11-26) : la cellule lit l'auxine par le niveau (l'auxine ne garde jeune
qu'à la dose d'un bout de racine). Essayé d'abord sans code, par `youth_half=10` au lieu de 3
(0,09 d'appel à la dose d'une tige, 0,67 à celle d'un bout de racine) :

| Réglages (avec `youth_half=10`) | Résultat au pas 24000 |
|---|---|
| Génome par défaut | aucune tige, racine seule (183 cellules) |
| Auto-entretien, `restraint_made=0.45` | aucune tige (186 cellules) |
| Idem, `ck_brake=3` | aucune tige (296 cellules) |
| Idem, `restraint_made` de 0,2 à 0,4 | boule jeune de 4600 à 7700 cellules, carte pleine dès le pas 12000 |

Ce que cela révèle : aujourd'hui la pointe de tige naît et tient grâce à l'auxine qui garde jeune
(la logique de la racine), pas grâce à la boucle relais et retenue. Laissée seule, la boucle est en
tout ou rien : rien du tout à 0,45 de retenue, une boule qui envahit tout à 0,4. Elle ne règle
aucune taille. La fenêtre étroite déjà notée plus haut (0,3 à 0,6) était le même défaut, masqué par
l'auxine. Tant que la boucle n'a pas d'état stable intermédiaire, ni la sortie du flanc ni la
lecture par seuil ne peuvent être jugées : aucune règle n'a été ajoutée, le code est inchangé.

Piste suivante, à valider : étudier la boucle seule dans le juge, sur un bloc de cellules qui ne
pousse pas (peau éclairée d'un côté), et chercher pourquoi elle n'a pas d'état stable, en la
comparant à Gruel 2016, où le relais ne s'entretient pas lui-même et où le domaine est borné par
deux signaux de la peau de portées différentes.

### La boucle relais et retenue seule, sur un tissu figé (2026-11-26)

Sonde du juge `cargo run --release -p geno_sim --example relay_loop -- <pas> <rayon> <hauteur>
[dome=1] [start=…] [paramètre=valeur]` : une colonne de cellules tassées au hasard, en l'air,
éclairée d'en haut, qui ne pousse, ne bouge ni ne se divise, sans auxine. Dix secondes par essai,
sans carte. Elle redonne le profil de la plante (relais plat en travers, jeune sur une dizaine de
couches), donc ce profil vient de la boucle et non de la croissance. Réglages : `ck_brake=10
relay_made=0.1`, 3000 pas.

| Question | Mesure | Réponse |
|---|---|---|
| Deux états stables ? | départ à relais 0 ou 5 : résultat identique au chiffre près | non, un seul état, aucune mémoire |
| Effet de seuil ? | `restraint_made` 0, 0,45, 1,5, 5 : 650, 475, 231, 0 cellules jeunes sur 972 ; relais sous le sommet 3,6, 2,2, 1,3, 0,65 | non, le niveau glisse en douceur |
| Sent-elle la largeur ? | rayon 3, 5, 8 : 189, 475, 1257 cellules jeunes, soit la moitié du tissu à chaque fois | non, aucune |
| Le frein suit-il la taille ? | retenue sous le sommet 0,60, 0,30, 0,17 quand le rayon passe de 3 à 8 | à l'envers : plus large, moins freiné |
| Et avec un sommet arrondi ? | mêmes tendances (retenue 0,92, 0,58, 0,39) | la forme ne change rien |
| Et une retenue à longue portée ? | `restraint_decay=0.0003` : retenue uniforme mais 1,23, 0,77, 0,61 selon le rayon | toujours à l'envers |

Lecture :

- La boucle règle un niveau de relais, le même tout le long d'une peau éclairée. Le « tout ou
  rien » vu sur la plante n'est pas dans la boucle : c'est ce niveau uniforme qui passe d'un côté
  ou de l'autre du seuil de jeunesse, et la croissance fait le reste.
- La profondeur de la zone jeune est fixée par la portée de la cytokinine. La largeur n'est fixée
  par rien : dans un tissu uniforme le long de sa peau, aucune boucle de ce genre ne peut la
  sentir (c'est vrai aussi du modèle de Gruel, dont le domaine s'élargit avec le dôme).
- Défaut propre au modèle : la retenue est faite par la surface et perdue dans le volume, et la
  lecture « peau » faiblit sur une surface plate. Une pointe qui grossit est donc de moins en
  moins freinée : la boucle pousse au gonflement au lieu de le contenir.

Conséquence : la largeur de la pointe ne viendra pas de cette boucle. Elle doit venir du flanc
(décision A1 de Mickaël : les cellules du flanc sortent par l'auxine), ce qui demande ensemble :
une peau qui garde son auxine, une auxine qui ne garde jeune qu'à la dose d'un bout de racine, et
un relais qui porte seul la pointe. Le bloc figé sert de banc d'essai rapide pour ce réglage
(y remettre l'auxine : `light_production`, `production`).
