# Verdict des essais de calcul : CPU, GPU, CPU + GPU

*2026-11-25. Machine : Ryzen 9 7950X3D (32 fils), RTX 4080 SUPER, Windows 11, wgpu 30 (Vulkan),
build `--release`. Tout ce qui suit est mesuré, sauf mention « estimation ». Fichiers bruts :
`Spikes/Résultats/*.json` (le code des essais, supprimé le 2026-11-25, reste dans l'historique git au commit `331c7be`). Plan suivi : `docs/Plan des essais.md`.*

## Résultat en une phrase

Le GPU tient environ 9 fois plus de cellules que le CPU à vitesse égale, les trois essais sont
justes, et le choix recommandé est **CPU + GPU** : un moteur GPU pour faire tourner la plante, et
un moteur CPU simple gardé comme référence, là où chaque règle naît et se teste.

## Validation : les trois essais sont justes

| Contrôle | CPU | GPU | CPU + GPU |
|---|---|---|---|
| V1a justesse exacte, 200 pas avec divisions (écart max 0,001 rayon) | 0,00014 | 0,00005 | 0,00013 |
| V1b plafond de capacité respecté | oui | oui | oui |
| V1c horizon long, 600 pas, 1 000 → 7 216 cellules : même nombre, même forme | oui | oui | oui |
| V2 hormone conservée sur 1 000 pas (max 1e-5) | 7e-8 | 6e-8 | 6e-8 |
| V3 volume conservé par la division (max 1e-5) | 5e-9 | 5e-9 | 5e-9 |
| V4 pas d'écrasement (max 20 % du rayon) | 9,8 % | 9,8 % | 9,8 % |
| V5 deux lancements identiques au bit près | oui | oui | oui |
| V6 fonctionne sans carte graphique (WARP) | sans objet | oui | oui |

Deux résultats au-delà de l'attendu : le GPU est **identique au bit près d'un lancement à
l'autre** (ce n'était que souhaité), et sur l'adaptateur logiciel WARP il donne les
mêmes écarts à la référence que l'essai CPU, à tous les chiffres affichés.

## Vitesse

Temps d'un pas en régime stable (médiane, attente du GPU comprise) :

| Cellules | CPU | GPU | CPU + GPU | Avantage GPU |
|---|---|---|---|---|
| 1 000 | 0,19 ms | 0,66 ms (0,15 par lots) | idem GPU | CPU devant si l'on attend chaque pas |
| 10 000 | 0,74 ms | 0,69 ms (0,18) | idem | ×1 (×5 par lots) |
| 100 000 | 2,7 ms | 0,84 ms (0,33) | idem | ×3 (×9 par lots) |
| 1 000 000 | 24,7 ms | 2,9 ms | idem | ×8,4 |
| 4 000 000 | 151 ms | 10,3 ms | idem | ×14,6 |
| 16 000 000 | non mesuré | 50,9 ms | idem | |

« Par lots » : 10 pas envoyés d'un coup, comme le fera un timelapse accéléré.

| Mesure | CPU | GPU | CPU + GPU |
|---|---|---|---|
| **Chiffre clé : population tenue à 30 pas par seconde** (interpolé) | 1,25 million | 11 millions | 11 millions |
| Croissance de 1 000 à 100 000 cellules (1 600 pas) | 3,26 s | 0,57 s | 0,67 s |
| Une ronde de division à 1 million de cellules | 2,5 ms | 0,76 ms | 11,4 ms |
| Pas par seconde à 1 million de cellules | 40 | 340 | 340 (environ 280 avec les divisions) |

Dans l'essai CPU + GPU, la division relit toutes les cellules vers le processeur puis les
renvoie : 15 fois plus cher que la division faite sur le GPU, mais seulement tous les 20 pas, soit
environ 20 % de temps en plus à 1 million de cellules.

## Confort et effort

Test du confort : ajouter une deuxième hormone à chaque essai, puis revalider. Les trois ont
compilé et passé la validation du premier coup (branche `essai-deuxieme-hormone`).

