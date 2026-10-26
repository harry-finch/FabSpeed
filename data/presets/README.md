# Réglages de référence importés (optionnel)

FabSpeed part du **calcul** : ce dossier est vide par défaut. La base de connaissances du lab se construit avec les essais notés dans l'application (`data/lab/trials/`), dont les essais « Bon » deviennent des réglages testés.

Ce dossier accueille des réglages venus d'ailleurs, au format `[[presets]]` (voir `docs/DATA.md`). Les fichiers `*.toml` de ce dossier sont des données d'un lab : ils sont ignorés par Git. Après y avoir copié des réglages, recharger (`POST /api/v1/reload`). À contexte identique, un essai du lab l'emporte sur une référence (`reference_data` < `tested`).
