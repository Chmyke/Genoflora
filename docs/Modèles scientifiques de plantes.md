# Comment les scientifiques modélisent une plante à l'échelle de la cellule

Recherche documentaire du 2026-11-25, demandée par Mickaël : s'inspirer au maximum des modèles
réels, travaux récents compris. Trois recherches menées en parallèle (pointe de tige, pointe de
racine, mécanique et simulateurs), résumées ici. Ce document sert à choisir les causes à ajouter ;
il ne décide rien.

Degré de vérification de chaque source : **[L]** texte lu, **[R]** résumé ou extrait seulement,
**[M]** rappelé de mémoire, à relire avant d'en copier une équation ou une constante. Beaucoup de
sites d'éditeurs ont refusé la lecture automatique : les classiques sont souvent [M].

## 1. Ce qui ressort d'abord

- Personne n'a simulé une plante entière cellule par cellule. Les modèles cellulaires s'arrêtent à
  un organe (une pointe de racine, un méristème : quelques centaines à quelques milliers de
  cellules). Un million de cellules de plante avec leur mécanique serait un terrain neuf, sans
  référence publiée à cette taille [R].
- Presque tous les modèles de racine imposent le sens des pompes selon le tissu (une donnée
  d'entrée). Faire émerger ce sens est précisément ce que la recherche ne sait pas encore faire
  de façon consensuelle.
- Plusieurs ingrédients des modèles reconnus sont des hypothèses de modélisateurs, pas des
  molécules identifiées (le répresseur à courte portée de Gruel, les molécules « compteuses » de
  Cieslak). À manier avec la règle « jamais de béquille ».

## 2. La pointe de tige

**Boucle WUS / CLV3.** WUS est fabriqué dans un petit amas profond (centre organisateur). La
protéine WUS monte de cellule en cellule et allume CLV3 dans les cellules du sommet ; le peptide
CLV3 diffuse en retour et éteint WUS (Yadav 2011 [M] ; Daum 2014 [M]). La boucle fixe le rapport
entre les deux domaines, pas l'endroit : tous les modèles qui marchent ont besoin d'un repère venu
de l'épiderme (déjà Jönsson 2005 [M]).

**Le modèle le mieux documenté : Gruel et al. 2016, Science Advances [L]**
(https://doi.org/10.1126/sciadv.1500989). L'épiderme émet deux signaux : une cytokinine à longue
portée et un répresseur à courte portée. WUS s'allume dans la bande entre les deux seuils, donc à
profondeur fixe sous la peau, et son domaine s'élargit quand le dôme s'élargit. CLV3 demande WUS
ET la proximité de la surface. Réserve des auteurs : la cytokinine n'a jamais été mesurée dans le
méristème, le répresseur est hypothétique.

**Pourquoi CLV3 ne s'allume pas là où WUS est le plus fort.** Deux explications concurrentes : HAM,
présent en profondeur, empêche WUS d'activer CLV3 (Zhou 2018, Science [R] ; Gruel 2018 [L],
https://doi.org/10.1038/s41540-018-0072-1) ; ou bien WUS active à faible dose et réprime à forte
dose (Perales 2016 [R]). Non tranché.

**Le réveil par la lumière : Pfeiffer et al. 2016, eLife [L]**
(https://doi.org/10.7554/eLife.17023). Dans le noir, la pointe dort. WUS demande deux entrées :
la lumière, perçue hors du méristème et relayée par la cytokinine, et du sucre, comme énergie et
non comme signal. Les deux convergent sur la kinase TOR.

**Phyllotaxie.** Chaque cellule de l'épiderme tourne ses pompes vers la voisine la plus riche en
auxine (Jönsson 2006 [M], https://doi.org/10.1073/pnas.0509839103 ; Smith 2006 [M]). L'auxine
converge en pics, qui vident leurs alentours : le « champ inhibiteur » n'est pas une molécule,
c'est le manque d'auxine (Reinhardt 2003 [R]). Pompes vers le gradient à faible auxine, dans le
sens du flux à forte auxine (Bayer 2009 [M]). Une cellule ne peut pas mesurer un flux net ;
Cieslak et al. 2015 [L] (https://doi.org/10.1093/jxb/erv262) proposent des molécules qui comptent
les passages à chaque paroi, hypothétiques. Hartmann et al. 2019 [L]
(https://doi.org/10.1371/journal.pcbi.1006896) le font en 3D, sans croissance, donc sans vraie
phyllotaxie. Le centre de la pointe est insensible à l'auxine [M].

**Feuille plate et finie.** Le méristème est déjà partagé en anneaux, identité « dessus » au
centre et « dessous » autour ; les organes naissent à la frontière, et c'est cette frontière qui
pousse de côté pour faire le limbe (Caggiano 2017 [M]). La feuille s'arrête parce qu'elle perd les
gènes du méristème [M].

**Ramification.** Un bourgeon ne pousse que s'il arrive à exporter son auxine dans la tige, et
une tige déjà remplie par une source plus ancienne l'en empêche (Prusinkiewicz 2009 [R],
https://doi.org/10.1073/pnas.0906696106 ; mise à jour Nahas 2025 [L],
https://doi.org/10.1371/journal.pbio.3003395). Mais le sucre agit d'abord : après décapitation,
les bourgeons démarrent avant que l'auxine ait eu le temps de voyager ; la pointe les retient en
prenant le sucre (Mason 2014 [R], https://doi.org/10.1073/pnas.1322045111 ; Bertheloot 2020 [R]).

**Ordres de grandeur.** Méristème d'inflorescence : 1 600 à 2 200 cellules (Klawe 2020 [L]) ;
niche environ 500 (Reddy 2008 [R]) ; centre organisateur 10 à 20 cellules [M].

## 3. La pointe de racine

**La fontaine inversée.** L'auxine descend par le cylindre central, s'étale dans la coiffe,
remonte par la coiffe latérale et l'épiderme, puis rentre par le cortex. La boucle piège
l'auxine : un maximum se forme au centre quiescent par le transport seul (Grieneisen 2007 [R],
https://www.nature.com/articles/nature06215).

| Tissu | Sens des pompes |
|---|---|
| Cylindre central | vers la pointe |
| Coiffe (columelle) | aucun : l'auxine s'étale de côté |
| Épiderme, coiffe latérale | vers le haut |
| Cortex | vers la pointe, et vers l'intérieur |

Les transporteurs d'entrée (AUX1/LAX), non orientés, décident quels tissus retiennent l'auxine ;
les pompes seules ne reproduisent pas le motif mesuré (Band 2014 [R]).

**Ce qui borne le méristème.** PLETHORA (PLT) n'est pas une lecture directe de l'auxine : une
auxine forte et durable allume PLT dans un domaine étroit, puis la protéine, lente et stable,
s'étale par la croissance (elle se dilue de division en division) et par passage de cellule en
cellule (Mähönen 2014 [R], https://www.nature.com/articles/nature13663). Auxine et PLT se
renforcent ; le reflux confine PLT à la niche (Rutten et ten Tusscher 2021 [L],
https://pmc.ncbi.nlm.nih.gov/articles/PMC8125115). La cytokinine creuse un minimum d'auxine qui
marque la frontière entre division et allongement (Di Mambro 2017 [R]).

**Arrêt de l'allongement.** Une cellule cesse de s'allonger quand elle atteint une longueur seuil,
pas après un temps ni à une distance de la pointe (Pavelescu 2018 [L],
https://pmc.ncbi.nlm.nih.gov/articles/PMC5787709/).

**Auxine.** Fabriquée surtout sur place à la pointe ; celle qui vient de la tige ne suffit pas
(Brumos 2018 [R]). Sous forme chargée dans la cellule, elle ne peut en sortir que par les pompes
(modèle chimiosmotique). Les membranes, pas la diffusion dans la cellule, limitent le transport.

**Gravité.** Des grains d'amidon tombent dans la coiffe et déplacent les pompes vers le bas ;
l'auxine s'accumule du côté inférieur, qui s'allonge moins (Band 2012 [R]).

**Racines latérales.** Soit une horloge de gènes (Moreno-Risueno 2010 [M]), soit, sans horloge,
le reflux plus les différences de taille de cellules dues à la croissance (van den Berg 2021 [R],
https://pubmed.ncbi.nlm.nih.gov/34343477/) : la seconde convient mieux à une simulation où tout
émerge.

**Niche.** Le parallèle WOX5/CLE40 avec WUS/CLV3 est lâche : la niche de racine est surtout
placée par le maximum d'auxine et PLT [R].

## 4. Mécanique, division et simulateurs

**Croissance.** Une cellule grandit quand sa pression interne dépasse ce que sa paroi supporte
(Lockhart 1965 [M]). Version à plusieurs cellules avec échange d'eau entre voisines : Cheddadi et
al. 2019 [L] (https://journals.plos.org/ploscompbiol/article?id=10.1371/journal.pcbi.1007121).
La pression y est un résultat, pas une donnée, et une cellule à paroi molle prend l'eau de ses
voisines. L'auxine ramollit la paroi [M]. Les fibres de cellulose se posent selon la direction où
la paroi est le plus tirée, et la cellule s'allonge en travers (Hamant 2008 [R]). Les pompes
peuvent suivre la paroi la plus tendue (Heisler 2010 [L],
https://journals.plos.org/plosbiology/article?id=10.1371/journal.pbio.1000516).

**Division.** Ni taille fixe ni ajout fixe : un mélange des deux (Willis 2016 [R]). Le plan de
division est la plus petite surface qui passe par le centre (Moukhtar 2019 [L]), ou suit la
tension là où le tissu est tiré dans un sens (Louveaux 2016 [R]).

**Représentations.**

| Famille | Force | Faiblesse pour une plante |
|---|---|---|
| Sommets (parois explicites) | voisines fixes, parois réelles | comptabilité lourde, difficile sur GPU |
| Éléments finis sur les parois | vraies contraintes | coûteux |
| Centres de cellules (le nôtre) | le moins cher, fait pour le GPU | voir ci-dessous |

Dans un modèle à centres, les cellules glissent et changent de voisines, ce qui n'arrive jamais
dans une plante (collées par la lamelle moyenne) [R]. Parades connues : liens permanents créés à
la division et jamais refaits, ou parois déduites d'un découpage de Voronoï.

**Diffusion.** Un schéma explicite reste stable et positif tant que la somme des échanges d'une
cellule par pas ne dépasse pas ce qu'elle contient ; la limite vient de la cellule la plus entourée
[M]. Parades : pondérer par la surface de contact, plafonner la sortie, schéma implicite. C'est
exactement la panne diagnostiquée dans la Feuille de route.

**Plante entière.** Les modèles de plante entière raisonnent en organes, pas en cellules. Le
sucre circule par différence de pression créée par sa propre concentration (flux de Münch) [R].

## 5. Ce que cela désigne pour Genoflora

Rapprochements avec ce qui existe déjà :

- La cytokinine faite à la lumière par les cellules du dessus joue le rôle de la cytokinine de
  l'épiderme chez Gruel, et l'ombre celui du repère de profondeur.
- La double façon de placer les pompes est celle de Bayer 2009.
- Le frein de pression est la loi de Lockhart réduite à sa conséquence.

Causes manquantes, par ordre de proximité avec les prochaines étapes :

1. **Taille de la pointe de tige.** Un signal de retenue seul ne suffit dans aucun modèle publié :
   il faut un repère de surface (deux portées) et un relais qui monte (WUS mobile). Le signal
   unique validé le 2026-11-25 est donc à mesurer d'abord pour ce qu'il donne ; si le centre du
   foyer mûrit avant ses bords, la cause manquante sera ce relais.
2. **Maximum étroit de la racine.** Les modèles l'obtiennent en imposant le sens des pompes par
   tissu. Sans identité de tissu, deux pistes locales : une couche externe qui se reconnaît (une
   cellule sait qu'elle touche le sol) et pompe vers le haut ; un facteur lent et stable de type
   PLT, allumé par une auxine durable et dilué par les divisions.
3. **Feuilles.** Elles demandent une identité dessus / dessous et un centre insensible à
   l'auxine : rien de tel n'existe encore ici.
4. **Sucre.** Ressource consommée et rapide, absente du modèle : elle explique le réveil de la
   pointe et la retenue des bourgeons.
5. **Voisines permanentes.** Les cellules d'une plante ne glissent pas. À mesurer d'abord : à quel
   point les nôtres changent de voisines.
