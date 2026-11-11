# Illustrations

Figures au trait de FabSpeed, dessinées avec la skill `hairline-create` sur le moteur de [Hairline](https://github.com/lucasmarkes/hairline) (MIT). Les skills ne sont pas versionnées ici (`.agents/` est ignoré) ; pour retoucher une figure, installer celles de Hairline à la racine : `npx skills add lucasmarkes/hairline -s hairline-create hairline-adopt`.

| Figure | Concept | Emplacements | Date |
| --- | --- | --- | --- |
| `endmill` | Une fraise couchée, pointe vers le lecteur. Le pointeur la fait rouler sur son axe : les goujures défilent, une arête est vive. Elle prend aussi la forme d'une fiche (`shape`) : type, hélice, dents, diamètre, longueur utile, queue, longueur totale, angle de pointe, partie upcut. Proportions de la fiche, cadrées dans la même boîte ; queue en pointillé quand la queue ou la longueur totale manquent. | Premier lancement (accueil) ; bibliothèque, fiche d'une fraise en création (aperçu) | 11/11/2026 |

## Fichiers

- `endmill.js` : la figure (source). Sa copie exacte est servie par l'application depuis `ui/public/illustrations/`, à côté de `kernel.js`, copie exacte du moteur de la skill.
- `hairline-endmill.html` : page d'essai autonome, à ouvrir depuis le disque (`?play=1` lance le parcours).
- Les planches de contrôle (`*.png`) ne sont pas versionnées.

## Retoucher une figure

Depuis ce dossier, modifier `endmill.js`, puis :

```bash
node ../../.agents/skills/hairline-create/look.mjs endmill.js --answer 320,166 --edge 20,166 --edge 380,166
cp endmill.js ../../ui/public/illustrations/
```

Lire la planche `hairline-endmill-look.png` et répondre aux questions de `look.md` avant de recopier. La planche ne montre que la fraise par défaut : vérifier les autres formes dans l'application (bibliothèque → Fraises → Nouvelle fraise).
