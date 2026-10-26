# ADR-0010 : Types de fraises (V, hémisphérique, surfaçage, compression)

Status: Accepted
Date: 2026-10-22

## Context

Le modèle calculait toutes les fraises comme des fraises droites de diamètre D. Les fraises en V étaient refusées, les hémisphériques traitées comme droites (vitesse de coupe et copeau réels surestimés en passe peu profonde), et une grosse fraise de surfaçage aurait reçu un copeau proportionnel à son diamètre (≈ 1,7 mm/dent pour Ø50). Les règles du skill `feeds-speeds` imposent des facteurs nommés, documentés et testés, et l'honnêteté sur ce qui n'est pas calé.

## Decision

- **Un diamètre qui coupe** `Dc` (géométrie) et un **diamètre de référence** `D_ref` (copeau et passe visés), par type :
  - V : `Dc = min(D, 2 × profondeur × tan(θ/2))`, `D_ref = max(Dc, 0,5 × D)` (pointe soutenue par le corps ; sans ce plancher, la gravure peu profonde tombait au minimum anti-frottement et à 0,1 mm de passe) ; angle obligatoire ;
  - hémisphérique : `Dc = 2 √(ap (D − ap))` si ap < D/2 ; rotation et copeau programmé compensés par D/Dc, plafonnés (1,5) ; passe × 0,5 ;
  - surfaçage : `D_ref = min(D, 12 mm)`, largeur réelle conservée pour la puissance ;
  - compression : calcul de fraise droite, rappel de la première passe (au-delà de la partie upcut, longueur facultative sur la fiche).
- La passe est calculée **avant** la rotation (elle fixe Dc d'une hémisphérique). L'évaluation (copeau réel, vitesse de coupe, efforts) utilise toujours le Dc de la passe retenue, y compris pour les valeurs testées ou saisies.
- Facteurs dans `EnginePolicy.families` (copeau, passe, plafond du diamètre de référence), `ball_nose_max_compensation`, `v_bit_reference_min_ratio` ; surchargeables par `data/policy.toml`.
- V et surfaçage portent l'avertissement `tool_type_not_calibrated` : aucun essai du lab ne les a encore calés.

## Alternatives considered

- Garder les V hors calcul (réglages fournisseur seuls) : les hobbyistes gravent beaucoup en V ; une valeur prudente, expliquée et signalée comme non calée, est plus utile qu'un refus.
- Copeau d'une V calé sur la seule largeur gravée : physiquement séduisant, mais donnait 0,02 mm/dent et des passes de 0,1 mm, loin de toutes les références fabricants.
- Modèle d'engagement 3D pour l'hémisphérique : hors périmètre (la FAO connaît le relief, pas FabSpeed).

## Consequences

Les fraises droites et à compression gardent exactement le calcul précédent (facteurs 1, calibrage CAL-01 inchangé). Les valeurs V, hémisphérique et surfaçage sont des estimations géométriques prudentes à confirmer par des essais ; le premier essai réussi les remplace, comme partout.

## Validation

ENG-28 à ENG-32 ([VALIDATION.md](../VALIDATION.md)) ; tableau de valeurs relu (V 60° Ø12 à 1 mm dans le contreplaqué : 18 000 tr/min, 3 700 mm/min, 0,10 mm/dent, une passe) ; parcours dans le navigateur sur une copie des données (fraise en V créée par le formulaire rapide, gravure, schéma avec la pointe selon l'angle et la largeur gravée ; hémisphérique du lab en finition).
