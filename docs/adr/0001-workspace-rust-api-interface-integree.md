# ADR-0001 : Workspace Rust, API REST et interface intégrée

Status: Accepted
Date: 2026-10-22

## Context

FabSpeed doit être un outil autonome utilisable à l'atelier par des hobbyistes, et exposer une API à laquelle FabCAM (Rust + Svelte/Tauri) puisse se connecter. Le moteur doit rester réutilisable sans réseau.

## Decision

- Workspace Cargo à deux crates : `fabspeed-core` (moteur pur, sans IO) et `fabspeed-server` (binaire `fabspeed` : axum, API `/api/v1`, interface intégrée par rust-embed).
- Interface en Svelte 5 + Vite + TypeScript + `@lucide/svelte`, même pile que FabCAM, compilée dans `ui/dist` puis intégrée au binaire.
- Le calcul se fait côté serveur ; l'interface appelle l'API (recalcul différé de 90 ms).

## Alternatives considered

- Page HTML unique avec Tailwind et Alpine par CDN (proposée dans la conversation de conception) : plus rapide à écrire, mais dépend d'Internet à l'atelier, et ne partage ni composants ni conventions avec FabCAM.
- Application Tauri : installation par poste, alors qu'un serveur sur le réseau du lab sert tous les postes et FabCAM.
- Moteur compilé en WebAssembly dans le navigateur : intéressant hors ligne, mais duplique le catalogue côté client ; reste possible plus tard grâce à `fabspeed-core` sans IO.

## Consequences

Un seul binaire à déployer ; l'interface fonctionne sans Internet. Node n'est nécessaire que pour construire l'interface. FabCAM peut appeler l'API ou importer `fabspeed-core`.

## Validation

Tests API (`api.rs`), vérification visuelle dans le navigateur ; à revoir si FabCAM préfère l'import direct ou un mode hors ligne.
