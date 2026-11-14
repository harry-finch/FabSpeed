---
id: A-006
type: decider
titre: "Choisir quand reconstruire FabSpeed après nettoyage"
statut: fait
cree: 2026-11-14
par: agent
fait: 2026-11-14
---

A et B ont été mis à la Corbeille le 14/11/2026 avec accord utilisateur ; D est conservé. Projet ramené à environ 20,5 Mio. Aucun fichier source retiré. Choisir : reconstruire et vérifier maintenant, ou reporter au prochain développement. Une reconstruction maintenant recréera les dossiers nettoyés et prendra plusieurs minutes : npm ci --prefix ui, npm --prefix ui run build, cargo fmt --all --check, cargo clippy --workspace --all-targets --locked -- -D warnings, cargo test --workspace --locked, npm --prefix ui run check, cargo build --workspace --locked. Les deux calibrages privés restent non exécutés si le corpus manque. Aucun installeur ne sera lancé. Branche locale fabboard/nettoyage, aucun push ni fusion.

## Réponse

Décision explicite du 14/11/2026 : « on reporte au prochain dév ». Reconstruction et contrôles reportés au prochain développement. Avant les contrôles Cargo : npm ci --prefix ui puis npm --prefix ui run build ; relancer ensuite les vérifications du projet décrites dans AGENTS.md. Nettoyage terminé, aucun contrôle de build ou test relancé dans cette session. A et B restent à la Corbeille, installeur Windows et dernier DMG conservés.
