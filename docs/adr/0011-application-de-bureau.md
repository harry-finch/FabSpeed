# ADR-0011 : Application de bureau (Tauri) et base par poste

Status: Accepted
Date: 2026-10-22

## Context

La v1.0 vise une application installable sur chaque poste du lab, sans serveur à lancer ni navigateur, avec sa propre base d'essais (ADR-0008 pour les droits). L'interface est une application web servie par `fabspeed-server` ; FabCAM utilise Tauri 2 avec Rust 1.88. Les données livrées (machines, matières, fraises) évoluent avec les versions, la base du lab (essais, fiches ajoutées) appartient au poste.

## Decision

- **Crate `fabspeed-desktop`** (Tauri 2, même pile et même toolchain que FabCAM) : au démarrage, installe ou met à jour les données, lance `fabspeed-server` **dans le processus** sur `127.0.0.1`, puis ouvre une fenêtre sur cette adresse. Aucune commande Tauri (IPC) : l'interface reste identique à la version navigateur et l'API reste disponible pour FabCAM sur le même poste.
- **Port local fixe (47830)**, un autre seulement s'il est occupé : l'origine de la page ne change pas, donc la langue, le thème et la configuration mémorisés par la vue web sont conservés.
- **Une base par poste** dans le dossier de données de l'application (`app_data_dir/data`). Les données de base sont livrées comme ressources du paquet (`base-data/`) et installées par `fabspeed_server::install` : copie au premier lancement ; nouvelle révision de `dataset.toml` → copie de sécurité complète dans `backups/`, puis remplacement des seuls fichiers de base. `lab/`, `auth.toml`, `policy.toml` et `backups/` ne sont jamais touchés.
- **`fabspeed-server` devient aussi une bibliothèque** (`app`, `serve`, `install`), le binaire `fabspeed` reste pour l'usage en navigateur ou en réseau.
- **Téléchargements** (sauvegardes, CSV) de la vue web enregistrés dans le dossier Téléchargements.
- **Rust 1.88** fixé (`rust-toolchain.toml`) et résolveur Cargo 3 : les dépendances (dont Tauri 2.11) restent compatibles.
- **Builds** par GitHub Actions sur tag `v*` : macOS Apple Silicon et Intel (`.dmg`, signature ad hoc), Windows (`.exe` NSIS, `.msi`), Linux (`.deb`, `.AppImage`). Artefacts du workflow, pas de publication automatique ; pas de signature d'éditeur en v0.9-beta.

## Alternatives considered

- Commandes Tauri à la place de l'API HTTP : réécriture du client de l'interface, deux chemins à maintenir, et plus d'API locale pour FabCAM.
- Port aléatoire : la mémoire de la vue web (langue, thème, réglage en cours, présentation déjà vue) serait perdue à chaque lancement.
- Données de base lues directement dans le paquet : impossible de mettre à jour sans réinstaller, et la base du lab doit de toute façon vivre dans un dossier inscriptible.

## Consequences

Une mise à jour des données de base peut rendre une fiche du lab incohérente (ex. matière renommée) : le chargement le signale ; la copie de sécurité permet de revenir. Les installeurs non signés demandent une confirmation à l'utilisateur (Gatekeeper, SmartScreen). Les données livrées incluent des fiches privées du lab (CNCFraises) : les artefacts restent réservés aux collaborateurs du dépôt privé.

## Validation

INST-01 (premier lancement, mise à jour sans toucher au lab ni au mot de passe, copie de sécurité) ; build macOS local (`.app`, `.dmg`) lancé et vérifié ; workflow GitHub sur les trois systèmes.

> Mise à jour (2026-10-23, ADR-0012) : les données livrées ne contiennent plus de fiches privées ; les installeurs n'embarquent que des données publiques.

## Précision de consolidation — 26 octobre 2026

Le workflow courant ne produit que NSIS (`.exe`) sur Windows : le correctif `a160e0f` désactive MSI pour la préversion `0.9.0-beta.1`. Aucun installateur n’a été construit dans FS-V1-FINAL-1.

## État du dépôt source public — 26 octobre 2026

Les workflows historiques de fabrication/upload d’installeurs ne sont pas inclus
dans cette publication source. Le seul workflow conservé est manuel, sans
publication d’artefacts. Aucun tag ni release binaire n’est créé. Voir
[l’état courant](../SOURCE_PUBLICATION.md) ; les paragraphes précédents décrivent
les décisions historiques et ne promettent pas une qualification multiplateforme.