| | CPU | GPU | CPU + GPU |
|---|---|---|---|
| Lignes touchées pour la 2e hormone (hors partie commune, 20 lignes) | 11, dans 1 fichier | 19, dans 5 fichiers et 2 langages | comme GPU |
| Surcoût de calcul de la 2e hormone à 1 million | +7 % | +5 % | +5 % |
| Taille de l'essai (lignes) | 126 | 634 (dont 257 de shaders) | environ 550 |
| Bugs rencontrés dans l'essai | 0 | 0 | 0 |

L'effort est en lignes et non en heures : c'est une IA qui a codé, des heures ne voudraient rien
dire. Le GPU demande environ 5 fois plus de code, surtout de la tuyauterie écrite une seule fois
(tampons, somme préfixe, pipelines). Ajouter une règle ensuite coûte à peine plus que sur CPU.

## Application de la grille fixée d'avance

| Critère | Poids | CPU | GPU | CPU + GPU |
|---|---|---|---|---|
| Échelle | 40 % | 1,25 M | 11 M | 11 M (un peu moins avec divisions) |
| Confort de changement | 25 % | meilleur | bon | bon, et les décisions restent en Rust |
| Croissance | 15 % | 3,26 s | 0,57 s | 0,67 s |
| Fiabilité | 10 % | parfaite | parfaite | parfaite |
| Effort | 10 % | 126 lignes | 634 | 550 |

Règles de départage :
- Le GPU tient 8,8 fois la population du CPU (seuil : 5) : **le CPU seul est écarté**.
- Le GPU seul n'est pas 2 fois plus rapide que CPU + GPU sur la croissance (1,17 fois) :
  **CPU + GPU l'emporte**, parce qu'il est plus simple à faire évoluer.

## Recommandation

**CPU + GPU**, sous cette forme précise :

1. `geno_sim` (CPU, Rust pur) reste la **référence** : chaque règle biologique y naît, avec son
   test. C'est l'oracle qui a validé les deux moteurs GPU ici, sans une seule fausse alerte.
2. Un moteur GPU (wgpu, WGSL) fait tourner la plante : voisinage, mécanique, diffusion,
   croissance. Les essais prouvent que ça tient 11 millions de cellules à 30 pas par seconde.
3. La division peut rester **sur le GPU** : elle y est 15 fois moins chère et n'a posé aucun
   problème. Le processeur ne relit les cellules que pour les décisions rares et compliquées.
4. Les tests du moteur GPU tournent sur WARP dans `check.ps1`, comparés à la référence.

## Ce que les essais ont appris

- **Un tissu qui se divise est chaotique.** Une différence d'arrondi au 7e chiffre devient un
  écart d'une cellule entière en 600 pas. Conséquence : on ne peut comparer deux moteurs cellule
  par cellule que sur un horizon court ; au-delà, on compare le nombre, la forme et les masses.
  Le contrôle V1 du plan a été scindé pour cela (écart déclaré au plan).
- **Les forces doivent être continues.** Une adhésion qui s'arrêtait net à sa portée multipliait
  les écarts par mille. Corrigée (profil en tente). À retenir pour le vrai modèle.
- **Aucun des risques GPU redoutés ne s'est produit** : somme préfixe portable, division sur GPU,
  absence d'atomiques flottants (tout est en « rassemblement »), déterminisme, test sans carte.
- wgpu suffit : aucun besoin de CUDA.

## Limites de ces essais

- Le scénario est simple : sphères, une hormone, pas de réseau de gènes. Le vrai modèle coûtera
  plus par cellule (estimation : 3 à 10 fois), sur CPU comme sur GPU ; l'écart entre les deux
  devrait rester du même ordre.
- Le rendu n'a pas été testé (lecture directe des tampons GPU par l'affichage).
- Une seule machine, une seule carte (NVIDIA, Vulkan). « Identique au bit près » vaut pour elle.
- Le régime stable mesuré est une boule comprimée, pas une plante ramifiée.

## Lancer soi-même

```powershell
cargo build --release --manifest-path Spikes/Cargo.toml
```

Puis `Spikes/target/release/spike_cpu.exe validate` ou `bench` (de même `spike_gpu`,
`spike_hybrid`, avec `--warp` pour l'adaptateur logiciel).
