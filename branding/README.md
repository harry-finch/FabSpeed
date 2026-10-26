# Identité visuelle

`fabspeed-logo.svg` est la source vectorielle du logo **V2**, reconstruite le 25/10/2026 depuis `fabspeed-logo-v2.png` (référence conservée). Arcs et pivot magenta `#EE46FC`, tuile bleu graphite `#0C1928`, bordure `#496281`. ViewBox 1254 × 1254, taille intrinsèque 128 × 128, sans police ni image embarquée. La géométrie et les couleurs reprennent le PNG ; le grain du raster est retiré et l'extérieur de la tuile est transparent.

Utilisations :

- `README.md` et `README.fr.md` : source canonique ;
- `ui/public/fabspeed-logo.svg` : barre d'application et accueil ;
- `ui/public/favicon.svg` : onglet du navigateur ;
- `ui/public/favicon.ico` (16, 32, 48 px), `favicon-32.png`, `apple-touch-icon.png` (180 px) : variantes raster pour les navigateurs ;
- `crates/fabspeed-desktop/icons/` : icônes macOS, Windows et Linux (PNG, ICO, ICNS) ;
- couleur d'identité (`--identity` dans `ui/src/app.css`) : `#EE46FC`, réservée au logo et au liseré supérieur, jamais à un état.

Les images de l'interface et les favicons sont versionnés (`?v=2`) pour renouveler les caches. Les captures historiques dans `docs/images/` ne sont pas retouchées.

Pour synchroniser les copies SVG et régénérer toutes les icônes depuis la source :

```bash
bash scripts/generate_branding.sh
```

Prérequis : Node.js/npm et ImageMagick (`magick`). Le script utilise Tauri CLI 2.12.3 (rendu SVG par resvg), dans un dossier temporaire, et ne conserve que les icônes des plateformes de bureau.

Hors ligne, définir `TAURI_CLI` avec le chemin d’un `tauri.js` 2.12.3 déjà installé pour éviter la résolution npm.
