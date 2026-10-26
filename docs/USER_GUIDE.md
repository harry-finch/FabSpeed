# Guide utilisateur

FabSpeed vous donne des réglages de départ prudents pour fraiser à la CNC du fablab, et vous explique pourquoi. Pas besoin d'être machiniste : quatre choix suffisent.

## Premier lancement

Sur un poste neuf (aucun mot de passe administrateur), une courte présentation s'ouvre : langue et thème, mot de passe administrateur (facultatif, « Plus tard » possible), machine du poste (ou création d'une machine à partir d'un profil proche), puis un premier calcul commenté sur les vraies valeurs. **Passer la présentation** la ferme ; elle se revoit depuis l'aide (**Revoir la présentation**).

## En quatre étapes

1. **Matière** (panneau de gauche) : choisissez la matière et indiquez son épaisseur. Le petit flacon **Testé** signale qu'au moins un essai réussi a été enregistré pour cette matière sur cette machine.
2. **Fraise** : choisissez la référence gravée sur la fraise. La liste met en tête les fraises déjà testées avec cette matière, puis celles adaptées, puis les autres ; celles que le fournisseur déconseille sont en bas. Votre propre fraise ? **Nouvelle fraise** : nom, diamètre, nombre de dents, longueur utile, type ; elle rejoint la bibliothèque du lab.
3. **Opération** : détourage, poche (creuser), rainure, finition, surfaçage, gravure ; ce que fait l'opération choisie est rappelé sous les boutons. **Détourage** : découper une pièce, la fraise traverse la plaque. **Rainure** : un sillon de la largeur de la fraise, sans traverser. Les deux sollicitent la fraise sur toute sa largeur. La profondeur se remplit seule (l'épaisseur pour un détourage, la moitié pour une poche ou une rainure) ; pour une poche, réglez le pas latéral. Les opérations impossibles avec la fraise choisie sont grisées : une fraise en V ne fait que graver ; une hémisphérique fait rainures à fond rond, finition et gravure. Si vous changez de fraise, l'opération passe à une opération possible et FabSpeed le signale.
4. **Conditions** : comment la pièce est tenue (vissée, aspiration, scotch) et votre intention (Prudent, Normal, Rapide). En cas de doute : Prudent.

Thème : trois boutons dans la barre du haut, **écran** (thème du système), **soleil** (clair), **lune** (sombre) ; choix mémorisé sur ce poste.

Langue : boutons **FR / EN** dans la barre du haut. Le choix est mémorisé sur ce poste ; les valeurs ne changent pas, seuls les textes et l'écriture des nombres (16 850 / 16,850).

## Lire le résultat

En haut au centre, les **quatre valeurs à reporter dans la FAO** :

| Valeur | Ce que c'est |
| --- | --- |
| Rotation | Vitesse de la broche (tr/min). |
| Avance | Vitesse de la fraise dans la matière (mm/min). Pour la ShopBot, l'équivalent en mm/s est indiqué. |
| Plongée | Vitesse de descente. « Entrée en rampe conseillée » : préférez une descente en biais. |
| Profondeur de passe | Épaisseur enlevée à chaque passage, et le nombre de passes pour la profondeur totale. |

Chaque valeur porte son origine :

- **Testé** : valeur d'un essai réussi enregistré au lab, reprise telle quelle ;
- **Adapté** : dérivé d'un réglage testé (par exemple passe réduite en mode Prudent) ;
- **Calculé** : calcul théorique prudent, à confirmer par un essai ;
- **Modifié** : votre propre valeur.

Cliquez sur une grande valeur pour saisir la vôtre : FabSpeed la vérifie aussitôt (copeau, charges), sans jamais la corriger. Si vous imposez seulement la rotation, l'avance suit pour garder le même copeau ; imposez-la aussi pour la figer. La flèche circulaire revient à la valeur recommandée ; **Réinitialiser** revient au calcul de départ (leviers, corrections et valeurs saisies annulés). **Copier** met tous les réglages dans le presse-papiers ; au survol d'une valeur, l'icône de copie prend le nombre seul, à coller champ par champ dans la FAO (sur ShopBot, un clic sur « mm/s » copie la valeur par seconde).

Sous chaque valeur : **Plafond** indique ce qui la limite (rotation max de la broche, fraise trop sortie…). Le nombre de passes passe en ambre au-delà de 8 : l'usinage sera long ; le temps de coupe au mètre est indiqué (ou la durée totale si la longueur de coupe est renseignée). Quand le copeau réel est plus fin que l'avance par dent programmée (pas latéral faible, hémisphérique peu engagée), les deux sont affichés. Si la sortie de pince limite la passe, son champ remonte sous la fraise.

En dessous :

