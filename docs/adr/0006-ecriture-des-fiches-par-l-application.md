# ADR-0006 : Écriture des fiches par l'application dans `data/lab/`

Status: Accepted
Date: 2026-10-22

## Context

Les essais, les nouvelles machines et les nouvelles fraises doivent pouvoir être créés depuis l'interface, partagés par tous les postes du lab et rester lisibles et versionnables (ADR-0002). La v0.1 gardait les fraises personnelles dans le navigateur.

## Decision

- Une **couche lab** écrite par le serveur : `data/lab/{trials,machines,tools}/`, **un fichier TOML par fiche**, nommé d'après l'identifiant.
- Une fiche du lab de même identifiant qu'une fiche de base la **remplace** sans modifier le fichier de base (`layers.overridden`) ; archiver la version du lab rétablit l'originale. Une fiche de base ne se supprime pas depuis l'application.
- Écriture **atomique** (fichier temporaire puis renommage), une écriture à la fois (verrou), puis **rechargement complet** et validation ; si le catalogue devient invalide, le fichier précédent est restauré (ou le nouveau supprimé) et l'API répond 422.
- Suppression = **archivage** dans `data/lab/archive/<type>/` avec horodatage.
- Identifiants attribués par le serveur : `lab/<nom>` (unique), `essai-AAAAMMJJ-HHMMSS-xxxx`.
- La révision affichée inclut une empreinte de la couche lab (`…+lab.290c93`).
- Les fraises personnelles ne sont plus stockées dans le navigateur.

## Alternatives considered

- SQLite : plus robuste pour de gros volumes, mais opaque dans Git et à la relecture ; inutile pour quelques centaines de fiches.
- Un seul fichier par type, réécrit à chaque ajout : perte des commentaires, conflits Git plus fréquents.
- Stockage navigateur : non partagé entre postes, perdu au nettoyage du navigateur.

## Consequences

Le dossier `data/` doit être accessible en écriture par le serveur et sauvegardé (ou versionné). Les fiches restent modifiables à la main (puis `POST /api/v1/reload`). Pas de gestion d'accès : toute personne qui atteint le serveur peut écrire ; à revoir si le serveur est exposé hors du réseau du lab. *(Complété par l'ADR-0008 : écritures réservées à l'administrateur.)*

## Validation

LAB-01 à LAB-05 (écriture réelle dans une copie temporaire de `data/`), LAB-U1.
