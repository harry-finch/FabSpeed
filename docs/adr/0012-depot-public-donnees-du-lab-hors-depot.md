# ADR-0012 : Dépôt public, données du lab hors dépôt

Status: Accepted
Date: 2026-10-23

## Context

FabSpeed doit pouvoir être publié sous licence libre (GPL-3.0-or-later, ADR-0011). Le dépôt contenait des données du fablab sans droit de redistribution : fiches CNCFraises (`data/tools/cncfraises-fablab.toml`), archive du wiki et bibliothèque FabCAM (`docs/research/fablab-wiki/`), essais du lab (`data/lab/`), échanges de recherche. Les tests, la configuration par défaut de l'interface et la documentation du calibrage en dépendaient, et les installeurs embarquaient ces fiches.

## Decision

- **Données publiques seulement dans le dépôt** : profils machine, matières, symptômes, traductions, et un jeu de **11 fraises génériques** (`data/tools/generic.toml`, identifiants `generic/…`, géométries courantes sans données fournisseur). Licence des données livrées : celle du code.
- **Données d'un lab ignorées par Git** : `data/lab/`, `data/tools/cncfraises-*.toml`, `data/tools/*.private.toml`, `data/presets/*.toml`, `data/policy.toml`, `docs/research/`. Elles restent sur les postes ; le chargeur les lit comme avant.
- **Tests autonomes** : données publiques et réglages de référence **fictifs** (`crates/fabspeed-core/tests/fixtures/presets.toml`), bibliothèque FabCAM d'exemple (`docs/examples/`). Le calibrage CAL-01/02 lit l'archive privée à l'exécution si elle est présente, et se saute sinon. Les tests qui lisent `data/` ne supposent plus un nombre fixe de fraises.
- **Documentation sans valeurs privées** : le tableau détaillé du calibrage (valeurs du wiki) est déplacé dans l'archive privée ; la documentation publique n'en garde que l'exigence et la plage des écarts.
- **Fraises du commerce d'un lab** : importées dans l'application (**Importer FabCAM**), elles rejoignent `data/lab/tools/` en gardant leur identité FabCAM, donc le lien avec les essais déjà notés.

## Consequences

- Un clone public compile, passe les tests et produit des installeurs sans aucune donnée privée.
- Les installeurs n'embarquent plus les fiches CNCFraises. Une mise à jour remplace le dossier `tools/` livré : sur un poste du lab, réimporter les fraises CNCFraises avant (ou après) l'installation d'une version publique, sinon les essais qui les citent sont ignorés jusqu'à l'import.
- **L'historique Git contient toujours ces données** : rendre ce dépôt public tel quel les publierait. Il faut soit réécrire l'historique (retirer ces chemins de tous les commits), soit publier un nouveau dépôt à partir de l'arbre actuel.
- `.gitignore` ne protège que les fichiers non suivis : toute nouvelle donnée de lab doit être placée sous un chemin ignoré.

## Disposition de publication source — 26 octobre 2026

Le mainteneur, qui se déclare titulaire des droits, a confirmé la redistribution
sous GPL-3.0-or-later des constantes dérivées, valeurs machine et consignes
traduites présentes dans le jeu livré. Cette autorisation ne publie pas le corpus
historique : archives originales, fiches fournisseur, essais réels et profils
privés restent exclus. Les valeurs du moteur et les références de tests qualifiées
ne sont pas modifiées. L’historique privé n’est pas transféré dans ce dépôt.

La qualification technique ne prouve pas les droits ; cette disposition repose
sur la déclaration explicite du titulaire. Aucune validation physique nouvelle.
