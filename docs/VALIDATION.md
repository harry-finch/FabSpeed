# Validation

Deux niveaux distincts : les **tests automatisés** prouvent que le moteur applique ses règles ; seuls des **essais sur machine** menés et consignés par une personne peuvent qualifier un réglage. Aucun test logiciel n'est une validation physique.

## Tests automatisés

```bash
cargo test --workspace
```

```bash
cd ui && npm run check
```

### Moteur (`crates/fabspeed-core/tests/engine.rs`)

Les tests de précédence (ENG-01 à ENG-24) chargent des réglages de référence **fictifs** ([tests/fixtures/presets.toml](../crates/fabspeed-core/tests/fixtures/presets.toml), `catalog_with_presets()`), puisque le jeu livré n'en contient pas. Aucun test ne demande de donnée privée d'un lab, sauf le calibrage (CAL-01/02), sauté si l'archive privée est absente.

| ID | Propriété vérifiée |
| --- | --- |
| ENG-01 | Préréglage exact repris tel quel (réglage fictif : 15 000 tr/min, 5 000 mm/min, passe 3 mm, 6 passes sur 18 mm) ; plongée marquée calculée |
| ENG-02 | Vf = N × Z × fz |
| ENG-03 | Petite fraise bornée par la rotation max de la broche |
| ENG-04 | Avance plafonnée par la machine → rotation abaissée, copeau conservé |
| ENG-05 | Prudent sans préréglage prudent → passe réduite, origine « Adapté », fiabilité théorique |
| ENG-06 | Préréglage prudent exact prioritaire |
| ENG-07 | Rapide sans préréglage rapide → réglage normal conservé et signalé |
| ENG-08 | Périmètre des préréglages non étendu (pas latéral, finition, autre machine, autre bois dur) |
| ENG-09 | Alias FabCAM des matières |
| ENG-10 | Entrées inconnues ou invalides → erreurs explicites |
| ENG-11 | Fraises non calculables (denture diamant, V) → `unsupported`, pas de valeurs |
| ENG-12 | Valeurs imposées conservées, vérifiées ; plongée calculée suit l'avance imposée ; frottement détecté |
| ENG-13 | Déterminisme et aller-retour JSON |
| ENG-14 | Fraise trop courte → bloquant, aucun levier |
| ENG-15 | Métaux : supervision, épaisseur de détourage, rampe |
| ENG-16 | Faible engagement radial : avance programmée plus forte, copeau réel identique |
| ENG-17 | Mur du maintien : profondeur réduite, copeau inchangé, levier « mieux tenir la pièce » |
| ENG-18 | Un levier appliqué réduit les passes sans dépasser 100 % de charge |
| ENG-19 | Diagnostic : correction appliquée et réévaluée |
| ENG-20 | Invariants sur tout le catalogue (> 500 combinaisons) : valeurs > 0, plages machine, plongée < avance, passe ≤ longueur utile, charges ≤ 100 % pour le modèle |
| ENG-21 | La validation du catalogue détecte les incohérences |
| ENG-22 | Durée estimée (passes + plongées) |
| ENG-23 | Aucun levier d'accélération pour les métaux supervisés, sauf `supervised_speedups` |
| ENG-24 | Le levier « passes plus profondes » avance d'un palier (≤ +50 %, ≤ 1 × D en pleine largeur) |
| ENG-25 | Le jeu livré part du modèle : aucun réglage ni essai, valeurs « Calculé », fiabilité théorique |
| ENG-26 | En anglais, aucun texte français ne subsiste (avertissements, leviers, trace, provenance, diagnostic) sur toutes les combinaisons du catalogue |
| ENG-28 | Fraise en V : largeur gravée 2 × p × tan(θ/2), rotation au maximum, copeau calé sur la largeur, plus profond = plus large (plafonné à D), sans angle → pas de calcul |
| ENG-29 | Hémisphérique en passe peu profonde : Dc = 2√(ap(D−ap)), compensation plafonnée, copeau réel dans la zone saine, vitesse de coupe au Dc |
| ENG-30 | Surfaçage Ø50 : copeau et passe calculés sur 12 mm, charges ≤ 100 %, avertissement « non calé » |
| ENG-31 | Compression : rappel de la première passe si elle n'atteint pas la partie upcut (ou si celle-ci est inconnue) |
| ENG-32 | Messages des nouveaux types de fraise traduits en anglais |
| ENG-33 | Hémisphérique en détourage multipasse : la dernière passe engage toute la profondeur, donc Dc = D, aucune compensation, copeau réel = avance par dent ; avertissement `ball_nose_2d` (et pas `ball_nose`) ; une passe unique peu profonde garde la compensation |
| ENG-34 | Rotation imposée seule : l'avance calculée suit (même avance par dent à l'arrondi près), origine « Calculé », information `feed_follows_rpm` ; avance imposée aussi : jamais touchée |
| ENG-35 | Temps de coupe au mètre : passes × 60 000 / Vf |
| ENG-27 | Matière de la couche lab signalée hors calibrage : `material_modified` (fiche de base remplacée) ou `material_from_lab` (créée) ; aucune alerte pour une matière livrée |

