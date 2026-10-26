# ADR-0003 : Précédence des préréglages du lab sur le modèle

Status: Accepted — complété par ADR-0005 (les réglages connus viennent désormais des essais)
Date: 2026-10-22

## Context

Le wiki du lab contient 20 lignes de réglages historiques pour la ShopBot. Le modèle théorique, calé pour rester prudent, donne des valeurs plus faibles. Il faut une règle déterministe, explicable, qui ne généralise pas un réglage hors de son contexte.

## Decision

1. Un préréglage s'applique seulement si machine, matière (ou alias), fraise et opération correspondent exactement, et si le pas latéral reste dans son périmètre.
2. Mode identique : valeurs reprises telles quelles. Prudent demandé, seul Normal disponible : profondeur réduite par le facteur prudent, origine « Adapté », fiabilité Théorique. Rapide demandé : réglage Normal conservé et signalé.
3. Plusieurs candidats : fiabilité la plus haute, puis identifiant le plus petit.
4. Sinon : modèle théorique.
5. Les valeurs saisies par l'utilisateur priment toujours et ne sont jamais corrigées.

Les réglages du wiki sont classés `reference_data` (non datés, contexte incomplet), leur plongée n'est pas reprise (calculée, non testée).

## Alternatives considered

- Correspondance par famille de fraise ou plage de diamètres : risque de généraliser un essai à un outil différent.
- Mélanger préréglage et modèle (moyenne) : non explicable.

## Consequences

Les usagers obtiennent les réglages éprouvés quand ils existent, et le modèle reste un filet de sécurité prudent. Le modèle est affiché à côté pour comparaison.

## Validation

ENG-01, ENG-05 à ENG-09 ; CAL-01 vérifie que le modèle reste sous les réglages du lab.
