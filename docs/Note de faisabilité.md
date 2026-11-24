# Simulation de plante cellulaire : note de faisabilité

*2026-11-24. Recherche faite par Claude ; « vérifié » = page ouverte, « souvenir » = non revérifié.*

## L'idée

Faire pousser une plante entière, en 3D, à partir de cellules simplifiées. Chaque cellule porte le
même petit « génome » (quelques gènes et hormones) ; la forme de la plante n'est écrite nulle part,
elle doit émerger.

## Ce qui existe déjà

Chaque morceau existe quelque part, jamais les quatre ensemble (plante entière, 3D, cellules avec
gènes, temps réel). Confiance : environ 80 %.

| Ce qu'on veut | Qui s'en approche | Ce qui manque |
|---|---|---|
| Plante entière à partir de cellules | Biomaker CA (Google, 2023), Mito (jeu, 2020), jouets itch.io | Tout est en 2D |
| 3D, cellules, gènes et auxine | Simulateur de Jönsson, MorphoMechanX (Mosca), Gruel 2016 (méristème WUS/CLV3) | Un seul organe ou la pointe, jamais en temps réel |
| Cellules 3D en temps réel | jax-morpho (1 à 2 millions de cellules, GPU), splant (amateur, inachevé) | Aucune biologie végétale dedans |
| Croissance 3D apprise | Automates cellulaires neuronaux 3D (arbre Minecraft, 2021) | Copie une forme cible, sans gènes ni hormones |
| Plante entière en 3D | CPlantBox, L-Py, GroIMP | S'arrêtent à l'organe, pas de cellules |
| Génome vers organisme | Cell Lab (jeu) | 2D, quelques dizaines de cellules, pas végétal |

Le pont « cellules de la pointe → architecture de la plante entière » n'a été trouvé dans aucune
publication. C'est le trou que le projet occuperait.

## Pourquoi c'est tenable

Une plante ne pousse qu'à ses pointes (méristèmes des tiges et des racines, plus le cambium qui
épaissit). Le tissu mûr se fige. Donc :

- **Zones actives** : quelques centaines à quelques milliers de cellules simulées finement par
  méristème.
- **Tissu figé** : les cellules sorties de la pointe gardent leur type et deviennent de la
  géométrie (organe, segment), mises à jour rarement (lumière reçue, sucre, épaississement).

C'est un modèle hybride : cellules là où ça se décide, organes là où ça ne bouge plus. Les
chercheurs ne l'ont pas fait parce qu'ils visent la précision d'un organe ; un projet qui vise la
plante entière et crédible peut se le permettre.

## Modèle simplifié proposé

- **Cellule** : position, rayon, type (souche, tige, feuille, racine, vasculaire), 3 à 6
  concentrations.
- **Mécanique** : sphères qui se repoussent et s'attachent (« centre-based »), la méthode la moins
  coûteuse en 3D. Pas de parois ni d'éléments finis.
- **Génome** : un petit réseau de régulation. Chaque gène s'active selon les concentrations locales
  et produit une substance ou déclenche un comportement (diviser, s'allonger, se différencier).
- **Signaux de départ** : auxine transportée de cellule en cellule par des pompes orientées (PIN),
  couple WUS/CLV3 qui maintient la pointe, lumière, sucre.

## Jalons mesurables (vérifiables par un test, pas à l'œil)

1. **Méristème qui tient** : la zone de cellules souches garde une taille stable sur 1000 pas
   (WUS/CLV3).
2. **Phyllotaxie** : les feuilles naissent en spirale ; angle moyen entre feuilles successives
   proche de 137,5°, sans que la spirale soit codée.
3. **Dominance apicale** : couper la pointe déclenche la pousse des bourgeons latéraux.
4. **Phototropisme** : une lumière latérale courbe la tige vers elle.
5. **Plante entière** : tige, feuilles et racine ensemble, en temps réel.

Le jalon 2 est le premier vrai « ça marche ». Il se fait d'abord sur une surface (comme Smith 2006,
qui ne simule que la couche externe), donc presque en 2D.

## Risques

- **Réglage** (le principal) : un réseau de gènes peut ne rien produire de stable pendant des
  semaines. Parade : partir des modèles publiés (Jönsson 2006, Smith 2006, Gruel 2016) plutôt que
  d'inventer les équations.
- **Passage cellules → organes** : c'est la partie inédite, sans référence à copier.
- **Mécanique 3D** : à garder grossière, sinon le coût explose.

## Références pour construire

- Jönsson et al. 2006, PNAS : phyllotaxie par transport polarisé de l'auxine.
  https://resolver.caltech.edu/CaltechAUTHORS:JONpnas06
- Smith et al. 2006, PNAS : modèle plausible de phyllotaxie, couche externe seulement.
  https://www.algorithmicbotany.org/papers/phyllo.pnas2006.pdf
- Gruel et al. 2016, Science Advances : réseau WUS/CLV3 du méristème en 3D.
  https://pmc.ncbi.nlm.nih.gov/articles/PMC4846443
- Comparaison des familles de modèles cellulaires (coûts), Bull Math Biol 2020.
  https://link.springer.com/article/10.1007/s11538-020-00810-2
- Biomaker CA (le plus proche en esprit, 2D).
  https://google-research.github.io/self-organising-systems/2023/biomaker-ca
- Automates cellulaires neuronaux 3D. https://github.com/real-itu/3d-artefacts-nca
- Prusinkiewicz et Runions 2012 : gènes vers architecture, par modules.
  https://algorithmicbotany.org/papers/tansley.np2012.html

## Non vérifié

Licences et activité récente de plusieurs outils de recherche ; travaux de 2025 et 2026 absents des
index ; aucun fil Reddit trouvé directement.

## Verdict

Ça vaut le coup. La combinaison visée n'existe pas, elle est tenable grâce à la croissance par les
pointes, et chaque étape a un critère de réussite chiffré. Premier pas conseillé : le jalon 2
(spirale à 137,5°) sur une surface, avant toute 3D.
