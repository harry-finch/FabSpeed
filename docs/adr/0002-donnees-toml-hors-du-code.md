# ADR-0002 : Constantes de coupe en données TOML, hors de l'algorithme

Status: Accepted
Date: 2026-10-22

## Context

Le skill `feeds-speeds` interdit de cacher des avances, profondeurs ou coefficients empiriques dans l'algorithme. Les fabmanagers doivent pouvoir corriger une constante sans recompiler, et chaque recommandation doit être traçable.

## Decision

- Machines, matières, fraises, préréglages et symptômes dans `data/*.toml` commentés ; bibliothèques FabCAM `*.ivac-toolset.json` acceptées en plus.
- Facteurs du moteur dans `EnginePolicy`, chacun nommé et documenté, avec valeurs par défaut dans le code et surcharge possible par `data/policy.toml`.
- Révision du jeu de données (`dataset.toml`) exposée dans chaque recommandation.
- Chargement déterministe (ordre alphabétique), validation stricte (`deny_unknown_fields`, cohérence des références), refus du catalogue en cas d'erreur.

## Alternatives considered

- JSON : pas de commentaires, moins lisible pour un fabmanager.
- Base SQLite : plus lourde, inutile pour quelques centaines d'entrées, moins lisible dans Git.
- Constantes dans le code : contraire au skill et aux besoins du lab.

## Consequences

Les changements de données sont versionnés et relus dans Git. La politique par défaut vit dans le code pour rester testée ; `--print-default-policy` fournit un point de départ.

## Validation

Tests de chargement, de validation du catalogue (ENG-21) et de calibrage (CAL-01/02).
