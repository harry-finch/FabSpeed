# ADR-0004 : Assistant « Aller plus vite » et diagnostic inclus dès la v0.1

Status: Accepted
Date: 2026-10-22

## Context

Le skill `feeds-speeds` (écrit pour l'intégration dans FabCAM) reporte l'assistant de diagnostic après la qualification du moteur. Pour FabSpeed, outil autonome destiné aux hobbyistes, le besoin exprimé est explicite : un guide qui indique quels paramètres augmenter pour aller plus vite, et quoi corriger selon ce que l'on observe.

## Decision

- Inclure des leviers déterministes « Aller plus vite » et un diagnostic par symptômes, sans apprentissage automatique ni télémétrie.
- Facteurs des symptômes en données (`data/advisor.toml`).
- Toute correction est un `RequestPatch` appliqué seulement sur clic, puis réévaluée par le moteur comme n'importe quel réglage.
- Garde-fous : pas de levier d'accélération pour les métaux (usinage supervisé) ni tant qu'un avertissement bloquant subsiste ; profondeur augmentée par paliers (+50 % au plus, ≤ 1 × D en pleine largeur).

## Alternatives considered

- Reporter l'assistant : priverait les usagers du guide demandé.
- Appliquer automatiquement les corrections : contraire au principe « le moteur propose, l'utilisateur décide ».

## Consequences

Le périmètre du skill pour FabCAM reste inchangé ; FabSpeed peut servir de terrain d'essai avant une éventuelle intégration.

## Validation

ENG-17 à ENG-19, ENG-23, ENG-24 ; retours d'usage au lab.
