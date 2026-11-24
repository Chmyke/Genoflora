# Parallélisme : CPU, threads ou GPU

*2026-11-24, deux passes de recherche. « Vérifié » = source ouverte ; « souvenir » = à revérifier.*

## Pourquoi le GPU est tentant

- Un moteur cellulaire GPU récent (jax-morpho, alpha) annonce 1 à 2 millions de cellules à environ
  32 ms par pas sur une RTX 4090, chiffre de ses auteurs (vérifié sur leur page, non reproduit).
- Les modèles cellulaires sur GPU gagnent plusieurs ordres de grandeur sur le CPU (BioDynaMo 2021,
  FLAME GPU) ; FLAME GPU a fait pousser un tissu de peau avec division (vérifié).
- Atout propre au projet : les tampons de la simulation peuvent servir directement au rendu
  (positions des cellules = instances dessinées), sans aller-retour CPU.
- Mémoire négligeable : un million de cellules à 64 octets = 64 Mo.

## Risques vérifiés et parades

| Risque | Ce qui se passe | Parade | Gravité après parade |
|---|---|---|---|
| Nombre de cellules qui change (mitose, mort) | Il faut ajouter des filles sans trou ni collision d'indices ; FLAME GPU signale la « parcimonie » due aux naissances et morts | Capacité préallouée ; décision de division en parallèle, puis somme préfixe pour placer les filles dans un ordre fixe | Moyenne |
| Somme préfixe portable | L'algorithme rapide en une passe (« decoupled look-back ») suppose que le GPU garantit la progression des groupes ; Apple et ARM ne le font pas, la machine peut se figer | Somme préfixe en plusieurs passes (réduire puis balayer), lente mais sûre ; nos tableaux sont petits | Faible |
| Pas d'addition atomique de flottants en WGSL | Accumuler des forces ou des flux d'auxine par « dispersion » est impossible ou non déterministe | Toujours « rassembler » : chaque cellule lit ses voisines et calcule sa propre somme, dans un ordre fixe. Aucun atomique flottant | Faible |
| Déterminisme | Les atomiques et l'ordre des opérations changent les derniers bits ; d'un GPU à l'autre, jamais garanti (FMA, fonctions transcendantes) | Même GPU : bits identiques si rassemblement et ordre fixe des voisins. Entre machines : tolérance. Nos critères (angle 137,5° ± 5°) sont statistiques, donc robustes à ce bruit | Moyenne |
| Recherche des voisines | 36 % du temps chez BioDynaMo (la mécanique : 51 %) | Grille uniforme reconstruite à chaque pas par tri par comptage | Faible |
| Watchdog Windows (TDR) | Un calcul GPU de plus de 2 s fait réinitialiser le pilote (« device lost ») | Pas courts, plusieurs petits envois par image ; gérer la perte du périphérique | Faible |
| Tests sans carte graphique | La vérification doit tourner partout | wgpu se teste sur WARP (DX12 logiciel, Windows) et lavapipe (Linux), comme son propre CI | Faible |
| Seulement des flottants 32 bits | Pas de `f64` en WGSL ; une grande plante perd en précision loin de l'origine | Coordonnées locales par organe ou par pointe | Faible |
| Divergence | Si chaque type de cellule suit une branche de code, les cœurs attendent | Génome = données (matrice de poids), le même code pour toutes les cellules | Faible |
| Outillage et débogage | Pas de `println!` dans un noyau, débogueurs graphiques seulement (RenderDoc, souvenir) | Implémentation CPU de référence, comparée au GPU par les tests | Moyenne |
| Langage des noyaux | WGSL : stable, verbeux. CubeCL (Rust `#[cube]`, cible wgpu, CUDA, ROCm) : 0.x, API mouvante. rust-gpu : orienté graphique, expérimental | WGSL à la main d'abord ; CubeCL à réévaluer | Moyenne |
| Coût de développement | Chaque règle s'écrit en noyau et se teste contre la référence (souvenir : 2 à 3 fois l'effort CPU) | Ne porter sur GPU que les étapes chaudes | Haute |

Le vrai risque n'est donc pas technique (chaque problème a une parade connue), c'est le **coût
d'itération** : le projet est un bac à sable de règles biologiques qu'on va changer souvent, et
chaque changement coûte plus cher sur GPU.

## Architecture recommandée

1. `geno_sim` (CPU, pur) reste la **référence** : déterministe au bit, lisible, où une règle naît.
2. Les données sont dès maintenant en structure de tableaux, avec un pas à double tampon
   (`état(t) -> état(t+1)`, rassemblement sans atomique) et une division en deux temps (décider,
   puis placer par somme préfixe). C'est exactement la forme d'un noyau GPU.
3. Un crate `geno_gpu` (wgpu) porte les étapes chaudes (mécanique, voisinage, diffusion
   d'auxine). Ses tests comparent au CPU, à tolérance près, sur WARP dans `check.ps1`.
4. Le rendu lit les tampons GPU directement : choisir un rendu sur wgpu (Bevy en est un).

## Sources

- https://arxiv.org/abs/2105.00039 (BioDynaMo GPU, répartition du temps)
- https://pmc.ncbi.nlm.nih.gov/articles/PMC2936904 (FLAME GPU, cellules)
- https://eprints.whiterose.ac.uk/128797/1/1-s2.0-S1569190X18300170-main.pdf (naissances et morts sur GPU)
- https://pypi.org/project/jax-morpho/ (1 à 2 millions de cellules)
- https://gridwise-webgpu.github.io/gridwise/scan-and-reduce (somme préfixe et progression)
- https://escholarship.org/uc/item/0bk9z4bt (Decoupled Fallback, somme préfixe portable)
- https://nvidia.github.io/warp/v1.19/user_guide/execution_and_performance/deterministic_execution.html
- https://learn.microsoft.com/en-sg/WINDOWS-HARDWARE/DRIVERS/display/timeout-detection-and-recovery
- https://docs.rs/crate/wgpu/^0.20.0 (CI sur WARP et lavapipe)
- https://github.com/tracel-ai/cubecl
- https://docs.rs/crate/rustyml/0.14.0/source/guide/en/src/Chapter-06/6.3._Parallel_Reductions.md

## Décision : trois essais avant de choisir (2026-11-24, validée par Mickaël, pas commencée)

Trois preuves de concept jetables dans `Spikes/`, sur le **même scénario** :
une boule de cellules qui se poussent (mécanique), échangent une hormone (diffusion) et se
divisent tous les K pas (naissances). Ce scénario touche les trois points durs : calcul lourd,
échange entre voisines, nombre de cellules qui change.

| Essai | Ce qui tourne où |
|---|---|
| CPU seul | tout sur le processeur, multithread (`rayon`) |
| GPU seul | tout sur la carte (wgpu, WGSL), divisions comprises |
| CPU + GPU | mécanique et diffusion sur GPU ; décisions et divisions sur CPU, tous les K pas |

Mesures, fixées avant de coder : temps par pas à 1 000, 10 000, 100 000 et 1 000 000 de cellules ;
même résultat que la version CPU à tolérance près ; effort (lignes, heures) et douleur de débogage.
Le choix se fait sur ce tableau, puis les essais sont jetés.
