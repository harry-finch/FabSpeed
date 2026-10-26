<p align="center"><img src="branding/fabspeed-logo.svg" alt="FabSpeed" width="96" height="96"></p>

# FabSpeed

**Calculateur de vitesses de coupe pour fraiseuses CNC de fablab.** [English](README.md)

Choisissez la machine, la matière, la fraise, l’opération et le maintien de la pièce. FabSpeed propose rotation, avance, plongée et profondeur de passe, en expliquant leur calcul et leur provenance avant de les reporter dans votre FAO.

**Aperçu du code source — version 0.9.0-beta.1.** Ce dépôt possède un nouvel historique indépendant. Ce n’est pas une version stable FabSpeed V1 et aucun installeur ni binaire de release n’y est publié. Voir [l’état des plateformes et les prérequis](docs/SOURCE_PUBLICATION.md).

![Calcul, coupe à l’échelle et assistant](docs/images/calculate.jpg)

## Fonctionnalités

- Moteur Rust déterministe : limites machine, contexte d’opération, charges estimées, explications et valeurs imposées explicites.
- Coupe à l’échelle, jauge de copeau, leviers d’accélération et diagnostic par symptômes.
- Bibliothèque locale de machines, matières et fraises ; import/export additif, sauvegarde et restauration.
- Essais d’usinage enregistrés ; un essai réussi peut devenir le réglage testé de son contexte exact. Un administrateur peut le valider ou retirer sa validation. Les fixtures synthétiques et tests logiciels ne constituent aucune validation physique.
- Rôles invité et administrateur ; français et anglais ; thèmes sombre et clair ; millimètres et pouces.
- Trois profils machine, quinze matières et onze géométries de fraise génériques. Aucun préréglage de lab préchargé. Les limites machine non confirmées restent marquées « estimées » ou « à vérifier ».

![Bibliothèque](docs/images/library.jpg)

## Catalogue FabApps

FabSpeed conserve son moteur et son API locale. Dans **Bibliothèque → Données du poste → Catalogue FabApps**, l’administrateur approuve explicitement les ressources à publier. Le partage est désactivé par défaut. La publication locale a été exercée sur macOS arm64 ; **elle est indisponible sur Windows**, et non qualifiée nativement sur Linux. La consommation par FabCAM est prévue séparément : aucune synchronisation opérationnelle n’est annoncée. Voir le [guide de partage](docs/FABAPPS_CATALOGUE_USER_GUIDE.md) et le [contrat](docs/fabapps/catalogue/v1/README.md).

## Construire depuis les sources

Prérequis : **Rust 1.88** (fixé), **Node.js 24** avec npm et une chaîne de compilation native. Le workspace/bureau demande aussi les [dépendances natives Tauri](docs/SOURCE_PUBLICATION.md#build-prerequisites). Python 3.12+ et `jsonschema==4.23.0` servent aux contrôles indépendants du catalogue.

```sh
npm ci --prefix ui
npm --prefix ui run build
cargo run -p fabspeed-server --locked
```

Ouvrir <http://127.0.0.1:3030>. Le serveur utilise par défaut le dossier inscriptible `data/` ; les fiches créées restent dans `data/lab/`, ignoré par Git. Passer `-- --data <votre-copie>` pour employer une base séparée. Sauvegarder depuis **Bibliothèque → Données du poste**. Garder le serveur sur localhost ou un réseau local de confiance : son transport HTTP n’est pas chiffré.

Pour un binaire serveur avec interface intégrée, construire d’abord l’interface, puis :

```sh
cargo build --release -p fabspeed-server --locked
```

Lancer `target/release/fabspeed` avec un dossier de données inscriptible. Pour développer l’interface, démarrer le serveur puis `npm --prefix ui run dev`.

Le bureau Tauri embarque ce même serveur et cette interface. Compiler le workspace sans empaqueter ni lancer d’installeur :

```sh
cargo build --workspace --locked
```

L’acceptation bureau couvre actuellement **l’évaluation interne macOS arm64 uniquement**. macOS Intel, Windows et Linux natifs, la signature de distribution et la notarisation macOS restent à qualifier. Une compilation réussie ne vaut pas qualification de release native.

## Vérifier

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm --prefix ui run check
```

Les deux fonctions de calibrage privé retournent sans exercer leur corpus lorsqu’il est absent : résultat **non exécuté**, jamais « calibrage réussi ». Les tests moteur publics utilisent des fixtures synthétiques. La [validation](docs/VALIDATION.md) décrit les contrôles JSON Schema et le rejeu des octets publiés.

## Documentation et contributions

- [Guide utilisateur](docs/USER_GUIDE.md), [modèle de calcul](docs/CALCULATION_MODEL.md), [données](docs/DATA.md).
- [Architecture](docs/ARCHITECTURE.md), [API REST](docs/API.md), [décisions](docs/adr).
- [Publication source, limites et procédure de mise à jour](docs/SOURCE_PUBLICATION.md).

Signaler les problèmes reproductibles dans les [issues](https://github.com/harry-finch/FabSpeed/issues) ; les propositions passent par des pull requests. Indiquer la révision, la plateforme et un petit exemple synthétique. Ne joindre ni mot de passe, ni base d’atelier privée, ni document fournisseur sans droit de redistribution. Les changements numériques demandent une revue et une qualification dédiées.

## Sécurité et licence

FabSpeed propose des valeurs de départ prudentes, sans certification atelier. Faire un essai accompagné sur une chute et suivre les consignes de la machine. Usiner les métaux avec une supervision adaptée. Les valeurs saisies ne sont jamais corrigées silencieusement.

Copyright © 2026 Stéphane Muller. Code et données livrées : **[GPL-3.0-or-later](LICENSE)**. Les droits tiers sont conservés ; voir les [licences et attributions](THIRD_PARTY_NOTICES.md). Archives de lab, fiches fournisseur, essais réels et profils d’application privés sont exclus.
