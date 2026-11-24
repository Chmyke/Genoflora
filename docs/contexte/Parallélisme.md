# Parallélisme : CPU, threads ou GPU

*2026-11-24. Recherche rapide ; « vérifié » = source ouverte, le reste est un souvenir à revérifier.*

## Ordres de grandeur

| Étape | Cellules actives | Besoin |
|---|---|---|
| Jalon 2, phyllotaxie sur une surface | centaines à quelques milliers | un seul fil suffit (souvenir : les modèles de Jönsson et Smith tournaient sur un PC de 2006) |
| Méristème 3D, quelques pointes | dizaines de milliers | plusieurs fils CPU |
| Plante entière tout en cellules | centaines de milliers et plus | GPU, ou le modèle hybride de la note (tissu figé en organes) |

Le modèle hybride de la note réduit le besoin : seules les pointes et le cambium sont simulés finement.

## Ce que dit l'existant (vérifié)

- Dans un modèle cellulaire « centre-based », la mécanique (voisins qui se repoussent) est
  l'étape la plus coûteuse (BioDynaMo, Hesam et al. 2021, arXiv 2105.00039).
- Les décisions de la cellule (cycle, division, gènes) vivent sur un temps plus lent que la
  mécanique : environ 60 pas de mécanique par pas de cellule (Stack et Macklin, OpenACC 2021).
  On peut donc paralléliser fort la mécanique et garder le reste simple.
- Les GPU donnent des gains de plusieurs ordres de grandeur, mais surtout sur des modèles simples
  (peu de dynamique interne). FLAME GPU a fait de la croissance de peau avec division.
- Une somme flottante parallèle de `rayon` n'est pas reproductible (ordre de combinaison choisi par
  le vol de travail). Remède : blocs de taille fixe, résultats rangés par indice, fusion en série
  dans un ordre fixe ; interdire `par_iter().sum()` par clippy.
- CubeCL écrit des noyaux GPU en Rust (`#[cube]`) et vise wgpu (donc Vulkan, DX12, Metal), CUDA et
  autres. API encore jeune (versions 0.x).

## Difficultés propres au GPU

- **Division** : le nombre de cellules change. Il faut une compaction (somme préfixe) pour ajouter
  les filles, et des tampons réalloués.
- **Voisinage** : grille uniforme reconstruite à chaque pas (tri par case), classique mais à écrire.
- **Déterminisme** : atomiques et flottants donnent des bits différents d'un GPU à l'autre ; les
  tests deviennent des tolérances, plus des égalités.
- **Tests** : la vérification doit tourner sans carte graphique (adaptateur logiciel ou CPU).

## Recommandation

1. Écrire la simulation en données « structure de tableaux » (positions, rayons, concentrations
   en tableaux séparés) et un pas pur `état(t) -> état(t+1)` à double tampon : chaque cellule lit
   l'ancien état, écrit le nouveau. C'est la forme qui se parallélise, sur CPU comme sur GPU.
2. Les changements de structure (division, mort) se décident en parallèle puis s'appliquent en
   série, dans l'ordre des indices : déterminisme garanti.
3. Un seul fil tant qu'un banc d'essai ne prouve pas le besoin. Puis `rayon` sur la mécanique et
   la diffusion, avec les réductions à blocs fixes. Le test « mêmes bits » reste vrai.
4. GPU seulement quand le CPU plafonne (au-delà d'environ 100 000 cellules actives, à mesurer) :
   alors la mécanique d'abord, via CubeCL ou wgpu, et une version CPU de référence gardée comme
   oracle des tests.

## Sources

- https://arxiv.org/abs/2105.00039 (BioDynaMo GPU)
- https://pmc.ncbi.nlm.nih.gov/articles/PMC2936904 (FLAME GPU, cellules)
- https://docs.rs/crate/rustyml/0.14.0/source/guide/en/src/Chapter-06/6.3._Parallel_Reductions.md
- https://github.com/tracel-ai/cubecl
