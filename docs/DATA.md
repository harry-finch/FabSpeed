# Données

FabSpeed **part du calcul** : le jeu livré ne contient aucun réglage préenregistré. La connaissance du lab se construit avec les essais notés dans l'application.

Tout vit dans [data/](../data), au format TOML commenté (avec l'application de bureau : dans le dossier de données du poste, voir le [README](../README.md#application-de-bureau-v09-beta) ; les données de base y sont installées et mises à jour automatiquement). Deux couches :

| Couche | Qui l'écrit | Contenu |
| --- | --- | --- |
| **Base** (`data/*.toml`, `data/tools/`, `data/presets/`) | à la main, versionnée avec le code | Machines, matières, fraises et symptômes de départ |
| **Lab** (`data/lab/`) | l'application (et éventuellement à la main) | Essais, nouvelles machines et fraises, versions modifiées des fiches de base |

Les fichiers d'un même dossier sont lus par ordre alphabétique ; un catalogue incohérent est refusé avec la liste des erreurs. Après une modification à la main : `POST /api/v1/reload` ou redémarrage.

```text
data/
├── dataset.toml            révision du jeu de base (à incrémenter si on modifie la base)
├── machines.toml
├── materials.toml
├── advisor.toml            symptômes du diagnostic
├── policy.toml             (optionnel) facteurs du moteur : partir de `fabspeed --print-default-policy`
├── tools/*.toml            fraises de base
├── tools/*.ivac-toolset.json  (optionnel) bibliothèques FabCAM importées au démarrage
├── presets/*.toml          (optionnel, vide par défaut) réglages de référence importés
├── i18n/en.toml            traduction anglaise des textes (noms, descriptions, conseils, symptômes)
├── auth.toml               empreinte du mot de passe administrateur du poste (créé par l'application, hors Git)
├── backups/                copies de sécurité faites avant chaque restauration (hors Git)
└── lab/                    écrit par l'application, un fichier par fiche
    ├── trials/essai-AAAAMMJJ-HHMMSS-xxxx.toml
    ├── machines/<id>.toml
    ├── tools/<id>.toml
    ├── materials/<id>.toml
    └── archive/            fiches archivées depuis l'interface (récupérables)
```

La révision affichée dans chaque recommandation est celle de `dataset.toml` suivie d'une empreinte de `data/lab/` (`2026-10-22.2+lab.290c93`) : elle change dès qu'un essai ou une fiche du lab change.

## Essais

Un essai se note depuis le module Calcul (bouton **Noter un essai**), pré-rempli avec le contexte et les valeurs affichées :

| Champ | Contenu |
| --- | --- |
| Contexte | machine, matière, épaisseur, fraise (avec un instantané : nom, diamètre, dents), opération, pas latéral, profondeur, mode, maintien, sortie de pince |
| Réglages | rotation, avance, plongée et profondeur de passe **réellement utilisées** |
| Résultat | `good` (Bon), `adjust` (À ajuster) ou `failed` (Échec) |
| Constats | symptômes de `advisor.toml` (ça fume, ça siffle…) |
| Remarques, opérateur | texte libre |
| Traçabilité | date UTC, version du moteur, révision des données |
| Validation | (facultatif) `[trials.validation]` : `validated_at`, `validated_by`, `note`, ajoutée par l'administrateur |

Règles :

- Un essai **Bon** devient un réglage `tested` pour son contexte exact : même machine, même matière, même fraise, même opération (détourage et rainure sont équivalents), et pour les poches un pas latéral inférieur ou égal. La rotation, l'avance, la plongée et la passe de l'essai sont reprises telles quelles.
- À contexte égal, **l'essai réussi le plus récent l'emporte**, sauf s'il existe un réglage **validé**.
- L'administrateur peut **valider** un essai Bon (Bibliothèque → Essais, ou onglet Essais de l'assistant) : il devient `validated`, passe avant tous les essais testés du contexte (entre deux réglages validés, le dernier validé l'emporte) et s'affiche « Validé ». Retirer la validation le rend « Testé » ; l'essai n'est jamais modifié autrement.
- Un essai **À ajuster** ou **Échec** n'est jamais utilisé comme référence ; il reste dans l'historique, et le moteur prévient si l'on s'apprête à refaire les mêmes valeurs (± 3 % sur rotation et avance, ± 5 % sur la passe).
- Un essai dont la machine, la matière ou la fraise a été archivée est ignoré (avertissement non bloquant).
- Archiver un essai le déplace dans `data/lab/archive/trials/` ; pour l'annuler, remettre le fichier dans `data/lab/trials/` et recharger.

## Machines

Ne jamais inventer une limite réelle : chaque valeur incertaine porte une fiabilité dans `[machines.sources.<champ>]` (`verified`, `estimate`, `to_verify`) et une note. L'interface affiche « Estimation » ou « À vérifier » à côté.

**Ajouter une machine** : Bibliothèque → Machines → **Nouvelle machine**. On part d'un profil existant (ShopBot, routeur loisir, petite CNC) dont toutes les valeurs passent « À vérifier » ; on renseigne nom, rotation, puissance, avance max, effort latéral et facteur de rigidité. On peut aussi **Dupliquer** une machine.

**Modifier une machine de base** : la version modifiée est enregistrée dans `data/lab/machines/` avec le même identifiant et remplace l'originale, qui reste intacte dans `machines.toml` ; **Rétablir la fiche d'origine** archive la version du lab.

État du profil **ShopBot PRS Standard 96×48 (2016)** :

| Champ | Valeur | Statut | Origine |
| --- | --- | --- | --- |
| Rotation | 8 000 – 18 000 tr/min | estimation | Plage utilisée dans le wiki historique ; broches 2,2 kW ShopBot documentées 9 000–18 000 |
| Puissance broche | 2 200 W | à vérifier | Option courante sur PRS ; type de broche du lab non confirmé |
| Avance max | 6 000 mm/min | estimation | Maximum employé dans le wiki ; la fiche PRS5 annonce jusqu'à 360 in/min |
| Effort latéral | 180 N | estimation | Ordre de grandeur routeur portique, sans mesure |
| Courses | 2 464 × 1 245 × 152 mm | à vérifier | Course Z de 6 ou 8 pouces selon configuration |

**À faire sur la machine** : relever la plaque de la broche (puissance, régime nominal, plage du variateur), puis modifier la fiche et passer les champs confirmés en « Vérifiée ».

## Matières

| Champ | Sens |
| --- | --- |
| `vc_m_min` | Vitesse de coupe cible (carbure) |
| `k_fz` | Avance par dent nominale par mm de diamètre |
| `k_ap` | Profondeur de passe nominale (rainure) par mm de diamètre |
| `kc_n_mm2` | Pression spécifique de coupe (charges estimées) |
| `fz_min_mm` | Seuil de frottement |
| `default_stepover`, `default_thickness_mm` | Valeurs proposées par l'interface |
| `max_profile_thickness_mm` | Épaisseur max à détourer en plein (métaux : 10 mm) |
| `supervision_required` | Affiche « Toujours avec un fabmanager » et coupe les leviers d'accélération |
| `aliases` | Identifiants équivalents (ex. FabCAM `fablab-material-oak`) |
| `tips` | Conseils courts affichés à l'utilisateur |

Les matières se créent et se modifient dans la Bibliothèque (administrateur) : une matière de base modifiée est enregistrée dans `data/lab/materials/` avec le même identifiant et remplace l'originale, qui reste intacte dans `materials.toml` ; une nouvelle matière reçoit un identifiant `lab/<nom>` et part d'une matière proche (sans ses alias). Le calibrage CAL-01 ne porte que sur `materials.toml` : toute matière de la couche lab est hors calibrage, et le moteur le signale à chaque calcul (`material_modified`, `material_from_lab`). Modifier `materials.toml` à la main pour le jeu livré doit garder CAL-01 vert : le modèle reste sous les réglages historiques du wiki.

## Fraises

Les fraises livrées sont des géométries génériques, décrites dans [tools/generic.toml](../data/tools/generic.toml) (identifiants `generic/…`, **aucune donnée fournisseur ni réglage de coupe**) : droites upcut et downcut Ø3,175 / Ø6 / Ø8, compression Ø6, hémisphérique Ø3,175, en V 60° et 90°, surfaçage Ø25, taille diamant Ø3. Mesurez vos fraises et ajustez les fiches, ou importez votre bibliothèque. Points notables :

- `flutes` absent = nombre de dents inconnu (denture taille diamant) : pas de calcul. Si vous comptez les dents, modifiez la fiche : le calcul devient possible.
- `supplier_rpm_min` : rotation minimale du fournisseur ; si elle dépasse la broche, un avertissement le signale.
- `excluded_materials` : matières déconseillées par le fournisseur.
- `fabcam_identity` : identité FabCAM, utilisable comme `tool_id` dans l'API.
- `family` : `flat_endmill`, `ball_nose`, `compression`, `v_bit`, `diamond_cut`, `surfacing` (voir [CALCULATION_MODEL.md](CALCULATION_MODEL.md), étape 0).
- `tip_angle_deg` : angle de pointe, **obligatoire** pour une fraise en V (sinon pas de calcul).
- `upcut_length_mm` (facultatif) : longueur de la partie upcut d'une fraise à compression ; sert au rappel de la première passe.

**Ajouter une fraise** :

- rapidement depuis le calcul (sélecteur de fraise → **Nouvelle fraise** : nom, diamètre, dents, longueur utile, type) ;
- en détail dans Bibliothèque → Fraises → **Nouvelle fraise** (référence, fournisseur, lien, hélice, queue, rotations fournisseur, matières adaptées ou déconseillées, notes) ;
- par lot avec **Importer FabCAM** (`*.ivac-toolset.json`) : sens d'hélice et famille déduits du nom, à vérifier.

Les fraises créées reçoivent un identifiant `lab/<nom>` et un fichier dans `data/lab/tools/`. Une fraise de base modifiée est enregistrée comme version du lab (même identifiant), comme pour les machines.

## Réglages de référence importés (optionnel)

`data/presets/` accepte des réglages venus d'ailleurs (`[[presets]]`, voir [presets/README.md](../data/presets/README.md)). Il est **vide par défaut**. À contexte identique, un essai du lab (`tested`) l'emporte sur une référence (`reference_data`).

Les fichiers `data/presets/*.toml` sont ignorés par Git : ce sont des données d'un lab. Exemple : l'archive des réglages historiques du fablab d'origine (`docs/research/fablab-wiki/presets-wiki.toml`, privée ; son générateur n’est pas distribué ici) sert au test de calibrage et peut être réactivée en la copiant dans `data/presets/`.

## Données publiques et données du lab

Le dépôt ne contient que des données publiques : profils machine, matières, fraises génériques, symptômes du diagnostic, traductions. Licence : GPL-3.0-or-later, comme le code.

Les données propres à un lab restent sur ses postes et sont ignorées par Git (voir [.gitignore](../.gitignore)) :

| Chemin | Contenu |
| --- | --- |
| `data/lab/` | Base du lab écrite par l'application : essais, fiches créées ou modifiées, archives |
| `data/tools/cncfraises-*.toml`, `data/tools/*.private.toml` | Fiches fournisseur sans droit de redistribution |
| `data/presets/*.toml` | Réglages de référence importés |
| `data/policy.toml` | Politique de calcul locale |
| `docs/research/` | Archives de recherche (wiki, bibliothèque FabCAM, échanges) |

Une fiche placée dans `data/tools/` sous un nom ignoré est chargée localement mais n'est ni versionnée ni livrée par les installeurs (ceux-ci embarquent le dépôt). Pour qu'un poste installé garde ses fraises du commerce, les importer dans l'application (**Bibliothèque → Fraises → Importer FabCAM**) : elles rejoignent la base du lab (`data/lab/tools/`) en gardant leur identité FabCAM, ce qui préserve le lien avec les essais déjà notés. Exemple de bibliothèque : [docs/examples/fabcam-toolset.example.json](examples/fabcam-toolset.example.json).

Les tests n'utilisent que les données publiques et des réglages fictifs ([tests/fixtures/presets.toml](../crates/fabspeed-core/tests/fixtures/presets.toml)) ; seul le calibrage (CAL-01/02) lit l'archive privée quand elle est présente.

## Traductions

Les données de base sont rédigées en français. [data/i18n/en.toml](../data/i18n/en.toml) donne, par identifiant, les textes anglais :

| Section | Champs |
| --- | --- |
| `[materials."<id>"]` | `name`, `description`, `tips`, `source` |
| `[symptoms.<id>]` | `label`, `cause`, `explanation`, `advice` |
| `[machines.<id>]` | `name`, `category`, `description`, `notes`, `sources.<limite>` (note de fiabilité) |
| `[tools."<id>"]` | `name`, `notes` |

Seuls les textes se traduisent : les nombres restent dans les fichiers de base. Un texte absent retombe sur le français ; un identifiant inconnu (faute de frappe) est signalé dans `issues` au chargement. Les fiches créées dans l'application (`data/lab/`) s'affichent telles que saisies, dans toutes les langues. Ajouter une langue demande un fichier `data/i18n/<code>.toml`, un dictionnaire `ui/src/lib/locales/<code>.ts` et la traduction des messages du moteur et du serveur (voir [ADR-0007](adr/0007-internationalisation.md)).

## Sauvegarde, restauration, export

Bibliothèque → **Données du poste** :

- **Exporter en CSV** (tous) : tous les essais, une ligne par essai, avec noms et identifiants (machine, matière, fraise), valeurs, résultat, constats, remarques et validation. En français : séparateur `;` et virgule décimale ; en anglais : `,` et point.
- **Télécharger une sauvegarde** (administrateur) : un fichier JSON avec **toutes les données du poste** telles qu'elles sont sur le disque : `dataset.toml`, `machines.toml`, `materials.toml`, `advisor.toml`, `policy.toml` s'il existe, `tools/`, `presets/`, `i18n/` et la base du lab `lab/` (essais, validations, fiches ajoutées ou modifiées, archives). Le mot de passe du poste (`auth.toml`) et les copies de sécurité (`backups/`) n'y figurent pas.
- **Restaurer une sauvegarde** (administrateur) : le fichier est d'abord vérifié (format, chemins, fichiers lisibles, catalogue complet cohérent, chargé dans un dossier temporaire) et un résumé compare la sauvegarde aux données actuelles. Après confirmation, les données actuelles sont copiées dans `data/backups/avant-restauration-<date>.json`, puis chaque élément est remplacé et rechargé ; en cas d'échec, tout revient. Pour annuler une restauration, restaurer la copie de sécurité.

Une sauvegarde restaure le poste dans l'état où il était, données de base comprises : sur un poste installé avec une version plus récente des données de base, elle les ramène à celles de la sauvegarde (la révision est affichée avant confirmation). Une sauvegarde au format 1 (première version, base du lab seule) ne remplace que `data/lab/`.

Dans le dépôt de développement, `machines.toml`, `materials.toml` et `tools/` sont versionnés : une restauration les modifie comme le ferait une édition à la main (à vérifier avec `git status`).

## Intégrité des identités (FS-COMPAT-1)

Le correctif décrit dans FS_COMPAT_1_IDENTITY_INTEGRITY.md (compte rendu interne non publié) conserve les IDs existants et réserve aussi les IDs archivés, les alias et les références historiques dans `data/.identity-reservations.json`. Ne pas supprimer ce registre. Un échec de création peut réserver un ID sans créer de fiche ; il ne sera pas réattribué. Une mise à jour/surcharge de base conserve son ID, une copie en reçoit un nouveau. Les fichiers existants ne sont pas renommés ; collision de chemin ou propriété ambiguë = refus.

Un seul processus FabSpeed coopératif peut ouvrir la base en écriture ; les anciens binaires ignorant le verrou et les éditeurs externes concurrents sont hors contrat. Les nouvelles sauvegardes v2 transportent le registre ; une restauration unionne les réservations et ne remet pas les IDs postérieurs à disposition. Une version historique non reconnue exige un examen, sans remapping. Le diagnostic `--check-identities` est en lecture seule ; il ne répare rien. Les notes, opérateurs et contenus d'essais ne sont pas affichés par ce diagnostic.

## Source FabApps et validations liées

`.fabapps-source.json` est un contrôle privé créé sous le bail de la base : identité opaque aléatoire et compteur de publication (zéro à l’initialisation). Il n’appartient ni aux données livrées ni à `lab/` ; les mises à jour, sauvegardes v1/v2 et imports ordinaires ne le remplacent pas. `--check-identities` reste strictement en lecture seule.

Un clone indépendant exige une nouvelle source via `--initialize-independent-catalogue-source`, application arrêtée, sur la copie seulement. Un déplacement ou une reprise conserve la source uniquement après arrêt définitif de l’ancien éditeur et transfert du dernier contrôle de génération ; sinon nouvelle source. Les copies complètes entre hôtes ne sont pas détectables automatiquement. Voir les [règles normatives](fabapps/catalogue/v1/README.md).

`Trial.validation.binding` est facultatif pour lire l’historique, mais requis pour partager un preset qualifié. Toute reconfirmation capture les révisions et le contexte exacts ; rien n’est rétrocomplété. Les anciennes versions qui refusent les champs inconnus ne peuvent pas rouvrir ces nouveaux enregistrements. Les réglages historiques et le moteur restent inchangés.


### Consentement et journal de publication (FS-COMPAT-3/4)

`.fabapps-publication.json` est privé : version, `catalogue_id`, destination, autorisations par ID stable, réservation maximale consommée, publication en attente, dernière publication durable, erreur et restriction d’accès éventuelle. `.fabapps-source.json` ajoute `publication_initialized` et avance uniquement après publication du fichier. Le journal peut être en avance sur la génération source ; l’inverse sans réservation concordante est refusé. Ces fichiers restent hors sauvegardes/imports et hors rafraîchissement des données livrées. Ne pas les supprimer pour réparer une erreur ni revenir à un ancien binaire qui ignore ce protocole.

Une restauration conserve identité/générations mais suspend les autorisations, faute d’attestation fiable de la source de la sauvegarde. Le clone indépendant explicite crée une nouvelle identité et réinitialise le consentement, sans toucher au fichier publié par la source d’origine. Un déplacement de la base conserve ses deux fichiers de contrôle. Deux copies brutes simultanées avec la même identité sont hors contrat : initialiser explicitement une source indépendante avant utilisation de la copie.

Le fichier public est `fabapps-catalogue-v1.json`, mode 0600, dans un dossier local détenu par l’utilisateur (0700 si créé). Ses champs suivent strictement le contrat v1 existant. Taille maximale nommée : 64 Mio. Aucun numéro de version des données de coupe n’a changé : aucune donnée de base n’est modifiée.
