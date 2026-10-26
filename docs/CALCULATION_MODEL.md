# Modèle de calcul

Ce document décrit ce que fait `fabspeed-core` ([engine.rs](../crates/fabspeed-core/src/engine.rs)). Tous les coefficients cités vivent dans les données ([data/materials.toml](../data/materials.toml), [data/machines.toml](../data/machines.toml)) ou dans la politique du moteur ([policy.rs](../crates/fabspeed-core/src/policy.rs), surchargeable par `data/policy.toml`). Aucun coefficient empirique anonyme n'est codé dans l'algorithme.

## Principe

Les caractéristiques de la fraise et de la matière ne suffisent pas : il faut aussi la machine. Le moteur part d'une **cible idéale** (vitesse de coupe et épaisseur de copeau, constantes empiriques par matière), la passe à travers les **limites de la machine**, puis vérifie des **« murs » physiques**. Quand un mur est atteint, il réduit la **profondeur de passe**, jamais l'épaisseur du copeau : un copeau trop fin fait frotter la fraise, qui chauffe et brûle la matière.

## Précédence

FabSpeed part du **calcul**. Les réglages connus viennent ensuite des essais enregistrés au lab (voir [DATA.md](DATA.md#essais)) :

1. **Réglage connu** correspondant *exactement* : même machine, même matière (ou alias), même fraise, opération couverte (détourage et rainure sont équivalents), pas latéral inférieur ou égal pour une poche. Deux sources :
   - les **essais notés « Bon »** (`tested`), ou **validés** par l'administrateur (`validated`) ;
   - les **références importées** dans `data/presets/` (`reference_data`), vides par défaut.

   Ses valeurs (rotation, avance, plongée si elle a été testée, profondeur) sont reprises telles quelles.
   - Mode demandé absent : en **Prudent**, la profondeur du réglage Normal est réduite (facteur `modes.prudent.doc`) et marquée « Adapté », fiabilité Théorique ; en **Rapide**, le réglage Normal est conservé et l'interface le dit.
   - Plusieurs candidats : le plus fiable (`evidence` : validé > testé > référence), puis **le plus récent** (date de validation pour un réglage validé, date de l'essai sinon), puis l'identifiant. Résultat déterministe. Un réglage validé reste donc la référence même si un essai réussi plus récent existe ; celui-ci est signalé (`newer_tested_trial`).
2. Sinon, **modèle théorique** (ci-dessous), fiabilité Théorique.
3. **Valeurs imposées** par l'utilisateur : toujours prioritaires, jamais corrigées, mais évaluées (copeau, charges, plages machine). Rotation imposée seule : l'avance (non saisie) suit dans le même rapport pour garder l'avance par dent, bornée par l'avance max de la machine, et l'information `feed_follows_rpm` le dit ; la plongée calculée suit l'avance.

Les réglages ne sont jamais étendus hors de leur périmètre : un essai sur le chêne ne s'applique pas aux autres bois durs, ni un essai sur la ShopBot à une autre machine.

Les essais « À ajuster » et « Échec » ne sont jamais des références ; ils déclenchent un avertissement si les valeurs proposées les reprennent (± 3 % sur rotation et avance, ± 5 % sur la passe programmée).

## Modèle théorique

Notations : D diamètre (mm), Z nombre de dents, ae engagement radial, ap profondeur de passe, Vf avance (mm/min), N rotation (tr/min), Dc diamètre qui coupe réellement, D_ref diamètre de référence du copeau et de la passe.

### 0. Diamètre qui coupe et diamètre de référence (selon le type de fraise)

| Type | Dc (diamètre qui coupe) | D_ref (copeau, passe) | Particularités |
| --- | --- | --- | --- |
| Droite, compression, diamant | D | D | Compression : rappel de la première passe (au-delà de la partie upcut) |
| Hémisphérique | 2 √(ap × (D − ap)) si ap < D/2, sinon D | D | Passe × 0,5 ; rotation et copeau programmé compensés par D/Dc, plafonné à `ball_nose_max_compensation` (1,5) |
| En V (angle θ) | min(D ; 2 × profondeur gravée × tan(θ/2)) | max(Dc ; 0,5 × D) | Angle obligatoire ; copeau × 0,7 ; efforts sur la largeur gravée, résistance sur D_ref |
| Surfaçage | D | min(D ; 12 mm) | Copeau et passe calculés comme pour une fraise de 12 mm, largeur réelle conservée |

La profondeur de passe (étape 5) est calculée **avant** la rotation : pour une hémisphérique elle fixe Dc. Dc se prend à la **passe la plus engagée** : en détourage, rainure et finition, la fraise repasse dans le même sillon (ou longe tout le flanc), donc la dernière passe engage toute la profondeur usinée et Dc = D dès que celle-ci atteint D/2 (aucune compensation) ; en poche, surfaçage et gravure, chaque passe n'engage que ap. Une hémisphérique en détourage, poche ou surfaçage déclenche (la rainure à fond rond reste un usage normal) l'avertissement `ball_nose_2d` (préférer une fraise droite). Si les murs physiques réduisent ensuite la passe, Dc réel est plus petit que celui utilisé pour la compensation ; l'évaluation (copeau réel, vitesse de coupe, efforts) est toujours faite avec le Dc de la passe retenue. Facteurs dans `EnginePolicy.families`, `ball_nose_max_compensation` et `v_bit_reference_min_ratio`.

### 1. Rotation

```text
N = Vc × 1000 / (π × Dc)           Vc de la matière (× 0,5 pour une fraise HSS)
                                   hémisphérique : N(D) × min(D/Dc ; plafond)
N borné à [rpm_min ; min(rpm_max, rotation max de la fraise)]
```

Sur un routeur, les petites fraises butent presque toujours sur la rotation maximale.

### 2. Épaisseur de copeau visée

```text
h = D_ref × k_fz × facteur machine × facteur mode × facteur opération × facteur type de fraise × pénalité d'élancement
h ≥ fz_min (seuil anti-frottement de la matière)
```

Le copeau est proportionnel au diamètre : une petite fraise a des goujures et une âme plus fragiles.

### 3. Amincissement radial du copeau

Quand l'engagement radial r = ae/D est inférieur à 0,5, la dent sort de la matière avant d'atteindre l'épaisseur programmée. Pour garder le copeau visé, on programme plus :

```text
fz = h / (2 × √(r − r²))           facteur plafonné à rct_max_factor (2,0)
```

Fraise hémisphérique peu engagée : le copeau s'amincit aussi dans le rapport Dc/D ; `fz` est multiplié par la même compensation plafonnée que la rotation.

### 4. Avance

```text
Vf = N × Z × fz
```

Si Vf dépasse l'avance maximale de la machine, la **rotation est abaissée** pour conserver le copeau (`reduce_rpm_when_feed_limited`), sans descendre sous `rpm_min`. C'est aussi ce que faisaient les réglages historiques du lab, établis sur une ShopBot plafonnée à 6 000 mm/min.

### 5. Profondeur de passe

```text
ap = D_ref × k_ap × facteur petite fraise × bonus d'engagement × facteur mode × facteur opération × facteur type de fraise × pénalité d'élancement
facteur petite fraise = max((D_ref / 6 mm)^0,5 ; 0,4) pour D_ref < 6 mm
bonus d'engagement = 1,8 si r ≤ 0,25 ; 1,2 si r ≤ 0,5 ; sinon 1
ap ≤ 2 × D, ≤ longueur utile, ≤ passe max demandée, ≤ profondeur totale
```

### 6. Murs physiques

```text
Puissance de coupe   P = ap × ae × Vf × kc / 60 000                    (W, ae ≤ Dc pour une fraise en V)
Effort tangentiel    F = P × 60 000 / (π × Dc × N)                     (N)
Puissance dispo      P_broche × min(1 ; N / régime nominal)             broche à couple constant
Résistance fraise    4 × D² × pénalité d'élancement                     (N, heuristique)
Rigidité machine     max_lateral_force_n du profil                       (N, estimation)
Maintien             vis/brides 400 N · aspiration 150 N · scotch 60 N · inconnu 150 N
```

Chaque mur est comparé à une **charge visée** selon le mode (Prudent 60 %, Normal 80 %, Rapide 100 %). P et F étant proportionnels à ap, si le pire rapport R dépasse 1, `ap ← ap / R`. Si ap tombe sous 0,1 mm, l'avance est réduite à son tour et un avertissement est émis.

### 7. Plongée

Estimation documentée, non physique (reprise du pack FabCAM du lab) : première règle applicable de

| Règle | Matières / hélice | Plongée (mm/min) |
| --- | --- | --- |
| metal | métaux | min(0,10 × Vf ; 30 × D ; 100) |
| plastic | plastiques | min(0,10 × Vf ; 60 × D ; 150) |
| downcut | hélice downcut / compression | min(0,10 × Vf ; 60 × D ; 300) |
| default | autres (bois, panneaux, hélice upcut) | min(0,15 × Vf ; 100 × D ; 600) |

Une entrée en rampe est conseillée pour les métaux, les plastiques, les fraises downcut et les poches.

### 8. Passes et arrondis

La profondeur totale est répartie en passes égales : `passes = ⌈profondeur / ap_max⌉`, `ap = profondeur / passes`. Les valeurs calculées sont arrondies prudemment : rotation à 50 tr/min, avance et plongée vers le bas (10 et 1 mm/min), profondeur vers le bas à 0,05 mm. Les valeurs des préréglages restent exactes.

## Facteurs par défaut

| Facteur | Prudent | Normal | Rapide |
| --- | ---: | ---: | ---: |
| Copeau | 0,85 | 1,00 | 1,15 |
| Profondeur | 0,60 | 1,00 | 1,35 |
| Charge visée | 60 % | 80 % | 100 % |

| Opération | Engagement | Copeau | Profondeur |
| --- | --- | ---: | ---: |
| Détourage | 100 % (traité comme une rainure) | 1,0 | 1,0 |
| Poche | pas latéral demandé ou de la matière | 1,0 | 1,0 |
| Rainure | 100 % | 1,0 | 1,0 |
| Finition | 10 % | 0,5 | 1,0 |
| Surfaçage | 50 % par défaut | 0,8 | 0,25 |
| Gravure (fraise droite) | 100 % | 0,7 | 0,5 |

| Type de fraise | Copeau | Profondeur | Diamètre de référence |
| --- | ---: | ---: | --- |
| Droite, compression, diamant | 1,0 | 1,0 | D |
| Hémisphérique | 1,0 | 0,5 | D (compensation plafonnée à 1,5) |
| En V | 0,7 | 1,0 | max(Dc ; 0,5 × D) |
| Surfaçage | 1,0 | 1,0 | min(D ; 12 mm) |

Élancement : au-delà d'une sortie de pince de 5 × D, pénalité `(5 / élancement)²`, plancher 0,35. Sortie par défaut : longueur utile + 3 mm.

## Évaluation et zone de copeau

Toutes les valeurs (préréglage, calcul ou saisie) passent par la même évaluation : copeau réel `h = Vf / (N × Z) / facteur d'amincissement géométrique`, charges en % des limites physiques (100 % = limite estimée), débit de copeaux, durée si la longueur de parcours est connue.

Zone de copeau sain : de `fz_min` à `2,5 ×` le copeau nominal `D × k_fz × facteur machine`. En dessous : « la fraise frotte » ; au-dessus : « copeau épais ».

## Leviers « Aller plus vite »

Proposés seulement en l'absence de blocage et hors métaux (usinage supervisé), du plus sûr au plus agressif :

1. **Moins de passes, plus profondes** si la charge maximale est sous 60 % : un palier à la fois (+50 % au plus), jamais plus de 1 × D en pleine largeur, charge visée 75 %. Le copeau ne change pas.
2. **Rotation et avance ensemble** (+30 % au plus) si la broche et les axes ont de la réserve : copeau et effort par dent identiques.
3. **Rentrer la fraise dans la pince** si l'élancement pénalise ; **mieux tenir la pièce** si le maintien limite.
4. **Copeau plus épais** (+10 % d'avance) si la rotation est déjà au maximum et le copeau reste dans la zone saine.
5. **Mode plus rapide** si les charges sont faibles (calcul théorique uniquement).

Chaque levier est un `RequestPatch` : rien n'est appliqué sans action de l'utilisateur.

## Diagnostic

Les symptômes de [data/advisor.toml](../data/advisor.toml) appliquent des facteurs aux valeurs actuelles (rotation, avance, profondeur), bornés par la machine, puis la recommandation est **réévaluée** : la correction est vérifiée comme n'importe quel réglage. Exemple : « Ça fume » baisse la rotation de 15 % et monte l'avance de 10 % (copeau ≈ +29 %) ; si l'avance est déjà au maximum de la machine, seule la rotation baisse.

## Calibrage contre les réglages historiques d'un lab (CAL-01)

Les constantes des matières livrées ont été calées sous 21 réglages historiques testés sur la ShopBot du fablab d'origine (ancien wiki, 6 matières, 6 fraises). Ces réglages ne sont pas livrés (le jeu part du calcul) et ne sont pas dans le dépôt public : ce sont des données du lab. Exigence : en mode Normal sur la ShopBot, le modèle seul ne dépasse jamais l'avance ni la profondeur d'un réglage historique, tout en restant utile (≥ 35 % de l'avance testée). Au dernier calibrage, l'avance du modèle allait de 38 à 100 % de l'avance testée, la passe de 32 à 98 %.

Le test (`cargo test -p fabspeed-core --test calibration -- --nocapture`) lit l'archive privée si elle est présente sur le poste (`data/tools/cncfraises-fablab.toml` et `docs/research/fablab-wiki/presets-wiki.toml`) et affiche le tableau ; dans un clone public, il est sauté. Un autre lab peut calibrer de même avec ses propres réglages historiques.

CAL-02 vérifie en outre que chaque réglage historique tombe dans la zone de copeau sain et sous 100 % de charge : sinon la zone ou les limites seraient mal calées.

Les matières absentes de ces réglages (bois tendre, OSB, PMMA, PEHD, PVC expansé, mousses, HPL) partent de références générales prudentes. Toutes les recommandations restent « Théoriques » jusqu'au premier essai réussi enregistré pour leur contexte.

## Limites connues

- `kc` constant : la pression de coupe réelle augmente quand le copeau s'amincit (loi de Kienzle). Les charges sont des **comparaisons relatives**, pas des mesures.
- Effort admissible de la fraise (4 × D²) et rigidité machine (180 N pour la ShopBot) : heuristiques à confronter aux essais.
- Fraises hémisphériques : le diamètre qui coupe se déduit de la passe calculée ; en 3D, la passe réelle dépend du relief (la FAO la connaît, FabSpeed non).
- Fraises en V et de surfaçage : modèle géométrique non calé par des essais du lab (avertissement `tool_type_not_calibrated`). Fraise en V sans angle de pointe et dentures diamant (nombre de dents inconnu) : non calculées, l'interface l'explique.
- Pas de modèle de flexion, de broutage ni d'usure : volontairement hors périmètre (voir ROADMAP.md (compte rendu interne non publié)).
