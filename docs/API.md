# API REST v1

Base : `http://<hôte>:3030/api/v1`. Corps JSON, unités en millimètres, minutes et tours par minute. Énumérations en `snake_case`. CORS ouvert pour les clients locaux existants ; ce n’est pas une autorisation de publier le catalogue privé.

## Langue

Les textes destinés aux personnes (messages d'erreur, avertissements, leviers, trace, noms et conseils des matières, symptômes, fiches de base) sont rendus en **français** (`fr`, défaut) ou en **anglais** (`en`). La langue se choisit par le paramètre `?lang=en` (prioritaire) ou l'en-tête `Accept-Language`. Les identifiants, codes (`warnings[].code`, `trace[].key`, `limits`…) et nombres ne dépendent jamais de la langue : un programme doit s'appuyer sur eux, pas sur les phrases.

Les fiches créées dans l'application (`data/lab/`) sont affichées telles que saisies. Modifier une fiche de base en anglais (`PUT` avec `lang=en`) ne remplace pas ses textes français par la traduction : un texte renvoyé inchangé reste le texte français de base.

Erreurs : statut HTTP + `{ "error": "message dans la langue demandée", "code": "…" }`.

| Code | Statut | Cause |
| --- | --- | --- |
| `unknown_machine`, `unknown_material`, `unknown_tool`, `unknown_symptom` | 404 | Identifiant absent du catalogue |
| `missing_tool` | 422 | Ni `tool_id` ni `tool` |
| `invalid_input` | 422 | Valeur hors domaine (diamètre ≤ 0, pas latéral > 100 %…) |
| `invalid_toolset` | 422 | Fichier FabCAM illisible |
| `reload_failed` | 422 | Données invalides : l'ancien catalogue reste actif |
| `invalid_data` | 422 | Fiche refusée (catalogue incohérent) : rien n'est enregistré |
| `invalid_backup` | 422 | Sauvegarde refusée : rien n'est modifié |
| `already_exists` | 409 | Création avec un identifiant déjà pris |
| `base_entry` | 409 | Suppression d'une fiche de base (à retirer à la main dans `data/`) |
| `not_found` | 404 | Fiche absente de `data/lab/` |
| `read_only` | 503 | Serveur sans dossier de données accessible en écriture |
| `admin_required` | 401 | Écriture sans session administrateur (mode invité) |
| `wrong_password` | 401 | Mauvais mot de passe administrateur |
| `password_too_short` | 422 | Mot de passe de moins de 8 caractères |
| `already_configured`, `not_configured` | 409 | Premier mot de passe déjà défini / pas encore défini |

## Rôles : invité et administrateur

Lecture et calcul sont ouverts à tous (invité) : `/health`, `/catalog`, `/recommend`, `/diagnose`, `/import/ivac-toolset` (qui n'enregistre rien), `/reload`, `/trials.csv`. Les **écritures** (`/trials`, `/tools`, `/machines`, `/materials`) et la sauvegarde / restauration (`/lab/backup`, `/lab/restore`) exigent une session administrateur : en-tête `Authorization: Bearer <jeton>`, sinon 401 `admin_required`.

| Méthode et route | Corps | Réponse |
| --- | --- | --- |
| `GET /auth` | — | `{ "configured": bool, "admin": bool }` (`admin` : le jeton envoyé est valide) |
| `POST /auth/setup` | `{ "password" }` | 201 `{ "token" }` ; seulement si aucun mot de passe n'existe (sinon 409) |
| `POST /auth/login` | `{ "password" }` | `{ "token" }` ; 401 `wrong_password` après un délai |
| `POST /auth/logout` | — | `{ "admin": false }` ; ferme la session du jeton envoyé |
| `POST /auth/password` | `{ "current", "new" }` | `{ "token" }` ; admin requis ; ferme toutes les autres sessions |

Le mot de passe (8 caractères minimum) est propre au poste : son empreinte Argon2id est dans `<données>/auth.toml`, jamais le mot de passe en clair. Les sessions vivent en mémoire : elles expirent après 8 h d'inactivité et disparaissent au redémarrage du serveur. Mot de passe oublié : `fabspeed --reset-admin-password` (ou supprimer `auth.toml`), redémarrer, puis en définir un nouveau.

## `GET /health`

```json
{ "status": "ok", "engine_version": "0.1.0", "dataset_revision": "synthetic-example-dataset",
  "machines": 3, "materials": 15, "tools": 12, "presets": 0, "trials": 2, "languages": ["fr", "en"] }
```

## `GET /catalog`

Tout le catalogue : `meta`, `machines`, `materials`, `tools`, `presets` (références importées, vide par défaut), `trials` (essais), `symptoms`, `policy`, `layers` (`lab_machines`, `lab_tools`, `lab_materials` : fiches écrites par l'application ; `overridden` : fiches de base remplacées par une version du lab), plus `engine_version` et `issues` (problèmes non bloquants). Sert à remplir les menus.

## `POST /recommend`

Requête :

```json
{
  "machine_id": "shopbot-prs-standard-96x48-2016",
  "material_id": "material/wood.plywood",
  "tool_id": "generic/flat-6-1f-up",
  "operation": "profile",
  "depth_mm": 18,
  "aggressiveness": "normal",
  "workholding": "screws_clamps"
}
```

| Champ | Type | Défaut | Description |
| --- | --- | --- | --- |
| `machine_id` | texte | requis | Profil machine |
| `material_id` | texte | requis | Matière ou alias (identifiant FabCAM accepté) |
| `tool_id` | texte | — | Fraise du catalogue ou identité FabCAM |
| `tool` | objet `ToolSpec` | — | Fraise décrite en ligne (fraise personnelle, outil FabCAM) |
| `operation` | `profile` · `pocket` · `full_slot` · `finish` · `surfacing` · `engraving` | requis | |
| `stepover_ratio` | 0–1 | matière | Pas latéral (poche, surfaçage) |
| `depth_mm` | mm | — | Profondeur totale (épaisseur pour un détourage) ; active le calcul des passes |
| `aggressiveness` | `prudent` · `normal` · `rapid` | `normal` | Intention |
| `workholding` | `screws_clamps` · `vacuum` · `tape` · `unknown` | `unknown` | Maintien de la pièce |
| `stickout_mm` | mm | longueur utile + 3 | Sortie de pince |
| `max_doc_mm` | mm | — | Plafond de profondeur par passe |
| `cut_length_mm` | mm | — | Longueur de parcours par passe, pour la durée |
| `use_presets` | booléen | `true` | `false` = modèle seul |
| `supervised_speedups` | booléen | `false` | Matière sous supervision (métaux) : proposer aussi les leviers d'accélération. Ignoré sans session administrateur (`Authorization: Bearer`) |
| `overrides` | `{ rpm, feed_mm_min, plunge_mm_min, doc_mm }` | — | Valeurs imposées, vérifiées mais jamais corrigées |

Réponse illustrative (extrait construit avec un essai synthétique et les valeurs du diagnostic du jeu public ; personne, identifiant et date fictifs) :

```json
{
  "status": "ok",
  "values": {
    "rpm": 12600, "feed_mm_min": 3420, "plunge_mm_min": 513,
    "doc_mm": 2.25, "doc_max_mm": 2.25, "passes": 8, "depth_mm": 18,
    "stepover_mm": 6, "stepover_ratio": 1, "ramp_entry_advised": false
  },
  "origins": { "rpm": "lab_preset", "feed": "lab_preset", "plunge": "lab_preset", "doc": "lab_preset" },
  "metrics": {
    "chipload_mm": 0.271, "chip_thickness_mm": 0.271,
    "chip_zone": { "min_mm": 0.02, "nominal_mm": 0.21, "max_mm": 0.525, "state": "ok" },
    "cutting_speed_m_min": 237.5, "mrr_cm3_min": 46.2, "cutting_power_w": 46,
    "available_power_w": 1540, "cutting_force_n": 11.7,
    "loads": { "spindle_pct": 3.7, "machine_pct": 6.5, "tool_pct": 8.1, "holding_pct": 2.9 },
    "stickout_mm": 28, "slenderness": 4.67, "estimated_time_s": null, "time_per_metre_s": 140.4
  },
  "limits": [],
  "warnings": [{ "code": "machine_limits_unverified", "severity": "info", "message": "…" }],
  "levers": [{
    "id": "deeper_passes", "title": "Moins de passes, plus profondes",
    "detail": "… Passez de 8 passes à 6 passes (3 mm par passe) …",
    "time_gain": 0.25, "caution": "…", "patch": { "overrides": { "doc_mm": 3.35 } }
  }],
  "trace": [{ "key": "tested_setting", "label": "Réglage testé", "formula": "Essai du 01/01/2026 · Synthetic example", "result": "…" }],
  "provenance": {
    "source": "lab_preset", "preset_id": "trial:synthetic-api-trial",
    "preset_label": "Essai du 01/01/2026 · Synthetic example",
    "preset_source": "Essai synthetic-api-trial : 12 600 tr/min, 3 420 mm/min, passe 2,25 mm",
    "evidence": "tested", "engine_version": "0.1.0", "dataset_revision": "synthetic-example-dataset", "notes": []
  },
  "theoretical": { "rpm": 14850, "feed_mm_min": 3110, "plunge_mm_min": 466, "doc_mm": 2.5 },
  "history": [{ "id": "synthetic-api-trial", "outcome": "good", "…": "…" }]
}
```

- `status: "unsupported"` : `values` et `metrics` sont `null` (fraise en V sans angle de pointe, denture diamant) ; un avertissement `blocking` explique pourquoi.
- `metrics.cutting_diameter_mm` : diamètre qui coupe réellement (largeur gravée d'une fraise en V, diamètre réduit d'une hémisphérique peu engagée ; sinon le diamètre de la fraise). `cutting_speed_m_min` est calculée à ce diamètre.
- Avertissements propres aux types de fraise : `ball_nose`, `compression_first_pass`, `tool_type_not_calibrated` (V, surfaçage), tous `info`.
- `trace[].key` : `cutting_diameter` quand le diamètre qui coupe diffère du diamètre de la fraise.
- `origins.*` : `lab_preset` (réglage testé ou référence importée, repris tel quel), `derived_from_preset`, `calculated`, `user_override`.
- `warnings[].severity` : `info`, `warning`, `blocking` (triés du plus grave au moins grave).
- `limits` : facteurs qui ont borné le calcul (`spindle_max_rpm`, `machine_feed`, `tool_strength`, `workholding`, `flute_length`…).
- `trace[].key` : code stable de l'étape (`spindle_speed`, `chip_target`, `chip_thinning`, `feed`, `depth_of_cut`, `physical_limit`, `plunge`, `tested_setting`, `actual_chip`, `cutting_power`, `cutting_force`) ; `label`, `formula` et `result` sont localisés.
- `theoretical` : présent quand un réglage testé ou importé est utilisé, pour comparer avec le modèle seul.
- `provenance.evidence` : `validated` quand le réglage repris est un essai validé par l'administrateur (passe avant les essais `tested`) ; un essai réussi plus récent non validé produit l'avertissement `newer_tested_trial` (`info`).
- `history` : essais du même contexte (machine, matière, fraise, opération ; détourage ≈ rainure), du plus récent au plus ancien. Si les valeurs proposées reprennent celles d'un essai « à ajuster » ou en échec, un avertissement `repeats_unsuccessful_trial` est émis.
- Un levier s'applique en fusionnant son `patch` dans la requête (`overrides` fusionnés, `aggressiveness`, `workholding`, `stickout_mm`).

## `POST /diagnose`

```json
{ "request": { …même objet que /recommend… }, "symptom_id": "burning" }
```

Réponse : `symptom` (libellé, cause, explication, conseils), `before` (valeurs actuelles), `after` (recommandation complète réévaluée avec la correction) et `patch` à appliquer pour l'adopter. Symptômes : voir `GET /catalog` → `symptoms` ([data/advisor.toml](../data/advisor.toml)).

## `POST /import/ivac-toolset`

Corps : contenu brut d'un fichier FabCAM `*.ivac-toolset.json`. Réponse : `{ "tools": [ToolSpec…] }`. Cette route **n'enregistre rien** : l'interface envoie ensuite chaque fraise retenue à `POST /tools`. Le sens d'hélice et la famille sont déduits du nom et signalés dans `notes`.

## Bibliothèque : import additif et export

Ces routes sont réservées à l'administrateur. Elles échangent uniquement les machines, matières et fraises, sans toucher aux essais, préréglages, validations, paramètres du moteur ou données de base.

| Méthode et route | Corps / résultat |
| --- | --- |
| `GET /library/export` | Fichier `fabspeed-library.json` : `{ "machines": [MachineProfile…], "materials": [MaterialProfile…], "tools": [ToolSpec…] }`, textes sources et provenance conservés. `?kind=machines`, `materials` ou `tools` limite le contenu à une section. |
| `POST /library/import?check=true` | `{ "content": "contenu brut du fichier", "kind": "tools" }` (`kind` facultatif) : analyse et valide le lot sans écriture. |
| `POST /library/import` | Même corps ; ajoute les nouvelles fiches dans `data/lab/`. Le lot est revérifié sous verrou avant écriture, avec annulation du lot en cas d'échec. |

Réponse commune aux deux imports : `{ "added": 3, "skipped": 1, "records": [{ "kind": "tools", "id": "…", "name": "…", "action": "add", "reason": "" }, …], "issues": [] }`. En mode vérification, `added` compte les ajouts prévus ; après import, les ajouts effectifs. `action: "skip"` indique un identifiant ou alias déjà présent, sur le poste ou plus tôt dans le fichier ; `reason` explique le conflit. La première fiche est conservée, aucune fiche n'est remplacée. Un fichier réimporté avec les mêmes identifiants n'ajoute rien.

Formats acceptés dans `content` :

- JSON FabSpeed : l'objet de l'export (les trois listes sont facultatives) ;
- TOML FabSpeed : `[[machines]]`, `[[materials]]`, `[[tools]]`, seuls ou ensemble, avec les champs décrits dans [DATA.md](DATA.md) ;
- bibliothèque FabCAM `*.ivac-toolset.json` (fraises uniquement) ;
- sauvegarde FabSpeed v1 ou v2 : vérifiée dans un dossier temporaire, puis seules les fiches actives sont extraites (les versions du lab priment dans la sauvegarde). Les archives, essais, validations et réglages ne sont pas importés.

Les identifiants doivent être renseignés et stables. Les alias de matière et les identités FabCAM des outils participent à la détection des conflits, même lorsque l'identifiant principal diffère. Aucun rapprochement par nom ni modification automatique des valeurs. Une collision de nom de fichier ou une donnée invalide dans une nouvelle fiche refuse tout le lot (422 `invalid_import`). La limite du corps HTTP est de 50 Mo. L'ancienne route `/import/ivac-toolset` reste disponible pour la conversion seule ; l'interface utilise maintenant l'import par lot.

## Écriture : essais, fraises, machines, matières

Réservé à l'administrateur (`Authorization: Bearer <jeton>`). Chaque écriture crée ou remplace **un fichier** dans `data/lab/`, puis recharge tout le catalogue. Si le catalogue devient incohérent, l'écriture est annulée (422 `invalid_data`) et rien ne reste sur le disque. Une suppression **archive** la fiche dans `data/lab/archive/`.

| Méthode et route | Corps | Réponse |
| --- | --- | --- |
| `POST /trials` | `TrialInput` | 201 `{ "trial": Trial }` |
| `DELETE /trials/{id}` | — | `{ "archived": id }` |
| `POST /trials/{id}/validation` | `{ "validated_by", "note" }` (facultatifs) | `{ "trial": Trial }` avec `validation` ; 422 si l'essai n'est pas « Bon » ; 404 si inconnu |
| `DELETE /trials/{id}/validation` | — | `{ "trial": Trial }` sans `validation` |
| `POST /tools` | `ToolSpec` (`id` vide = attribué : `lab/<référence ou nom>`) | 201 `{ "tool": ToolSpec }` |
| `PUT /tools/{id}` | `ToolSpec` | `{ "tool": ToolSpec }` ; une fraise de base devient une version du lab |
| `DELETE /tools/{id}` | — | `{ "archived": id, "restored_base": bool }` ; 409 pour une fiche de base |
| `POST /machines` | `MachineProfile` (`id` vide = attribué : `lab/<nom>`) | 201 `{ "machine": … }` |
| `PUT /machines/{id}` | `MachineProfile` | `{ "machine": … }` |
| `DELETE /machines/{id}` | — | `{ "archived": id, "restored_base": bool }` |
| `POST /materials` | `MaterialProfile` (`id` vide = attribué : `lab/<nom>`) | 201 `{ "material": … }` |
| `PUT /materials/{id}` | `MaterialProfile` | `{ "material": … }` ; une matière de base devient une version du lab |
| `DELETE /materials/{id}` | — | `{ "archived": id, "restored_base": bool }` ; 409 pour une fiche de base |

Les identifiants peuvent contenir `/` (`/api/v1/tools/generic/flat-6-1f-up`).

`TrialInput` :

```json
{
  "operator": "Synthetic example",
  "machine_id": "shopbot-prs-standard-96x48-2016",
  "material_id": "material/wood.plywood",
  "tool_id": "generic/flat-6-1f-up",
  "operation": "profile", "aggressiveness": "normal", "workholding": "screws_clamps",
  "thickness_mm": 18, "depth_mm": 18, "stepover_ratio": 1, "stickout_mm": 28, "tool_condition": "new",
  "rpm": 12600, "feed_mm_min": 3420, "plunge_mm_min": 513, "doc_mm": 2.25,
  "outcome": "good",
  "symptoms": [],
  "notes": "Bords nets, copeaux francs."
}
```

`outcome` : `good` (devient le réglage testé du contexte), `adjust`, `failed`. `tool_condition` (facultatif) : `new` ou `used`. `symptoms` : identifiants de `GET /catalog` → `symptoms`. Le serveur ajoute `id` (`essai-AAAAMMJJ-HHMMSS-xxxx`), `recorded_at` (UTC), l'instantané de la fraise (`tool_name`, `tool_diameter_mm`, `tool_flutes`), `engine_version` et `dataset_revision`.

## Données du poste : sauvegarde, restauration, export

| Méthode et route | Accès | Réponse |
| --- | --- | --- |
| `GET /lab/backup` | admin | Fichier JSON (`Content-Disposition: attachment`) : `{ "format": "fabspeed-lab-backup", "version": 2, "created_at", "engine_version", "dataset_revision", "files": [{ "path": "machines.toml", "content": "…" }, …] }` ; toutes les données du poste, chemins relatifs au dossier de données : `dataset.toml`, `machines.toml`, `materials.toml`, `advisor.toml`, `policy.toml`, `tools/`, `presets/`, `i18n/` et `lab/` (archives comprises) ; jamais `auth.toml` ni `backups/` |
| `POST /lab/restore?check=true` | admin | Corps : une sauvegarde. Vérifie sans rien modifier : `{ "check": { "created_at", "engine_version", "dataset_revision", "backup": Counts, "current": Counts, "issues": [] } }` |
| `POST /lab/restore` | admin | Même vérification, puis copie complète des données actuelles dans `data/backups/avant-restauration-<date>.json`, remplacement de chaque élément sauvegardé et rechargement (retour arrière en cas d'échec) : `{ "restored": …, "safety_backup": "backups/…json" }` |
| `GET /trials.csv` | tous | Tous les essais en CSV, du plus ancien au plus récent (`?lang=fr` : `;` et virgule décimale ; `en` : `,` et point), UTF-8 avec BOM |

`Counts` : `machines`, `materials`, `tools` (totaux), `lab_records` (fiches ajoutées ou modifiées au lab), `trials`, `validated`, `archived` ; `check.full` vaut `false` pour une sauvegarde au format 1. Une sauvegarde est refusée (422 `invalid_backup`, rien n'est modifié) si son format est inconnu, si un chemin sort des éléments ci-dessus, si un fichier est illisible, ou si le catalogue qu'elle donne est incohérent (vérifié en la chargeant entièrement dans un dossier temporaire). Taille maximale : 50 Mo.

Format 1 (première version de la sauvegarde) : base du lab seule, chemins relatifs à `data/lab/`. Toujours accepté ; il ne remplace alors que `data/lab/` et garde les données de base du poste.

## `POST /reload`

Relit `data/` (après une modification à la main). Réponse : `{ "status": "reloaded", "dataset_revision": "…", "issues": […] }`. En cas d'erreur (422), les données précédentes restent actives.

## Exemple

```bash
curl -s localhost:3030/api/v1/recommend -H 'content-type: application/json' -d '{"machine_id":"shopbot-prs-standard-96x48-2016","material_id":"material/wood.mdf","tool_id":"generic/flat-8-1f-up","operation":"pocket","stepover_ratio":0.4,"depth_mm":6}'
```

Les types TypeScript correspondants sont dans [ui/src/lib/types.ts](../ui/src/lib/types.ts).

## Identités et concurrence — FS-COMPAT-1

Les routes existantes sont conservées. Créer avec un ID actif reste un conflit ; un ID historique réservé ou un chemin appartenant à une autre fiche est refusé explicitement, sans remplacement. Les créations sans ID choisissent un slug libre parmi toutes les réservations, pas seulement les fiches actives. Les PUT requièrent un ID primaire exact et conservent celui de l'URL. Les validations d'essais sont inchangées.

`/library/import` reste additif : les IDs/alias historiquement réservés comptent maintenant parmi les conflits `skip`. La sauvegarde v2 ajoute le champ optionnel `identity_reservations` ; les anciennes sauvegardes sans ce champ restent lisibles. La restauration d'un ID déjà connu avec une version non reconnue est refusée (`invalid_backup`) pour examen. Ne pas utiliser la création/import additif pour désarchiver une identité. Les nouveaux backups ne sont pas garantis lisibles par un ancien binaire qui refuse les champs inconnus.

Le diagnostic est une commande locale `--check-identities`, pas une nouvelle API publique. Voir FS-COMPAT-1 (compte rendu interne non publié).

## FS-COMPAT-2 : validation liée, sans nouvelle route publique

Le POST administrateur existant de validation capture désormais `validation.binding` : version, source, identifiant d’essai, références/révisions machine-outil-matière et digest du contexte/valeurs. Lecture, capture et écriture partagent le mutex transactionnel. Une dépendance absente ou une donnée partagée invalide refuse la confirmation (400 `validation_binding`) sans remplacer l’essai. DELETE retire toujours la validation entière.

Les validations historiques sans liaison restent lisibles et significatives dans FabSpeed ; elles ne qualifient pas un preset partagé sans reconfirmation explicite. Les sauvegardes v1/v2 conservent les liaisons mais ne transfèrent pas le contrôle d’identité de source (les liaisons gardent leur référence de source historique). Les anciens binaires refusant les champs inconnus ne sont pas compatibles avec les nouveaux essais liés ; aucun downgrade n’est promis.

Le [contrat FabApps](fabapps/catalogue/v1/README.md) est distinct de `/catalog`, des sauvegardes et de l’échange de bibliothèque. FS-COMPAT-3/4 ajoute les routes administrateur internes décrites ci-dessous et la publication locale sur consentement. Aucune route publique de récupération du catalogue partagé ni intégration FabCAM n’est activée.


## Catalogue FabApps : administration interne (FS-COMPAT-3/4)

Toutes ces routes exigent `Authorization: Bearer <session admin>` ; un invité reçoit 401. Elles ne modifient jamais le moteur de calcul. Le CORS existant ne dispense pas d’authentification.

| Route | Fonction |
| --- | --- |
| `GET /fabapps/sharing` | `{status, choices}` : configuration persistée, phase, dernière publication, destination proposée, ressources actives et admissibilité des préréglages, présence dans le dernier fichier accessible reconnu |
| `POST /fabapps/sharing` | Enregistrer `{destination: chemin_absolu_ou_null, consent:{enabled, machines:[], tools:[], materials:[], trials:[]}}` ; IDs exacts, pas d’approbation anticipée d’un ID inconnu |
| `POST /fabapps/sharing/preview` | Même corps ; `{snapshot, excluded, preview_only:true}` ; projection des seules autorisations du brouillon, aucune génération réservée ni écriture du fichier |
| `POST /fabapps/sharing/publish` | Demande de publication/réessai de la configuration enregistrée ; retourne l’état en attente, pas une promesse de réussite |

`supported_platform` indique si le backend local est disponible ; il vaut false sur Windows, où l’interface désactive l’enregistrement de publication. Les phases sont `disabled`, `configured`, `pending`, `publishing`, `published`, `outdated`, `failed`, `withdrawal_pending`, `withdrawn`, `unsafe`. `last` contient génération, révision, horodatage, destination, empreinte des octets et indicateur vide. `local_access_blocked` vérifie la restriction de l’inode reconnu ; `existing_may_be_readable` ne constitue pas une preuve d’effacement des copies. Une erreur d’action renvoie 409 `publication_refused` sans contenu privé ni détail OS ; le statut donne un code stable de diagnostic. Les contrôles sont privés, pas exportés dans le catalogue.

La restauration d’une sauvegarde n’atteste pas sa source de publication : elle suspend les autorisations et déclenche le retrait si nécessaire avant de terminer sa transaction. Les générations et l’identité du poste restent conservées. Une nouvelle approbation est nécessaire, y compris après restauration sur le même poste.
