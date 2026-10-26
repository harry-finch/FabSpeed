# ADR-0009 : Niveau « Validé »

Status: Accepted
Date: 2026-10-22

## Context

Un essai « Bon » devient le réglage testé de son contexte ; à contexte égal, le plus récent l'emporte (ADR-0005). Le lab veut distinguer un réglage qui a fait ses preuves, confirmé par un fabmanager, d'un essai réussi isolé, et éviter qu'un essai réussi plus récent mais moins éprouvé le remplace silencieusement. Le vocabulaire de preuve repris de FabCAM prévoit déjà `validated`.

## Decision

- **Validation portée par l'essai** : champ facultatif `validation { validated_at, validated_by, note }` dans le fichier TOML de l'essai. Pas de fichier ni d'entité à part : l'essai garde ses valeurs, la validation s'ajoute ou se retire.
- **Réservée à l'administrateur** (`POST` / `DELETE /trials/{id}/validation`, extracteur `Admin`), et seulement pour un essai « Bon ».
- **Précédence** : un essai validé produit un réglage de preuve `validated`, qui passe avant les essais `tested` grâce au tri existant par preuve ; entre deux réglages validés, la date de **validation** départage (le dernier validé fait référence).
- **Signalement** : si un essai réussi plus récent, non validé, existe dans le même contexte et le même mode, un avertissement `info` (`newer_tested_trial`) le dit ; l'administrateur décide de le valider.
- **Rappel avant validation** : l'interface affiche tous les essais du contexte et signale un essai plus récent « À ajuster » ou « Échec ».
- **Affichage** : « Validé » en vert (icône bouclier), distinct de « Testé » (cyan, flacon).

## Alternatives considered

- Critères de validation automatiques (n essais réussis, aucun échec plus récent) : séduisant, mais les critères dépendent de la pièce et du fabmanager ; la décision reste humaine, le rappel des essais l'éclaire.
- Retirer automatiquement les autres validations du contexte : plusieurs écritures non atomiques ; trier par date de validation donne le même résultat sans toucher aux autres fiches.
- Laisser le plus récent l'emporter même sur un réglage validé : contraire au sens de la validation.

## Consequences

Un réglage validé peut masquer un essai réussi plus récent : l'avertissement `newer_tested_trial` évite que cela passe inaperçu. Plusieurs essais validés peuvent coexister dans un contexte (historique des validations) ; seul le dernier validé sert de référence.

## Validation

TRI-09, TRI-10, LAB-08 ([VALIDATION.md](../VALIDATION.md)) ; parcours dans le navigateur sur une copie des données : validation demandant le mot de passe, rappel des essais, valeurs « Validé », fiche de l'essai, retrait de la validation.
