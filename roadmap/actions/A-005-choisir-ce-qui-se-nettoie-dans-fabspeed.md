---
id: A-005
type: decider
titre: "Choisir ce qui se nettoie dans FabSpeed"
statut: fait
cree: 2026-11-14
par: agent
fait: 2026-11-14
---

Inventaire du 14/11/2026 sur fabboard/nettoyage ; dépôt initialement propre, taille 4 158 369 792 octets. Rien retiré. Donner un accord distinct pour A et B. A — reconstructible : target/ (4 027 580 416 octets, recompilation Rust complète au prochain build), ui/node_modules/ (108 163 072 octets, npm ci --prefix ui), ui/dist/ (794 624 octets, npm --prefix ui run build avant les contrôles Cargo), crates/fabspeed-desktop/gen/ (311 296 octets, schémas générés par Tauri). Total A : 4 136 849 408 octets. B — résidus Finder : .DS_Store, crates/.DS_Store, docs/.DS_Store et ui/.DS_Store (8 192 octets chacun, total 32 768). Ils peuvent revenir et sont déjà ignorés. C — aucun fichier suivi suffisamment établi comme inutile. D — dist-desktop/FabSpeed_0.9.0-beta.1_x64-setup.exe (4 710 400 octets alloués) : utilité de test Windows incertaine ; recommandation conserver, répondre garder ou mettre à la Corbeille. Conserver le dernier DMG de test, les sources graphiques, les preuves et fixtures, les skills et skills-lock.json. Tout retrait A/B autorisé passera par fabboard clean trash. Potentiel A+B : 4 136 882 176 octets (environ 4,14 Go). Après retrait de A, demander si la reconstruction et les contrôles doivent être lancés maintenant, car ils recréeront les dossiers nettoyés.

## Réponse

Accord explicite du 14/11/2026 : « ok pour A et B, on garde D ». Les quatre dossiers de A et les quatre .DS_Store de B ont été mis à la Corbeille avec fabboard clean trash. Installeur Windows et dernier DMG conservés. Taille mesurée après : 20,5 Mio (contre environ 3,9 Gio avant). Aucun fichier source retiré. Reconstruction et contrôles en attente du choix suivant.
