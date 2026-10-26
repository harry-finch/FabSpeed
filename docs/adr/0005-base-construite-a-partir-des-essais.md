# ADR-0005 : Repartir du calcul et construire la base à partir des essais

Status: Accepted
Date: 2026-10-22

## Context

La v0.1 chargeait les 21 réglages historiques du wiki du lab comme préréglages. Ces réglages ne sont ni datés ni rattachés à un contexte complet, et leurs plongées étaient calculées. Le lab veut repartir de zéro : partir du modèle seul, puis construire sa propre base en notant les résultats de ses essais.

## Decision

- Le jeu livré ne contient **aucun réglage** : `data/presets/` est vide, toutes les recommandations partent du calcul (`theoretical`).
- Les réglages du wiki sont archivés dans `docs/research/fablab-wiki/presets-wiki.toml`. Ils restent la référence du test de calibrage (CAL-01/02) et peuvent être réactivés en copiant le fichier dans `data/presets/`.
- Un **essai** (`Trial`) enregistre le contexte complet, les valeurs réellement utilisées, le résultat (`good`, `adjust`, `failed`), les constats, des remarques, l'opérateur, la date et les versions.
- Un essai `good` devient un réglage `tested` pour son contexte exact (détourage ≈ rainure ; poche : pas latéral inférieur ou égal). À preuve égale, le plus récent l'emporte.
- Les essais `adjust` et `failed` restent dans l'historique et déclenchent un avertissement si l'on s'apprête à refaire les mêmes valeurs.
- Les réglages testés sont dérivés des essais à chaque calcul (pas de copie) : supprimer ou archiver un essai retire immédiatement son effet.
- La fiche « statut de la fraise » (`needs_tuning`) issue du wiki est supprimée : l'état « testé ou non » se déduit des essais.

## Alternatives considered

- Garder le wiki en `reference_data` sous les essais : contraire à la demande de repartir de zéro ; reste possible par copie du fichier.
- Promouvoir un essai en préréglage par une action séparée : une étape de plus pour l'utilisateur, et deux sources de vérité.
- Moyenner les essais réussis : non explicable, et deux essais réussis peuvent viser des intentions différentes (Prudent / Normal).

## Consequences

La base du lab reflète uniquement ce qui a été essayé sur ses machines, avec date et opérateur. Les premières recommandations sont plus prudentes qu'avec le wiki (38 à 100 % de l'avance historique). Le niveau `validated` n'est pas encore attribué.

## Validation

ENG-25, TRI-01 à TRI-08, LAB-04 ; CAL-01/02 sur l'archive du wiki.

> Mise à jour (2026-10-23, ADR-0012) : l'archive du wiki est privée et hors dépôt ; le calibrage la lit si elle est présente sur le poste.
