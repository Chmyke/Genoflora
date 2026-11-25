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
  puis le profil du centre au bord du foyer le plus haut (mesure `checks::profile`).
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

Prochaine étape, à trancher par Mickaël : la planche désigne deux défauts avant la fontaine
inversée de la racine, le manque de contraste entre pointe et tige (toute la tige au seuil) et la
silhouette trop épaisse. Chaque cause ajoutée se juge désormais à la planche et au bilan, pas
seulement au test.
