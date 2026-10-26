# ADR-0008 : Rôles invité et administrateur

Status: Accepted
Date: 2026-10-22

## Context

Chaque poste du lab a sa propre base d'essais (feuille de route v1.0). Les hobbyistes doivent pouvoir calculer et consulter librement, mais seul un fabmanager doit noter des essais, modifier la bibliothèque et, plus tard, valider un réglage. L'ADR-0006 laissait toute écriture ouverte. La protection doit tenir côté serveur (FabCAM et les scripts appellent l'API directement), pas seulement dans l'interface.

## Decision

- **Deux rôles.** Invité par défaut : lecture, calcul, diagnostic, import FabCAM sans enregistrement. Administrateur : toutes les routes d'écriture (`/trials`, `/tools`, `/machines`), via l'extracteur axum `Admin` qui répond 401 `admin_required` sans session valide.
- **Un mot de passe par poste**, sans comptes nominatifs : empreinte Argon2id (sel aléatoire, paramètres par défaut de la crate `argon2`) dans `<données>/auth.toml`, écrit atomiquement, exclu de Git. 8 caractères minimum (`MIN_PASSWORD_LEN`).
- **Premier mot de passe** défini depuis l'interface tant qu'aucun n'existe (`POST /auth/setup`, refusé ensuite). Oubli : `fabspeed --reset-admin-password` (accès au poste requis), sans toucher aux données.
- **Sessions en mémoire** : jeton aléatoire de 256 bits envoyé en `Authorization: Bearer`, expiré après 8 h d'inactivité (`SESSION_IDLE`), perdu au redémarrage ; un changement de mot de passe ferme toutes les autres sessions. Un mauvais mot de passe coûte 800 ms (`FAILED_LOGIN_DELAY`).
- **Interface** : bouton Invité / Admin ; les actions d'écriture demandent le mot de passe au moment voulu (`ensureAdmin()`), puis reprennent ; jeton gardé pour l'onglet (`sessionStorage`).

## Alternatives considered

- Comptes nominatifs : utiles pour tracer qui a validé quoi, mais lourds pour un poste de fablab ; le prénom de l'opérateur reste noté sur chaque essai.
- Cookie de session : plus simple pour le navigateur, mais FabCAM (Tauri) et les scripts appellent l'API depuis une autre origine ; un jeton `Bearer` sert les deux.
- Sessions persistées sur disque : inutile pour un poste local ; un redémarrage qui déconnecte est acceptable.
- Protéger aussi la lecture : contraire à l'objectif (consultation libre).

## Consequences

Le trafic HTTP n'est pas chiffré : le mot de passe protège contre les erreurs et les usages non prévus sur le poste ou le réseau du lab, pas contre un attaquant sur un réseau ouvert. Les futures écritures (matières, sauvegarde, niveau « Validé ») utiliseront le même extracteur. Un serveur sans dossier de données ne peut pas avoir d'administrateur.

## Validation

AUTH-01…03, LAB-07 ([VALIDATION.md](../VALIDATION.md)) ; parcours dans le navigateur sur une copie des données : premier mot de passe depuis « Noter un essai », essai enregistré, retour en invité, modification refusée puis mot de passe demandé, mauvais mot de passe, session perdue au redémarrage du serveur puis reprise de l'enregistrement après reconnexion, changement de mot de passe.
