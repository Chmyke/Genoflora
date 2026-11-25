# Plan des essais : CPU, GPU, CPU + GPU

*Rédigé le 2026-11-24, exécuté le 2026-11-25 (résultats et écarts au plan : `docs/Verdict des essais.md` ; le code des essais, supprimé le 2026-11-25, reste dans l'historique git au commit `331c7be`). But unique : choisir l'architecture de calcul de Genoflora
sur des mesures, pas sur une intuition. Tout le code des essais est jetable.*

Contexte et risques : `docs/contexte/Parallélisme.md`.

## 1. Le scénario commun

Les trois essais simulent exactement la même chose, avec les mêmes paramètres et la même graine.
Aucune biologie réelle : seulement les trois difficultés techniques du futur moteur.

**Une cellule** : identifiant `u32` stable, position `[f32; 3]`, rayon `f32`, hormone `f32`.
Stockage en structure de tableaux (un tableau par champ), capacité préallouée.

**Départ** : N cellules sur un réseau cubique bruité, dans une sphère. Bruit tiré d'un hachage
sans état `hash(graine, id)` (même résultat sur CPU et GPU, aucun générateur à état).

**Un pas** (`dt` fixe), dans cet ordre :

1. **Voisinage** : grille uniforme, case = diamètre maximal ; tri des cellules par case
   (comptage, somme préfixe, rangement). Ordre des voisines fixé par l'ordre du tri.
2. **Mécanique** (« rassembler », aucun atomique) : pour chaque voisine à distance
   `d < ri + rj`, répulsion `k_rep * (ri + rj - d)` ; si `ri + rj <= d < 1,2 (ri + rj)`,
   adhésion `k_adh * (d - ri - rj)`. Mouvement amorti : `x += dt * F / gamma`.
3. **Diffusion de l'hormone** (« rassembler ») : `h_i += dt * D * somme(h_j - h_i)` sur les
   voisines, plus production `p` si `x > 0` et dégradation `-lambda * h_i`. Flux symétrique, donc
   la masse totale se conserve quand `p = lambda = 0`.
4. **Croissance** : `r += dt * g` jusqu'à `r_div`.
5. **Division**, tous les `K` pas seulement : chaque cellule avec `r >= r_div` le signale ; une
   somme préfixe sur les signaux donne à chaque fille son indice et son identifiant, dans l'ordre
   des indices (déterministe). Volume conservé : `r_fille = r / racine_cubique(2)` ; axe tiré de
   `hash(graine, id, pas)` ; hormone partagée en deux. Plafond : aucune division au-delà de la
   capacité.

**Paramètres** : dans un fichier `scenario.toml` lu par les trois essais (valeurs choisies au
premier lancement de la référence, puis figées).

## 2. Les trois essais

| Essai | Voisinage | Mécanique, diffusion, croissance | Division | Outil |
|---|---|---|---|---|
| **CPU** | CPU | CPU, `rayon` par blocs fixes | CPU, en série | Rust + `rayon` |
| **GPU** | noyau GPU | noyaux GPU | noyau GPU (signal, somme préfixe en plusieurs passes, écriture) | `wgpu` + WGSL |
| **CPU + GPU** | noyau GPU | noyaux GPU | CPU tous les `K` pas : relire les rayons, décider, renvoyer les filles | `wgpu` + WGSL + Rust |

Règles communes : pas de `HashMap`, pas d'atomique flottant, aucune somme `rayon` libre
(`par_iter().sum()` interdite), tout aléa par hachage sans état. Côté GPU : chaque envoi reste
court (watchdog Windows de 2 s) ; aucune relecture CPU dans la boucle sauf la division de
l'essai CPU + GPU.

## 3. La référence

Avant les essais : `reference`, CPU **un seul fil, naïve** (voisins en O(N²), sommes en série).
Lente mais évidente : c'est l'oracle. Elle vit dans le crate commun et sert aux trois essais.

## 4. Protocole de validation

Chaque essai expose deux commandes : `validate` et `bench`. Un essai qui échoue à `validate` ne
compte pas au classement.

| Code | Contrôle | Seuil de réussite |
|---|---|---|
| V1 | Justesse : 1 000 cellules, 600 pas, divisions comprises, comparé à la référence | même nombre de cellules et mêmes identifiants ; écart de position ≤ 0,001 × rayon moyen ; masse d'hormone à 1e-4 près en relatif |
| V2 | Conservation : production et dégradation à 0 | masse d'hormone constante à 1e-5 près en relatif sur 1 000 pas |
| V3 | Volume conservé par la division | écart ≤ 1e-5 en relatif |
| V4 | Pas d'écrasement : après 300 pas sans division | aucun recouvrement au-delà de 20 % du rayon |
| V5 | Déterminisme : deux lancements, même machine | CPU : bits identiques (obligatoire). GPU : noter si bits identiques (souhaité, pas éliminatoire) |
| V6 | Sans carte graphique : essais GPU sur WARP | `validate` passe (lenteur acceptée) |

## 5. Protocole de mesure (`bench`)

- **Régime stable** (division coupée) à 1 000, 10 000, 100 000, 1 000 000 de cellules :
  50 pas de chauffe, puis médiane et 95e centile du temps par pas sur 200 pas, mesurés de bout en
  bout (attente de fin du GPU comprise).
- **Croissance** : de 1 000 à 100 000 cellules avec divisions ; temps total.
- **Chiffre clé** : la plus grande population tenue à **30 pas par seconde** (≤ 33 ms par pas).
- Machine de référence : Ryzen 9 7950X3D, RTX 4080 SUPER (Windows). Build `--release`, machine branchée, rien d'autre de lourd en cours ; noter CPU, GPU, pilote.
- Sortie : un fichier `Spikes/Résultats/<essai>.json` par essai, mêmes champs pour les trois.

## 6. Grille de choix (fixée avant de coder)

| Critère | Mesure | Poids |
|---|---|---|
| Échelle | population tenue à 30 pas/s | 40 % |
| Confort de changement | temps pour ajouter une règle simple (une 2e hormone) à chaque essai, en fin de parcours | 25 % |
| Croissance | temps total de 1 000 à 100 000 cellules | 15 % |
| Fiabilité | V5 et V6, nombre de bugs rencontrés | 10 % |
| Effort | heures et lignes de code | 10 % |

Règles de départage :
- Si les deux essais avec GPU tiennent moins de 5 fois la population du CPU à 30 pas/s, le
  CPU gagne : le gain ne paie pas la complexité.
- Entre GPU et CPU + GPU, le GPU seul ne gagne que s'il va au moins 2 fois plus vite sur la
  croissance ; sinon CPU + GPU, plus simple à faire évoluer.

Le verdict s'écrit dans `docs/Verdict des essais.md` ; le code des essais, supprimé le 2026-11-25, reste dans l'historique git au commit `331c7be` (tableau rempli, choix, raison), puis se reporte
dans `docs/contexte/Parallélisme.md` et la feuille de route. Le dossier `Spikes/` est ensuite
supprimé du dépôt (l'historique git le garde).

## 7. Ordre de travail au top départ

Chaque étape laisse un résultat utile même si la session s'arrête là.

1. Crate commun `spike_scenario` : cellules, paramètres, départ, hachage, référence, contrôles
   V1 à V4, format du fichier de résultats.
2. Essai **CPU** (le plus court ; sert aussi à régler les paramètres).
3. Essai **GPU** : voisinage, puis mécanique, diffusion, croissance, puis division.
4. Essai **CPU + GPU** : réutilise les noyaux de l'essai GPU, remplace la division.
5. `bench` des trois, test de la « 2e hormone », verdict.

## 8. Structure prévue

```
Spikes/                   workspace Cargo séparé, exclu du workspace principal et de check.ps1
  Cargo.toml
  scenario.toml           paramètres figés
  spike_scenario/         commun : modèle, référence, contrôles, résultats
  spike_cpu/              binaire : validate | bench
  spike_gpu/              binaire : validate | bench (+ shaders/*.wgsl)
  spike_hybrid/           binaire : validate | bench (réutilise les shaders de spike_gpu)
  Résultats/              un JSON par essai
  Verdict.md
```
