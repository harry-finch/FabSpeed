# ADR-0013 : Source FabApps et validation liée

Status: Accepted
Date: 2026-10-26

## Context

La projection publique doit préserver les identités existantes, empêcher une validation historique de qualifier silencieusement des dépendances modifiées et fonctionner sans publier les données privées du catalogue interne.

## Decision

KEEP le moteur, TOML, les identités et les réservations FS-COMPAT-1. EXTEND `Validation` par une liaison optionnelle capturée lors de la confirmation admin sous le verrou existant. ADD un contrôle de source séparé des sauvegardes et une projection pure dans le serveur, sans IO dans le cœur. Source aléatoire 128 bits, génération zéro jusqu'à FS-COMPAT-3. Consentement explicite par IDs, désactivé par défaut.

La projection allow-listée et le [contrat normatif](../fabapps/catalogue/v1/README.md) utilisent SHA-256 sur FAB-C14N-1 (encodage typé exact des binary64) plutôt que le formatage décimal JSON. Dates/génération de publication sont assemblées séparément. Le flux futur reste catalogue FabSpeed → cache FabCAM ; aucun moteur embarqué ni flux de projet retour.

## Alternatives considered

Exposer `Catalog` ou les sauvegardes divulguerait les notes et données internes. Utiliser FNV/dataset ou `fabcam_identity` ne suit pas les champs partagés. Reconstituer les anciennes révisions depuis les fichiers actuels fabriquerait une preuve. Une seconde identité par ressource ou une base partagée ne sont pas nécessaires. La canonisation typée évite une nouvelle dépendance JCS et les divergences de rendu décimal entre lecteurs.

## Consequences

Les anciennes validations restent historiques et utilisables par le moteur, mais non exportables sans reconfirmation. Les nouvelles liaisons ne sont pas compatibles avec les anciens lecteurs `deny_unknown_fields` : pas de downgrade promis. Une copie complète ne peut pas être détectée entre hôtes ; clone et reprise imposent une procédure explicite. Les fonctions de projection ne sont pas un mécanisme d'authentification ou de publication. Les permissions, la persistance du consentement, les réservations de génération et le remplacement atomique appartiennent à FS-COMPAT-3.

## Validation

Tests synthétiques de source/restart/move/clone, backup v1/v2, liaison et révocation, concurrence, confidentialité, schéma, vecteurs et lecteur Python indépendant. Qualification native macOS ; Windows/Linux restent à exécuter. Aucune validation physique revendiquée.