- les **avertissements** : un bandeau rouge bloque (par exemple fraise trop courte), un triangle ambre demande votre attention ;
- le **schéma à l'échelle** : la plaque, les passes, la fraise et sa sortie de pince ; les rayures de la fraise défilent dans le sens où partent les copeaux. La vue de dessus montre la part de la fraise engagée dans la matière ;
- **Épaisseur du copeau** : le repère blanc doit rester dans la zone verte. Trop à gauche, la fraise frotte et brûle ; trop à droite, elle force ;
- **Charges estimées** : la part des limites de la broche, de la machine, de la fraise et du maintien utilisée. Ce sont des estimations pour comparer, pas des mesures.

## Invité ou administrateur

Le bouton **Invité** / **Admin** de la barre du haut indique le mode :

- **Invité** (par défaut) : tout le calcul, l'assistant, la consultation de la bibliothèque et des essais. On peut modifier les valeurs pour expérimenter, mais rien n'est enregistré.
- **Administrateur** : noter des essais, créer, modifier et archiver machines et fraises, importer une bibliothèque FabCAM. Les boutons concernés portent un petit cadenas en mode invité ; un clic demande le mot de passe, puis l'action continue.

Au premier enregistrement sur un poste neuf, FabSpeed propose de **définir le mot de passe administrateur** (8 caractères minimum). Il est propre au poste et enregistré sous forme d'empreinte, jamais en clair. Menu **Admin** : changer le mot de passe ou repasser en invité. Fermer l'onglet du navigateur repasse aussi en invité.

Mot de passe oublié : sur le poste, lancer `fabspeed --reset-admin-password`, redémarrer FabSpeed, puis en définir un nouveau (les essais et fiches ne sont pas touchés).

## Noter un essai : la base se construit avec vous

Au départ, toutes les valeurs sont **calculées**. Après l'usinage, cliquez sur **Noter un essai** (mode administrateur) :

1. vérifiez les **réglages réellement utilisés** (pré-remplis, modifiables si vous avez changé quelque chose sur la machine) ;
2. choisissez le **résultat** : **Bon**, **À ajuster** ou **Échec** ;
3. cochez ce que vous avez **constaté** (ça fume, ça siffle, la pièce bouge…), l'**état de la fraise** (neuve ou déjà utilisée) et ajoutez vos remarques ;
4. indiquez votre prénom et enregistrez.

Un essai **Bon** devient la référence pour ce contexte exact (machine, matière, fraise, opération) : la prochaine fois, ses valeurs s'affichent avec l'étiquette **Testé**. Un essai **À ajuster** avec un constat ouvre directement le diagnostic : appliquez les corrections, refaites un passage, notez le nouvel essai. Si vous vous apprêtez à refaire des réglages déjà notés « à ajuster », FabSpeed vous prévient.

**Valider un réglage** (administrateur) : quand un réglage testé a fait ses preuves (plusieurs pièces conformes, par exemple), bouton **Valider** sur l'essai (onglet **Essais** ou Bibliothèque). La fenêtre rappelle tous les essais du même contexte et signale un essai plus récent noté « À ajuster » ou « Échec » ; indiquez qui valide et les critères vérifiés. Les valeurs s'affichent alors **Validé** (vert) et restent la référence, même si un essai réussi plus récent est noté ensuite : FabSpeed le signale, à vous de le valider à son tour. **Retirer la validation** revient à « Testé ».

L'onglet **Essais** de l'assistant liste les essais du contexte courant ; **Reprendre** réapplique les valeurs d'un essai.

## Bibliothèque : machines, matières, fraises, essais

Le module **Bibliothèque** (en haut à droite, à côté de **Calcul**) rassemble :

- **Machines** : **Nouvelle machine** part d'un profil existant dont les valeurs sont marquées « À vérifier » ; renseignez le nom, la plage de rotation, la puissance, l'avance max. Chaque limite indique sa fiabilité (Vérifiée, Estimation, À vérifier).
- **Matières** : nom, catégorie, constantes de coupe (vitesse de coupe, copeau et passe par mm de diamètre, pression de coupe, copeau minimal), pas latéral par défaut, épaisseur courante, supervision obligatoire, conseils. **Nouvelle matière** part d'une matière proche. En invité, la fiche se consulte en lecture seule ; modifier ou créer une matière demande le mot de passe administrateur. Les constantes livrées sont calées sous les réglages historiques du lab : les modifier, ou créer une matière, sort de ce calibrage ; l'éditeur le rappelle et chaque calcul avec cette matière le signale. Les matières se créent aussi depuis le menu de choix de la matière.
- **Fraises** : types droite, hémisphérique, compression, **en V** (angle de pointe obligatoire : il fixe la largeur gravée), denture diamant, surfaçage ; fiche complète (référence, fournisseur, géométrie, rotations du fournisseur, matières adaptées ou déconseillées), import d'une bibliothèque FabCAM, nombre d'essais réussis par fraise. La liste montre d'abord diamètre, dents et type, la référence fournisseur à droite ; des filtres rapides trient par type. La fiche signale une rotation minimale du fournisseur au-dessus de la broche active, et le lien s'ouvre d'un clic.
- **Essais** : tous les essais du lab, filtrables par résultat et matière ; **Ouvrir dans le calcul** restaure le contexte d'un essai. Chaque valeur d'un essai indique son écart au calcul théorique pour le même contexte (« +15 % vs calcul ») : de quoi voir si le modèle est trop timide ou trop agressif.