### Essais (`crates/fabspeed-core/tests/trials.rs`)

| ID | Propriété vérifiée |
| --- | --- |
| TRI-01 | Un essai « Bon » devient le réglage testé : valeurs exactes (plongée comprise), origine « Testé », fiabilité `tested`, historique |
| TRI-02 | Périmètre : détourage ≈ rainure, pas la poche ni une autre fraise |
| TRI-03 | À contexte égal, l'essai réussi le plus récent l'emporte ; historique du plus récent au plus ancien |
| TRI-04 | Un essai « à ajuster » n'est pas une référence ; reprendre ses valeurs déclenche l'avertissement de répétition |
| TRI-05 | Un essai de poche vaut pour un pas latéral plus faible, pas plus fort |
| TRI-06 | Saisie validée (fraise inconnue, rotation nulle, symptôme inconnu) |
| TRI-07 | Essai orphelin (fraise archivée) : ignoré, avertissement non bloquant |
| TRI-08 | Aller-retour TOML des fiches essai, fraise et machine |
| TRI-09 | Un réglage validé passe avant un essai réussi plus récent (signalé par `newer_tested_trial`) ; entre deux validés, le dernier validé l'emporte ; validation retirée → retour au plus récent testé |
| TRI-11 | Export CSV (avec l'état de la fraise) : BOM, du plus ancien au plus récent, valeurs à la française (`;`, virgule) ou à l'anglaise, validation exportée, champs protégés (séparateur, guillemets), autant de colonnes que d'en-têtes |
| TRI-10 | Seul un essai « Bon » se valide (message dans la langue demandée) ; validation conservée à l'aller-retour TOML |

### Calibrage (`tests/calibration.rs`)

| ID | Propriété vérifiée |
| --- | --- |
| CAL-01 | Mode Normal, ShopBot : le modèle seul reste sous l'avance et la profondeur de chaque réglage du wiki, et au-dessus de 35 % de l'avance (voir [CALCULATION_MODEL.md](CALCULATION_MODEL.md)) ; lit l'archive privée si elle est présente, sauté sinon |
| CAL-02 | Chaque réglage du wiki tombe dans la zone de copeau sain et sous 100 % de charge estimée |

### Autres

| ID | Emplacement | Propriété |
| --- | --- | --- |
| IMP-01/02 | `src/import.rs` | Import de la bibliothèque FabCAM synthétique livrée (hélice, denture diamant, hémisphérique) ; refus d'un autre type de fichier |
| EXP-01 | `src/export.rs` | Champs CSV protégés seulement si nécessaire (séparateur, guillemets, retour à la ligne) |
| I18N-01…03 | `src/i18n.rs` | Nombres selon la langue, langue depuis `?lang=` / `Accept-Language`, dates et pluriels |
| INST-01 | serveur `install.rs` | Premier lancement : données livrées copiées (sans base du lab), catalogue chargeable ; même révision : rien ; nouvelle révision : base remplacée, `lab/`, `auth.toml`, `policy.toml` intacts, copie de sécurité complète (sans mot de passe) |
| LOAD-01…03 | serveur `loader.rs` | Chargement des données du dépôt ; traductions anglaises chargées et appliquées sans identifiant inconnu ; dossier absent → erreur explicite |
| API-01…07 | serveur `api.rs` | Santé, catalogue, recommandation (modèle seul par défaut), erreurs JSON, diagnostic, import FabCAM, langue par en-tête ou paramètre |
| LAB-01 | serveur `lab_api.rs` | Création, modification et archivage d'une fraise : fichier écrit puis déplacé dans `archive/` |
| LAB-02 | serveur `lab_api.rs` | Modifier une fraise de base crée une version du lab ; l'archiver rétablit l'originale ; une fiche de base ne se supprime pas (409) |
| LAB-03 | serveur `lab_api.rs` | Fiche invalide : 422 et aucun fichier laissé sur le disque |
| LAB-04 | serveur `lab_api.rs` | Essai « Bon » enregistré → recommandation `tested` ; archivé → retour au modèle |
| LAB-05 | serveur `lab_api.rs` | Nouvelle machine utilisable immédiatement pour le calcul |
| LAB-06 | serveur `lab_api.rs` | Modifier une fiche de base en anglais garde ses textes français de base |
| LAB-07 | serveur `lab_api.rs` | Invité (sans jeton ou jeton inventé) : écritures refusées (401), rien sur le disque ; lecture ouverte ; premier mot de passe → écriture permise ; second « premier » mot de passe refusé ; changement réservé à l'admin et fermant les autres sessions ; déconnexion |
| LAB-08 | serveur `lab_api.rs` | Valider / retirer la validation : invité refusé, essai « à ajuster » refusé (422), inconnu (404), validation écrite dans le fichier de l'essai, preuve `validated` puis `tested` |
| LAB-10 | serveur `lab_api.rs` | Métaux : `supervised_speedups` ignoré pour un invité (seuls les leviers de sécurité), honoré avec une session administrateur |
| LAB-09 | serveur `lab_api.rs` | Matières : invité refusé ; création (`lab/<nom>`) utilisable aussitôt et signalée ; constante nulle refusée (422) ; fiche de base modifiée en anglais (textes français conservés, constante appliquée, signalée) ; archivage rétablit la base ; fiche de base non supprimable (409) |
| UI-01 | serveur `ui.rs` | Fichier absent (icône, script) → 404 ; adresse d'écran sans extension → interface |
| BAK-01 | serveur `backup.rs` | Seuls les fichiers de données connus sont acceptés (pas de `..`, d'absolu, de `auth.toml`, de `backups/`, d'autre dossier ou extension) |
| BAK-02 | serveur `backup.rs` | Sauvegarde complète (machines, matières, fraises, traductions, base du lab ; jamais le mot de passe) → machine de base modifiée, fraise archivée, nouvel essai → vérification (résumé, rien modifié) → invité refusé → restauration : fichiers identiques (machine de base comprise), validation conservée, copie de sécurité complète, mot de passe intact, aucun dossier d'échange ; nouvelle sauvegarde identique à la première |
| BAK-03 | serveur `backup.rs` | Sauvegardes invalides (format, version, chemin, `auth.toml`, sauvegarde complète sans matières, TOML illisible, matière incohérente) : 422, vérification comme restauration, aucun fichier modifié, aucune copie de sécurité |
| BAK-05 | serveur `backup.rs` | Sauvegarde au format 1 (base du lab seule) : acceptée, ne remplace que `data/lab/`, données de base intactes |
| BAK-04 | serveur `backup.rs` | Export CSV ouvert aux invités, en-têtes selon la langue |
| AUTH-01 | serveur `auth.rs` | Mot de passe trop court refusé ; empreinte Argon2id seule sur le disque ; relu au redémarrage (sessions perdues) ; changement et déconnexion ferment les sessions |
| AUTH-02/03 | serveur `auth.rs` | Sans dossier de données, personne n'est admin ; fichier `auth.toml` corrompu → démarrage refusé |
| LAB-U1 | serveur `lab.rs` | Identifiants et noms de fichiers sûrs, unicité |

Les tests serveur partent d'une **copie temporaire** du jeu livré, sans `data/lab/` ni `auth.toml` (`state::test_state`) : leurs résultats ne dépendent pas de la base réelle du lab, et ils n'y écrivent jamais. Les tests d'écriture ouvrent une session administrateur de test.

## Essais sur machine

L'application enregistre les essais (bouton **Noter un essai**) : contexte, réglages utilisés, résultat, constats, remarques, opérateur, date, versions. Un essai « Bon » fait passer le contexte de `theoretical` à `tested`.

### Ce qu'il est utile de noter

La fiche de l'application couvre l'essentiel ; dans les remarques, pensez à :

| Rubrique | À noter |
| --- | --- |
| Contexte | Date, opérateur, machine, version FabSpeed, révision des données |
| Matière | Nature, nuance, épaisseur, fournisseur |
| Fraise | Référence exacte, état (neuve / usée), sortie de pince mesurée |
| Maintien | Vis, brides, aspiration, scotch |
| Opération | Type, profondeur totale, pas latéral, entrée (plongée / rampe) |
| Réglages | Rotation, avance, plongée, profondeur de passe, passes |
| Observations | Son, vibrations, copeaux (forme, couleur), chaleur de la fraise, état de surface, cotes mesurées, usure |
| Conclusion | Accepté / à ajuster / refusé, et pourquoi |

### Progression

1. **Air cut** : parcours au-dessus de la pièce, vérification du Z et des limites.
2. **Chute** de la même matière, en mode Prudent.
3. Si le son, les copeaux et la surface sont bons : réglage Normal, puis éventuellement un levier à la fois.
4. Noter chaque passage, y compris les échecs : l'historique évite de refaire les mêmes erreurs.
5. Un essai noté « Bon » devient le réglage `tested` du contexte.
6. L’administrateur peut confirmer un essai Bon comme `validated` après revue des critères atelier ; le logiciel ne certifie pas la réussite physique.

Rappel : un `validated` n'est jamais généralisé à une autre fraise, matière ou machine.

## Import additif de bibliothèque

Tests serveur `library_transfer::tests` sur des copies temporaires des données livrées :

- aperçu sans écriture, ajout des trois types, fiches de base et du lab conservées, persistance après redémarrage et réimport sans doublons ;
- lot invalide ou collision de noms de fichier : aucun ajout ; erreur d'écriture après le premier ajout : retour arrière du lot ;
- conflits par alias de matière ou identité FabCAM, doublons internes, filtre par section ;
- TOML, bibliothèque FabCAM, aller-retour export JSON ;
- sauvegardes v1/v2 : extraction des fiches actives sans restauration du poste ;
- accès administrateur et imports concurrents : un seul ajout par identifiant ;
- conservation octet pour octet des essais validés, de la politique du moteur et du mot de passe.

## FS-COMPAT-1 — Identités et persistance

Qualification locale et mesures : rapport (compte rendu interne non publié). `identity_tests.rs` exerce les routes axum sur des bases temporaires : réservations historiques, collisions de chemins, créations concurrentes (32 fraises + 32 essais), alias, import/restauration, diagnostic en lecture seule, refus de permission/symlink sur Unix, processus concurrent et tailles 10/1 000/10 000. Aucune base réelle n'est utilisée.

Le correctif est qualifié sur macOS arm64 ; Windows/Linux natifs restent à vérifier avant une promesse multiplateforme. Les tests de cohérence logiciel ne constituent pas des essais physiques ni une validation de paramètres de coupe.

## FS-COMPAT-2 — catalogue et provenance

`catalogue_tests` vérifie la projection désactivée par défaut, les dépendances approuvées, l’exclusion des validations anciennes/révoquées/modifiées, les champs privés, les nombres et dates invalides, les révisions, les liaisons sous concurrence, la persistance/reprise/restauration et le maintien de source lors de la mise à jour. Les tests d’intégrité FS-COMPAT-1 restent dans la suite complète. Aucun essai n’utilise la base réelle du lab.

Le [contrat normatif](fabapps/catalogue/v1/README.md) comprend JSON Schema, exemple synthétique produit par Rust, cas invalide et 14 vecteurs de canonisation calculés indépendamment. `python scripts/check_fabapps_catalogue.py` (environnement avec `jsonschema==4.23.0`) vérifie schéma, dates, références, hashes et règles de générations, y compris des mutations négatives. La désérialisation serde seule n’est pas une qualification de consommateur.

Commandes : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo build --workspace --locked`, puis `npm ci --prefix ui`, `npm --prefix ui run check`, `npm --prefix ui run build`. Mesure bornée : `cargo test --release -p fabspeed-server --locked catalogue_scale_measurement -- --nocapture`. Deux tests de calibrage retournent sans exécuter les références privées lorsqu’elles sont absentes ; aucune preuve physique n’en découle.

La qualification de fichiers natifs reste propre à chaque plateforme : macOS exécuté ici, Windows/Linux non exécutés sans runners disponibles. Les commandes précédentes doivent être exécutées sur les cibles natives de la matrice V1 avant de les déclarer qualifiées. Aucune compilation croisée ne remplace ces preuves.


## Publication FabApps — FS-COMPAT-3/4

Les scénarios de `publication_tests` utilisent des répertoires temporaires et des essais synthétiques. Ils couvrent consentement, génération, fichiers inconnus/corrompus/anciens, réservations interrompues, sorties forcées de processus enfant, permissions, retrait avec blocage local, mutation/import/restauration, concurrence et fuite de sentinelles. Les tests FS-COMPAT-2 restent inchangés dans leurs assertions métier. Le lecteur Python contrôle les octets effectivement écrits :

```sh
python3 -m venv /tmp/fabspeed-contract-validator
/tmp/fabspeed-contract-validator/bin/pip install jsonschema==4.23.0
FABSPEED_PUBLICATION_EVIDENCE=/tmp/fabspeed-published cargo test -p fabspeed-server publication_tests --locked -- --nocapture
/tmp/fabspeed-contract-validator/bin/python scripts/check_fabapps_publication_replay.py /tmp/fabspeed-published
```

Sur macOS, le chemin final doit être canonique (`/private/tmp`, pas le lien `/tmp`). Les tests utilisent `canonicalize`. Ne pas lancer les tests de permissions en superutilisateur. Les essais natifs Windows/Linux et le parcours Tauri sont distingués des tests navigateur et des compilations dans le rapport (compte rendu interne non publié). Aucun réglage n’est déclaré physiquement validé par ces tests logiciels.
