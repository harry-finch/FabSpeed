# ADR-0007 : Internationalisation (français, anglais)

Status: Accepted
Date: 2026-10-22

## Context

La v1.0 doit être utilisable en anglais (feuille de route). Les textes à traduire viennent de trois endroits : l'interface, les messages produits par le moteur et le serveur (avertissements, leviers, trace, provenance, erreurs), et les données (noms et conseils des matières, symptômes, fiches de base). FabCAM consomme l'API et ne doit pas dépendre des phrases.

## Decision

- **Codes stables, phrases localisées.** Chaque avertissement garde son `code`, chaque étape de trace reçoit une `key` ; identifiants, codes et nombres ne dépendent jamais de la langue. Les phrases sont rendues dans la langue de la requête.
- **Langue de la requête** : `?lang=` puis `Accept-Language`, français par défaut (`ReqLang` dans `api.rs`). Pas de session, pas de réglage serveur.
- **Moteur** : `Lang` dans le catalogue (`Catalog.lang`) ; les messages sont écrits avec la macro `tr!(lang, "français", "english", …)`, les deux textes côte à côte. Nombres (`16 850,5` / `16,850.5`) et dates (`22/10/2026` / `2026-10-22`) suivent la langue.
- **Données** : le français reste la langue des fichiers de base ; `data/i18n/en.toml` donne les textes anglais par identifiant, sans aucun nombre. Le serveur construit au chargement un catalogue par langue (`Catalog::localized`) ; un texte manquant retombe sur le français, un identifiant inconnu est signalé.
- **Écriture** : modifier une fiche de base depuis l'interface anglaise ne remplace pas ses textes français par la traduction affichée (un texte renvoyé inchangé reprend le texte de base). Les fiches du lab sont affichées telles que saisies.
- **Interface** : dictionnaires TypeScript `ui/src/lib/locales/fr.ts` (référence) et `en.ts` (mêmes clés, vérifiées à la compilation), fonction `t('clé', { paramètres })`, choix mémorisé dans le navigateur, langue du système au premier lancement.

## Alternatives considered

- Moteur renvoyant uniquement des codes et paramètres, phrases construites par l'interface : plus pur, mais FabCAM et les scripts auraient dû réimplémenter des dizaines de messages paramétrés, et la trace (formules avec valeurs) se prête mal à ce découpage.
- Fichiers de messages (Fluent, gettext) côté Rust : dépendance et indirection supplémentaires pour deux langues ; les textes côte à côte dans `tr!` se relisent et se testent plus facilement.
- Bibliothèque i18n côté Svelte (svelte-i18n, Paraglide) : inutile pour deux dictionnaires statiques ; la vérification des clés par TypeScript suffit.

## Consequences

Deux langues seulement : en ajouter une troisième demande de compléter chaque appel `tr!` (ou de passer à des fichiers de messages), un fichier `data/i18n/<code>.toml` et un dictionnaire de l'interface. Toute nouvelle phrase du moteur doit être écrite dans les deux langues (la macro l'impose). Le test ENG-26 vérifie qu'aucun texte français ne subsiste en anglais sur tout le catalogue.

## Validation

ENG-26, I18N-01…03, LOAD-02, API-07, LAB-06 ([VALIDATION.md](../VALIDATION.md)) ; parcours vérifié dans le navigateur dans les deux langues sur une copie des données (calcul, diagnostic, essai noté en anglais puis relu en français, bibliothèque, aide).
