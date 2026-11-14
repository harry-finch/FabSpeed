---
id: A-007
type: decider
titre: "Choisir le nouveau nettoyage de FabSpeed"
statut: fait
cree: 2026-11-14
par: agent
fait: 2026-11-14
---

Inventaire du 14/11/2026 après reconstruction. Branche fabboard/nettoyage-2026-11-14, issue de main 4c914969. Taille initiale 1 251 606 528 octets. Rien retiré. Accord séparé demandé pour A et B. A — reconstructible : target/ 1 112 309 760 octets (recompilation Rust complète), ui/node_modules/ 108 163 072 octets (npm ci --prefix ui), ui/dist/ 802 816 octets (npm --prefix ui run build avant Cargo), crates/fabspeed-desktop/gen/ 311 296 octets (schémas régénérés par Tauri). Total A : 1 221 586 944 octets, environ 1,22 Go. B — résidus Finder : .DS_Store et branding/.DS_Store, 8 192 octets chacun, total 16 384 octets ; déjà ignorés par Git. Total A+B : 1 221 603 328 octets. C : aucun fichier suivi établi comme inutile. Conserver branding/fabspeed-logo-v3.svg (non suivi, travail utilisateur), toutes les sources graphiques, données, preuves, fixtures, licences, skills et skills-lock.json. Conserver le DMG le plus récent et l installeur Windows, conformément à la décision A-005 ; aucune nouvelle décision D. Retrait autorisé uniquement via fabboard clean trash, récupérable depuis la Corbeille. Répondre A oui/non et B oui/non. La reconstruction longue sera proposée après retrait ; le choix précédent A-006 était de reporter au prochain développement.

## Réponse

Accord explicite : « je valide les deux ». Les quatre dossiers reconstructibles de A et les deux fichiers Finder de B ont été mis à la Corbeille avec fabboard clean trash. Environ 1,22 Go retiré du projet, passé de 1,25 Go à environ 30 Mo (28,6 Mio affichés par FabBoard). Aucun fichier source retiré ; logo non suivi et installeurs macOS/Windows conservés. Reconstruction et tests reportés au prochain développement selon la décision A-006, conservée. Fichiers récupérables dans la Corbeille tant que celle-ci reste intacte. Branche fabboard/nettoyage-2026-11-14, sans fusion ni push.
