# Architecture

## Vue d'ensemble

```text
┌───────────────────────────── fabspeed (binaire) ─────────────────────────────┐
│                                                                               │
│  data/ (base) ─┐                                                             │
│  data/lab/ ────┴► loader ──► Catalog ─┐      ┌───────────────────────────┐    │
│  (essais, machines, matières, fraises)   ├────► │ fabspeed-core (pur)       │    │
│        ▲                              │      │ recommend · diagnose      │    │
│        │ écriture atomique + rechargement    │ essais · leviers · import │    │
│  HTTP /api/v1 (axum) ─────────────────┘      └───────────────────────────┘    │
│        ▲                                                                      │
│        │  JSON                                                                │
│  Interface web (Svelte, intégrée au binaire via rust-embed)                   │
└───────────────────────────────────────────────────────────────────────────────┘
          ▲                                    ▲
     navigateur du lab                 scripts locaux (API REST)
```

## Crates

### `fabspeed-core`

Bibliothèque pure, déterministe, sans IO, sans réseau, sans UI. Dépendances : `serde`, `serde_json`, `thiserror`, et `toml` derrière la fonctionnalité `toml` (parsing de texte uniquement).

| Module | Rôle |
| --- | --- |
| `model` | Données du catalogue : `MachineProfile`, `MaterialProfile`, `ToolSpec`, `LabPreset`, vocabulaire (`Aggressiveness`, `Evidence`, `OperationKind`…). |
| `policy` | `EnginePolicy` : tous les facteurs de correction nommés, avec leurs valeurs par défaut documentées. |
| `catalog` | `Catalog` en mémoire, recherche par identifiant / alias, `validate()`. |
| `request` | `RecommendationRequest`, `Overrides` (valeurs imposées), `RequestPatch` (proposition de modification). |
| `engine` | `recommend()` : préréglage ou modèle, valeurs imposées, évaluation, avertissements, trace, provenance. |
| `levers` | Leviers « Aller plus vite ». |
| `trials` | `Trial`, `TrialInput`, `TrialOutcome`, `Validation` ; essai « Bon » → réglage `tested`, validé par l'administrateur → `validated` ; historique par contexte. |
| `advisor` | `diagnose()` : symptôme → correction réévaluée. |
| `import` | Conversion d'une bibliothèque FabCAM `*.ivac-toolset.json`. |
| `export` | Essais en CSV (séparateur et décimales selon la langue). |
| `toml_data` | Texte TOML ↔ catalogue (`Catalog::merge_toml`, documents d'essai, de fraise et de machine, traductions). |
| `i18n` | `Lang` (`fr`, `en`), choix depuis `Accept-Language`, nombres et dates selon la langue, macro `tr!` (texte français et anglais côte à côte dans le code). |
| `localize` | `Translations` (textes des données par identifiant) et `Catalog::localized()` : copie du catalogue dans une langue, texte français si la traduction manque. |

Le vocabulaire reprend celui du moteur Feeds & Speeds de FabCAM (`ivac-feeds-speeds`) : modes `prudent / normal / rapid`, preuves `theoretical / reference_data / tested / validated`, opérations `profile / pocket / full_slot / finish / surfacing / engraving`, identités `{ id, revision }`.

### `fabspeed-server` (bibliothèque et binaire `fabspeed`)

| Module | Rôle |
| --- | --- |
| `lib` | `app(state)` (API + interface + CORS) et `serve(listener, state)`, réutilisés par l'application de bureau. |
| `main` | Binaire `fabspeed` : arguments (`--data`, `--host`, `--port`, `--print-default-policy`, `--reset-admin-password`), démarrage axum. |
| `install` | Données d'un poste : installation des données livrées au premier lancement, mise à jour à chaque nouvelle révision sans toucher à `lab/`, `auth.toml`, `policy.toml`, `backups/` (copie de sécurité avant). |
| `loader` | Lecture déterministe de `data/` puis de la couche `data/lab/` (une fiche du lab remplace la fiche de base de même identifiant), empreinte de révision, validation ; un catalogue par langue à partir de `data/i18n/`. |
| `lab` | Écriture d'une fiche par fichier dans `data/lab/`, atomique, suivie d'un rechargement ; retour arrière si le catalogue devient invalide ; archivage au lieu de suppression. |
| `lab_api` | Routes `POST/PUT/DELETE` des essais, fraises, machines et matières, réservées à l'administrateur. |
| `library_transfer` | Export JSON des fiches actives ; import additif JSON/TOML, FabCAM ou sauvegarde, aperçu, conflits par identifiant/alias ignorés, lot vérifié et écrit sous verrou avec retour arrière. |
| `backup` | Sauvegarde de toutes les données du poste (base livrée et `data/lab/`) en un fichier JSON, vérification (chemins, chargement complet dans un dossier temporaire), restauration avec copie de sécurité et retour arrière, export CSV des essais. |
| `auth` | Rôles : mot de passe administrateur du poste (empreinte Argon2id dans `auth.toml`), sessions en mémoire (jeton `Bearer`), extracteur `Admin` des routes d'écriture, routes `/auth`. |
| `state` | Catalogues partagés, un par langue (`Arc<RwLock<…>>`), rechargeables à chaud ; les anciens restent actifs si le rechargement échoue. |
| `api` | Routes `/api/v1`, erreurs JSON, langue de la requête (`?lang=`, puis `Accept-Language`). |
| `ui` | Fichiers de `ui/dist` intégrés (rust-embed) ; en debug, relus sur disque à chaque requête. Une adresse sans extension renvoie l'interface ; un fichier absent, 404. |

Écoute par défaut sur `127.0.0.1:3030` ; `--host 0.0.0.0` pour le réseau du lab.

### `fabspeed-desktop` (application de bureau)

Tauri 2 (même pile et même toolchain Rust 1.88 que FabCAM). Au démarrage : `install::install_base` depuis les ressources du paquet (`base-data/`) vers `app_data_dir/data`, `AppState::from_dir`, serveur lancé dans le processus sur `127.0.0.1:47830` (autre port si occupé), fenêtre unique sur cette adresse. Pas de commande Tauri : l'interface est exactement celle du navigateur. Téléchargements de la vue web enregistrés dans le dossier Téléchargements. Voir [ADR-0011](adr/0011-application-de-bureau.md).

### `ui`

Svelte 5 (runes) + Vite + TypeScript, icônes `@lucide/svelte`, police Inter embarquée. Même pile que l'interface de FabCAM. L'état est centralisé dans [store.svelte.ts](../ui/src/lib/store.svelte.ts) ; chaque changement de configuration déclenche un recalcul différé (90 ms) via l'API. Seuls la configuration en cours, les unités, la langue et le prénom de l'opérateur sont mémorisés dans le navigateur ; machines, fraises et essais vivent sur le serveur.

Premier lancement : [Onboarding.svelte](../ui/src/components/Onboarding.svelte) s'ouvre si aucun mot de passe administrateur n'existe et que la présentation n'a jamais été faite ou passée sur ce navigateur (`fabspeed.onboarded.v1`) ; elle se relance depuis l'aide.

Rôles : l'interface démarre en **invité** (calcul, consultation). Toute écriture du store passe par `ensureAdmin()` : si besoin, la fenêtre de mot de passe s'ouvre, puis l'action reprend ; une session perdue (serveur redémarré) redemande le mot de passe une fois. Le jeton est gardé pour l'onglet (`sessionStorage`) : fermer l'onglet repasse en invité. Le serveur reste seul juge (401 `admin_required`).

Langues : les textes de l'interface sont dans [ui/src/lib/locales/](../ui/src/lib/locales) (`fr.ts` fait référence, `en.ts` doit avoir les mêmes clés : TypeScript le vérifie) et passent par `t('clé')` ([i18n.svelte.ts](../ui/src/lib/i18n.svelte.ts)) ; les nombres suivent la langue (`16 850,5` / `16,850.5`). L'interface envoie sa langue au serveur (`Accept-Language`), qui renvoie avertissements, leviers, trace et données déjà traduits. Changer de langue recharge le catalogue et relance le calcul.

Deux modules, à la manière de Lightroom (sélecteur en haut à droite) : **Calcul** et **Bibliothèque**. La bibliothèque suit le schéma navigation / liste / inspecteur : la fiche sélectionnée s'édite dans le panneau de droite.

Anatomie (skills `professional-workshop-ui` et `cam-ux-ui`) :

```text
┌ Barre d'application : logo · machine · mm/pouces · FR/EN · invité/admin · aide ┐
├ Réglage (gauche) ─┬─ Espace de travail (centre) ────────────┬─ Assistant ────┤
│ 1 Matière         │ contexte · source · fiabilité           │ Aller plus vite │
│ 2 Fraise          │ 4 valeurs à reporter (modifiables)      │ Un problème ?   │
│ 3 Opération       │ avertissements                          │ Le calcul       │
│ 4 Conditions      │ schéma à l'échelle (coupe + dessus)     │                 │
│ ▸ Avancé          │ copeau · charges · métriques            │                 │
├───────────────────┴─────────────────────────────────────────┴─────────────────┤
│ Barre d'état : connexion · moteur · données · limites machine                  │
└────────────────────────────────────────────────────────────────────────────────┘
```

Thème sombre de référence et thème clair (même hiérarchie), surfaces neutres, couleur réservée à l'état (bleu = sélection/action, ambre = attention, rouge = bloquant, cyan = valeur du lab, violet = valeur modifiée). Couleur d'identité FabSpeed (magenta) limitée au logo et au liseré supérieur. Toutes les couleurs sont des jetons de [app.css](../ui/src/app.css) (aucune couleur en dur dans les composants, schéma de coupe compris) ; `:root[data-theme='light']` les redéfinit. Le choix (système, clair, sombre) est mémorisé dans le navigateur ([theme.svelte.ts](../ui/src/lib/theme.svelte.ts)) et appliqué par un court script d'`index.html` avant le premier affichage. Contraste du texte ≥ 4,5:1 en thème clair sur tous les fonds (vérifié par calcul WCAG). Sous 1 280 px l'assistant passe sous l'espace de travail ; sous 860 px tout s'empile. Les quatre valeurs passent en 2 × 2 quand le centre fait moins de 900 px (requête de conteneur, indépendante de la largeur d'écran) ; dans une cellule étroite, l'étiquette d'origine se réduit à son icône.

## Cycle d'une recommandation

```text
requête ─► préréglage exact ? ─oui─► valeurs du lab ──┐
              │ non                                    ├─► valeurs imposées ─► évaluation ─► avertissements
              └──────────► modèle théorique ───────────┘        (jamais corrigées)      leviers, trace, provenance
```

Une recommandation distingue toujours : la **valeur** (à reporter), son **origine** (testée, adaptée, calculée, saisie), la **fiabilité** des données (`evidence`) et l'**intention** (mode).

Boucle d'apprentissage du lab :

```text
calcul ─► usinage ─► « Noter un essai » ─► Bon ─────► réglage testé du contexte ─► (admin) Valider ─► réglage validé
                                          │
                                          └ À ajuster / Échec ─► diagnostic ─► corrections ─► nouvel essai
``` Les valeurs saisies par l'utilisateur ne sont jamais réécrites ; un changement de contexte (machine, matière, fraise, opération) les efface et le signale.

## Intégration FabCAM

Direction retenue : **catalogue FabSpeed → cache local FabCAM**. FabSpeed reste propriétaire des ressources, essais et validations ; FabCAM conserve ses identités locales, ses extensions FAO et son fonctionnement hors ligne. Aucun contenu de projet/opération ne revient dans FabSpeed et le moteur de recommandation n'est pas embarqué dans FabCAM.

FS-COMPAT-2 fournit la projection pure allow-listée, le [contrat v1](fabapps/catalogue/v1/README.md), une identité de source persistante et une liaison des validations à leurs dépendances/contexte. FS-COMPAT-3/4 ajoute la publication locale atomique après consentement administrateur. Aucun nouvel endpoint public de catalogue ; le consommateur sera développé dans FabCAM séparément. Les anciens ADR restent des traces historiques.

## Décisions

Voir [adr/](adr) : workspace Rust + API + interface intégrée (0001), données TOML hors du code (0002), précédence des réglages connus (0003), assistant de diagnostic inclus (0004), base construite à partir des essais (0005), écriture des fiches par l'application (0006), internationalisation (0007), rôles invité / administrateur (0008), niveau « Validé » (0009), types de fraises (0010), application de bureau (0011), dépôt public et données du lab hors dépôt (0012).

## Intégrité de persistance — FS-COMPAT-1

Voir le rapport et les limites de qualification (compte rendu interne non publié). `identity::DataLease` réserve la base à un processus par verrou OS non bloquant ; installation et reset du mot de passe prennent aussi ce verrou. AppState garde un registre monotone d'identités historiques et le mutex couvre allocation → écriture → recharge/rollback. Les créations utilisent `persist_noclobber`, les mises à jour vérifient le propriétaire du fichier avant remplacement. Les fonctions internes `_locked` évitent une reprise récursive du mutex ; reload public effectue un inventaire historique sous verrou.

La restauration conserve l'union des réservations et ne retire au rollback que les éléments réellement échangés. Son marqueur d'interruption bloque le redémarrage jusqu'à examen. Cela ne constitue pas une transaction durable à toute panne électrique. Le registre ne fournit aucune identité de source FabApps et ne change pas la validation des essais.

## Fondations du catalogue — FS-COMPAT-2

`source` conserve `.fabapps-source.json` sous le bail AppState, hors sauvegardes/imports ordinaires. `canonical` définit FAB-C14N-1/SHA-256 ; `catalogue` projette uniquement les champs explicitement autorisés depuis un catalogue français cohérent. L'action admin de validation ajoute une liaison optionnelle sans changer le choix des réglages du moteur. `Consent` est explicite, désactivé par défaut et désormais persisté par `publication` ; son interface se trouve dans Bibliothèque → Données du poste → Catalogue FabApps. Voir le contrat pour les règles de clone, reprise et générations.


## Publication locale — FS-COMPAT-3/4

`publication_api` utilise l’authentification existante. `publication` conserve le consentement et un journal privé ; `publication_fs` contrôle le chemin, le bail de destination et les opérations atomiques. La projection, le hachage et l’admissibilité restent ceux de `catalogue`/`canonical`, sans second modèle métier.

Le coordinateur unique prend le même verrou que les transactions du lab, les imports et les restaurations. Il lit ensuite le catalogue français et le consentement courant. Les notifications sont coalescées ; les entrées ne transportent jamais un ancien DTO. Le verrou possédé est déplacé dans `spawn_blocking`, pour rester détenu même si la tâche asynchrone est annulée. Le bail de la base reste détenu pendant ces écritures.

Ordre durable : journal de réservation → temporaire complet et synchronisé → remplacement dans le même dossier → synchronisation du dossier → génération source → journal validé. La reprise compare les octets attendus, conserve les générations réservées et refuse les états non démontrables. Désactiver produit un catalogue vide ; un échec de retrait essaie de retirer l’accès local au seul inode reconnu. Voir protocole et limites (compte rendu interne non publié).

Le bureau propose `app_data_dir/exchange/fabapps-catalogue-v1.json` à partir du chemin Tauri existant ; le serveur propose `--exchange-file` / `FABSPEED_EXCHANGE_FILE`. Aucun chemin global n’est deviné à partir d’un `--data` personnalisé. Pas de commande Tauri, permission filesystem large ou dialogue natif ajouté. La publication Windows est refusée tant qu’un backend de remplacement/ACL durable n’est pas qualifié ; Linux local est implémenté mais non qualifié nativement dans ce jalon.
