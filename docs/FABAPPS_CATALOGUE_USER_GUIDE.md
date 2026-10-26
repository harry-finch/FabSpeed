# Partager un catalogue FabApps

FabSpeed fonctionne seul. FabCAM ne synchronise pas encore ce fichier : sa fonction de lecture sera développée séparément.

1. Ouvrir **Bibliothèque → Données du poste → Catalogue FabApps → Configurer le partage**. Se connecter comme administrateur du poste.
2. Confirmer le chemin complet du fichier `fabapps-catalogue-v1.json`. Le bureau propose son dossier `exchange`. Le serveur peut proposer un chemin avec `--exchange-file /chemin/local/exchange/fabapps-catalogue-v1.json` (ou `FABSPEED_EXCHANGE_FILE`). Le dossier doit vous appartenir, sur un disque local, hors de la base privée. Un dossier manquant peut être créé dans un parent existant ; pas de lien symbolique. Sur macOS, utiliser `/private/tmp` pour un essai temporaire.
3. Cocher les machines, fraises, matières et préréglages souhaités. Aucun import ou nouvel enregistrement n’est automatiquement autorisé. Un préréglage nécessite un essai « Bon », une validation liée aux caractéristiques actuelles et l’approbation de ses trois dépendances. Le bouton d’approbation des dépendances est une action explicite ; une dépendance absente exclut le préréglage.
4. Cocher **Autoriser la publication**, puis **Aperçu de la sélection**. L’aperçu montre les seuls champs publics ; ouvrir les détails pour voir le JSON. Il n’écrit rien et n’engage pas sa génération provisoire.
5. **Enregistrer les autorisations**. Attendre **Publié avec succès** et vérifier chemin et date. **Publier maintenant / réessayer** utilise la configuration enregistrée. L’enregistrement seul n’est pas une preuve de publication. Les mutations réussies actualisent ensuite automatiquement le fichier ; des notes privées sans effet sur la projection ne provoquent pas de nouvelle génération.

Les autorisations survivent au redémarrage, au renommage d’une fiche, aux mises à jour du jeu de base et au rechargement. Après restauration d’une sauvegarde, le partage est suspendu et les ressources doivent être approuvées de nouveau : une sauvegarde n’atteste pas sa source de publication.

## Retirer le partage

Décocher une ressource et enregistrer pour l’omettre du prochain instantané complet. Un préréglage perdant sa validation, sa liaison aux caractéristiques ou une dépendance approuvée est également omis.

Pour tout retirer, décocher **Autoriser la publication** et enregistrer. Le résultat attendu est **Partage désactivé — catalogue vide publié** : même source, génération plus récente, listes vides. Les sélections peuvent rester mémorisées tant qu’aucune restauration ne les a suspendues.

**Retrait en attente n’est pas un retrait réussi.** Si la destination ne peut être mise à jour, FabSpeed essaie de retirer les permissions locales du seul fichier qu’il reconnaît (mode 000). Réparer les permissions du **dossier** ou l’espace libre puis réessayer : FabSpeed remplace ce fichier bloqué par le catalogue vide. Il ne rend pas temporairement lisibles ses anciennes données.

Si l’accès n’a pas pu être bloqué, arrêter FabSpeed et retirer l’accès au fichier avec les permissions du système avant réparation. Si l’enregistrement du consentement lui-même a échoué, ne pas redémarrer sans examen : les anciennes autorisations peuvent être rechargées. Ne pas supprimer les métadonnées de contrôle, remettre une génération à zéro ou choisir une nouvelle identité pour cacher l’erreur.

Aucun retrait ne peut effacer une copie déjà faite, un fichier déjà ouvert ou un cache hors ligne. Supprimer ou rendre un fichier inaccessible ne constitue pas une révocation vérifiable pour un consommateur : celui-ci devra accepter le nouvel instantané complet vide.

## Résoudre un échec

Vérifier le chemin, les permissions, l’espace libre et l’absence d’autre processus. Un fichier inconnu, modifié extérieurement, corrompu, plus ancien ou appartenant à une autre source est refusé, jamais remplacé automatiquement. Un état `unsafe` nécessite un examen par le mainteneur. Avant de changer une destination déjà publiée, réussir le retrait à l’ancienne destination.

L’échange est local, mode 0600, normalement accessible au même utilisateur. Une future application sandboxée peut nécessiter un emplacement choisi explicitement ou une autorisation OS ; le dossier privé Tauri n’est pas implicitement partagé entre applications. Pas de réseau, dossier synchronisé dans le cloud ou filesystem distant garanti. Publication Windows actuellement refusée ; qualification Linux native encore nécessaire. Le parcours Tauri macOS arm64 a été testé sur un profil synthétique dédié, y compris retrait et reprise sur panne de permission ; voir le rapport FINAL-2 (compte rendu interne non publié) pour les limites et l’incident de profil standard. Le sélecteur natif de destination reste différé.

## Vérifier indépendamment le fichier

Depuis le dépôt, dans un environnement Python disposant de `jsonschema==4.23.0` :

```sh
python scripts/check_fabapps_catalogue.py /chemin/exchange/fabapps-catalogue-v1.json
```

`PASS: complete catalogue` vérifie schéma, révisions, références et nombres. Ce résultat n’est pas une validation physique d’un réglage d’usinage.