Dès qu'une fiche est modifiée, la barre **Enregistrer / Annuler** reste visible en bas de l'inspecteur (en invité, elle rappelle qu'il faut le mot de passe administrateur). Modifier une fiche de base (fournie avec FabSpeed) crée une version du lab ; l'originale n'est pas touchée et **Rétablir la fiche d'origine** la fait revenir. Archiver ne détruit rien : la fiche est rangée dans `data/lab/archive/` sur le serveur.

### Importer sans remplacer la bibliothèque

Dans **Machines**, **Matières** ou **Fraises**, **Importer** ajoute uniquement les fiches de cette section. Dans **Données du poste → Échanger des fiches**, l'import peut ajouter les trois types ensemble. Le mot de passe administrateur est requis.

1. Choisissez un fichier FabSpeed JSON/TOML, une bibliothèque FabCAM ou une sauvegarde FabSpeed.
2. L'aperçu liste les fiches à ajouter et celles ignorées car leur identifiant ou alias existe déjà. Aucune valeur n'est remplacée, même si la fiche importée diffère.
3. Cliquez sur **Ajouter**. Si une nouvelle fiche est invalide ou si l'écriture échoue, tout le lot est annulé.

**Exporter** produit un fichier de fiches FabSpeed JSON pour les échanger avec un autre poste (section courante, ou toute la bibliothèque depuis **Données du poste**). Les identifiants et la provenance sont conservés : réimporter le même fichier ne crée pas de doublons. Des noms identiques avec des identifiants distincts restent des fiches distinctes.

Importer une sauvegarde par ce chemin ne reprend que les machines, matières et outils actifs. Les essais, validations, réglages et paramètres du poste restent en place. **Restaurer une sauvegarde** garde son rôle de remplacement complet de la base.

## Assistant atelier (panneau de droite)

- **Plus vite** : leviers classés du plus sûr au plus agressif, avec le temps gagné. Au survol d'un levier, les valeurs qu'il donnerait s'affichent en vert au centre ; *Essayer* l'applique et les valeurs passent en « Modifié ». Le levier le plus sûr consiste presque toujours à faire moins de passes plus profondes : l'épaisseur du copeau ne change pas. Aucun levier n'est proposé tant qu'un point bloquant subsiste. Pour les métaux, seuls les leviers de sécurité (sortie de pince, maintien) sont proposés en invité ; en administrateur, les accélérations apparaissent aussi, à appliquer par paliers, fabmanager présent.
- **Problème** : décrivez ce que vous voyez ou entendez (ça fume, ça siffle, la pièce bouge…). FabSpeed explique la cause, propose une correction vérifiée et rappelle quoi contrôler.
- **Essais** : l'historique du contexte courant (voir plus haut).
- **Calcul** : d'où vient la recommandation, comparaison entre le réglage testé et le calcul seul, chaque étape avec sa formule, et les limites de la machine (« À vérifier » si elles ne sont pas confirmées).

## Bonnes pratiques

- Premier essai **sur une chute**, en restant près de l'arrêt d'urgence, et en écoutant : un son régulier est bon signe.
- Rentrez la fraise au maximum dans la pince.
- Vissez la pièce dès que possible ; une fraise upcut a tendance à la soulever.
- **Métaux : toujours avec un fabmanager.** Fixation par 6 pinces, aspiration coupée, air comprimé, pas latéral sous 30 %, Z vérifié entre chaque opération.
- En cas de doute, demandez à un fabmanager.

## Pour les fabmanagers

- Essais, matières, machines et fraises ajoutés depuis l'interface sont des fichiers TOML dans `data/lab/` sur le serveur. Bibliothèque → **Données du poste** : **Télécharger une sauvegarde** (un fichier avec machines, matières, fraises, essais et validations), **Restaurer une sauvegarde** (vérifiée et résumée avant confirmation ; les données actuelles sont d'abord copiées dans `data/backups/`), **Exporter en CSV** les essais pour un tableur.
- Les constantes des matières se modifient dans la Bibliothèque (version du lab dans `data/lab/materials/`) ou à la main dans `data/materials.toml` (voir [DATA.md](DATA.md)) ; après une modification à la main, recharger avec `POST /api/v1/reload` ou redémarrer.
- Le protocole d'essai recommandé est dans [VALIDATION.md](VALIDATION.md).
