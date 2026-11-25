# Genoflora

*Open-source simulator of plant growth timelapses, emerging from cells that all share one small
genome. Nothing about the plant's shape or motion is coded: it has to emerge.*

Un simulateur open source de timelapse de plante. Chaque cellule porte le même petit génome
(quelques gènes et hormones, auxine en tête) ; la plante pousse par division, échanges d'hormones,
lumière, gravité, eau. Aucune forme ni aucun mouvement n'est écrit dans le code : la spirale des
feuilles, la circumnutation, les tropismes doivent émerger des règles locales des cellules.

On regarde, on n'agit pas : pas d'interaction, et un rendu cellulaire où l'on peut zoomer jusqu'à
voir les cellules vivre (ressources qui circulent, pores qui s'ouvrent, cellules qui meurent).

État : tout début. Rien ne pousse encore.

## Lire

- `docs/Note de faisabilité.md` : l'idée, l'existant, le modèle, les jalons.
- `docs/contexte/Feuille de route.md` : ce qui est décidé, ouvert, et la suite.
- `docs/Verdict des essais.md` : les essais CPU, GPU et CPU + GPU qui ont choisi l'architecture (moteur GPU, juge CPU pour la vérification).

## Vérifier

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File check.ps1
```

Licence : MIT.
