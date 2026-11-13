# Consignes pour les agents

> Ce fichier est repris dans CLAUDE.md.

FabSpeed propose des paramètres de coupe pour les fraiseuses CNC de fablab, avec leur provenance et leurs limites. Le projet associe un moteur Rust pur, un serveur axum, une interface Svelte 5 et une application Tauri 2.

Avant de modifier, lire [README.fr.md](README.fr.md), [STATUS.md](STATUS.md) et [l'architecture](docs/ARCHITECTURE.md). Pour les calculs, lire le [modèle](docs/CALCULATION_MODEL.md) et les [cas de validation](docs/VALIDATION.md) ; pour les échanges FabApps, lire le [contrat du catalogue](docs/fabapps/catalogue/v1/README.md).

## Skills à utiliser (`.agents/skills/`)

- `fab-roadmap` : feuille de route, spécifications, statuts et actions de l'utilisateur.
- `project-documentation` : création et mise à jour des documents, bilan de session.
- `fab-design-system` : toute modification de l'interface, des textes ou de l'identité graphique.
- `fab-illustration-system` : choix, intégration ou révision d'une illustration Fab.
- `hairline-create` : création d'une figure au trait ou retouche de la figure existante, avec ses contrôles visuels.
- `hairline-adopt` : adaptation d'un SVG en figure Hairline.

Ces skills sont propres au poste et ignorées par Git ; les installer depuis la bibliothèque FabBoard si elles manquent. Les skills Hairline s'installent selon [branding/illustrations/README.md](branding/illustrations/README.md).

## Feuille de route

La feuille de route et les actions de l'utilisateur vivent dans `roadmap/`, lu par FabBoard. `ROADMAP.md` est généré : ne jamais le modifier à la main. Appliquer `fab-roadmap` pour spécifier, tenir les statuts, lire les réponses et créer une action lorsqu'on attend l'utilisateur. Exécuter `fabboard roadmap lint` avant chaque commit.

## Règles

- Garder `fabspeed-core` pur, déterministe et sans accès au disque, au réseau ou à l'interface.
- Ne jamais corriger silencieusement les valeurs imposées ; préserver la distinction entre origine, fiabilité et intention.
- Un test logiciel ou une fixture synthétique ne valide pas un réglage d'usinage physique. Si le corpus privé manque, les deux tests de calibrage sont **non exécutés**, même si Cargo les affiche réussis.
- Utiliser des copies temporaires des données livrées pour les essais ; ne pas modifier un profil réel d'atelier. Ne pas lancer un installeur dans les contrôles source.
- Préserver l'exclusion des données privées, des mots de passe et des profils locaux. Conserver la liste explicite des ressources du paquet et son contrôle par empreintes.
- Respecter les limites de qualification des plateformes et du partage FabApps décrites dans [SOURCE_PUBLICATION.md](docs/SOURCE_PUBLICATION.md). Le consommateur FabCAM relève d'un travail distinct.
- Préserver l'historique public indépendant ; suivre la procédure de publication avant tout transfert de sources privées.
- Mettre à jour la documentation dans le même changement que le comportement. Garder les textes d'interface en français et en anglais, avec les jetons des deux thèmes.

## Commandes

Depuis la racine, avec Rust 1.88, Node.js 24 et les dépendances natives décrites dans le README :

```sh
npm ci --prefix ui
npm --prefix ui run build
cargo run -p fabspeed-server --locked
```

Le serveur écoute sur `http://127.0.0.1:3030`. Pour développer l'interface, garder le serveur actif et lancer `npm --prefix ui run dev` : `http://localhost:5173`, avec relais de `/api` vers le serveur. Passer `-- --data <copie>` au serveur pour choisir une base distincte.

Construire l'interface avant les contrôles Cargo, car le serveur intègre `ui/dist` :

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix ui run check
cargo build --workspace --locked
fabboard roadmap lint
```

Les contrôles indépendants du catalogue et des octets publiés sont décrits dans [docs/VALIDATION.md](docs/VALIDATION.md).
