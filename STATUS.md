# État du projet

**Version des sources : 0.9.0-beta.1**, selon `Cargo.toml` et `ui/package.json`. État du 13 novembre 2026. Les trois lots reconstitués depuis Git ont été acceptés par l'utilisateur le 13/11/2026 : [socle](roadmap/items/R-001-socle-0-9-0-beta-1.md), [préparation Windows](roadmap/items/R-002-preparation-de-l-installeur-windows.md), [illustrations](roadmap/items/R-003-illustrations-et-schema-de-coupe.md). Aucun jalon restant n'est défini ; voir [ROADMAP.md](ROADMAP.md).

| Sujet | État |
| --- | --- |
| Sources | Moteur Rust, serveur axum, interface Svelte et bureau Tauri présents ; frontières décrites dans [l'architecture](docs/ARCHITECTURE.md). |
| Illustration | Figure `endmill` déclarée pour la carte FabBoard, également intégrée à l'accueil et à l'aperçu des fraises ; voir [les illustrations](branding/illustrations/README.md). |
| Tests | Formatage, Clippy strict, tests et build du workspace Rust réussis. 130 tests et 1 doctest exercés ; 2 calibrages privés non exécutés (corpus absent). Svelte : 0 erreur, 0 avertissement ; build web réussi. |
| Vérifié | Lecture des manifestes, commandes de développement et historique Git ; contrôles ci-dessus exécutés sur ce poste le 13/11/2026. |
| Non vérifié | Parcours manuel de cette révision et essais physiques non exécutés dans cette session ; contrôles Python indépendants du catalogue non relancés. L'acceptation des lots ne lève pas les limites de qualification des plateformes. |

## Points ouverts

- La documentation existante limite l'acceptation bureau à l'évaluation interne macOS arm64 ; macOS Intel, Windows, Linux, signature et notarisation restent à qualifier. Voir [l'état des plateformes](docs/SOURCE_PUBLICATION.md).
- Publication FabApps refusée sur Windows, non qualifiée nativement sur Linux ; consommation FabCAM prévue séparément. Voir [le guide de partage](docs/FABAPPS_CATALOGUE_USER_GUIDE.md).
- Le corpus privé de calibrage est facultatif : son absence signifie « non exécuté », jamais une réussite de calibrage.
- Le build web signale un avertissement CSS existant sur `.editor .warn :global(svg)` ; la compilation aboutit.

## À suivre

Les validations de la reconstitution, du remplacement de la skill et de la documentation sont enregistrées dans les actions A-001 à A-003. Aucune nouvelle priorité n'a été donnée. La validation finale de mise d'équerre figure dans [ROADMAP.md](ROADMAP.md), action A-004.
