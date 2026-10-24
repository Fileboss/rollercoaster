# Feuille de route — Looping en Rust

Légende : ✅ fait · 🔜 prochain · ⏳ plus tard. Chaque jalon part d'une
lecture REA du binaire, avec les adresses notées dans `docs/FORMAT.md`.

## Version 0.1.0 — première release 🔜

Périmètre : **concevoir un schéma de base de données par un MCD**, de bout
en bout, sans défaut. Tout ce qui sort de ce chemin est reporté et listé
plus bas. Ordre de travail : du haut vers le bas.

Chemin critique :

1. ✅ Documents `.loo` : ouvrir (formats 1 à 51), enregistrer, rouvrir dans
   Looping
2. ✅ Nouveau document identique à l'octet à celui de Looping (bloc de
   paramètres capturé par `tools/wine/newdoc.py`, test `new_document`) ;
   « récupérer les propriétés d'un modèle existant » proposé comme Looping
3. ✅ Ne jamais perdre de travail : demande d'enregistrement avant de
   fermer la fenêtre, de créer ou d'ouvrir un document (Fichier › Quitter,
   Ctrl+N, Ctrl+O) — à vérifier à la main (point 10)
4. ✅ Édition du MCD : entités, rubriques et types, associations, liens et
   cardinalités, identification relative, rôles, héritage, CIF,
   contraintes, règles ; dialogues ; annuler, copier, aligner, ordre
5. ✅ Dessin correct des éléments du MCD : héritage (triangle, lettre,
   pattes pleines, flèche pleine vers le surtype), CIF (flèche pleine,
   pointillé vers l'association), libellés des liens de contrainte, rôles
   sous les cardinalités, règles (pentagone à coin replié, titre,
   description, script), agrandissement automatique des entités,
   associations, règles et héritages comme Looping (`edit::fit_box`) ;
   tous les cas sous 1,2 % de pixels différents sauf c03, c22, c23, c28
   (1,3 à 1,6 %, libellés approchés)
6. ✅ MLD textuel et script SQL identiques à Looping pour les 9 SGBD et les
   options (convention de nommage, contraintes, CASCADE, commentaires…)
7. ✅ « Compléments script LDD » modifiables (Paramètres), sur plusieurs
   lignes (CRLF, cas c50 identique à Looping)
8. ✅ Exports : script LDD (TXT, VBA pour Access), MLD XML ; image du
   schéma en PNG ou SVG depuis l'application (Fichier › Exporter l'image…,
   tel qu'affiché : options d'affichage et vues actives)
9. ✅ Affichage utile à la conception : rubriques, types, zoom, ajuster,
   magnétisme ; notation MCD (flèches DF, association simplifiée)
10. 🔜 Campagne de vérification : chaque opération du chemin, à la main et
    par cas synthétiques rouverts par Looping ; aucune panique.
    Fait : partie automatique —
    - `tests/campaign.rs` : suites aléatoires d'opérations du chemin
      critique, toutes les sorties et l'aller-retour `.loo` après chacune ;
      aucune panique sur 3 000 graines (180 000 opérations) ;
    - `examples/random_cases.rs` : modèles aléatoires « réalistes » rouverts
      par Looping sous Wine, MLD et SQL comparés (150 modèles), plus les cas
      c51 à c95 qui fixent chaque règle trouvée ; `examples/shrink.rs` réduit
      un écart à ses objets nécessaires.
    Reste : la vérification à la main dans l'application sur macOS.
    Écarts connus, hors v0.1.0 (modèles invalides, Looping refuse le script) :
    - références circulaires : ordre des rangs, clé d'un parent lue avant
      son héritage (`KNOWN` de `tests/reference.rs`) ;
    - deux CIF contradictoires sur une même association ;
    - modèles absurdes du mode « chaos » (`--chaos`) : associations à 3 ou 4
      pattes répétant une entité avec plusieurs x,1, identification relative
      au travers d'une réflexive, cycles d'héritage.
11. 🔜 Release : CI GitHub Actions (tests sur Linux et macOS ARM) qui
    construit `Rollercoaster.app` et le met dans un `.dmg` pour macOS ARM,
    attaché à la release `v0.1.0` ; README d'utilisation, notes de version.
    Sans compte Apple Developer, l'application n'est ni signée ni
    notarisée : au premier lancement, macOS demande une ouverture manuelle
    (Réglages Système › Confidentialité et sécurité › « Ouvrir quand
    même » depuis macOS 15), expliquée dans le README.
    Fait : `.github/workflows/ci.yml` (formatage, tests, livrables de
    chaque push), `.github/workflows/release.yml` (au merge sur `main`
    d'une version de `Cargo.toml` sans tag : tag `v<version>` et release
    avec le `.dmg` macOS ARM et le `.tar.gz` Linux x64, notes
    `docs/RELEASE-NOTES.md`), `tools/macos/bundle.sh`,
    `tools/linux/package.sh` (archive des binaires construite sur Ubuntu
    22.04). Reste : premier passage de la CI sur GitHub, merge sur `main`.

Reporté après v0.1.0 (déjà fait ou non, sans garantie pour cette version) :

- Vues graphiques MLD et UML et leurs notations (MLD standard / crow's
  foot, UML standard / simplifiée) — jalon 5 étape 4
- Urbanisation (dialogue 201) ; vues nommées déjà utilisables (filtre,
  appartenance) — étape 8
- Rétroconception SQL → MCD (`0x1400567d0`) — étape 8
- Impression, aperçu, configuration de l'impression ; export vers le
  presse-papiers ; formats jpeg, bmp, gif — étape 7
- SGBD personnalisé (dialogue 187), types libres prédéfinis (dialogue 189)
  — étape 5
- Options d'apparence du document : polices, couleurs, dimensions modifiables
  dans une grille — étape 5
- Fichiers récents, panneau de recherche dans le MLD et le script,
  masquer les panneaux — étapes 5 et 6
- Illustrations (faites à l'étape 3, hors garantie) et outil « Prof »
- Fidélité au pixel de l'affichage (jalon 4) au-delà du point 5 ci-dessus
- Paquet Linux : v0.1.0 livre une archive `.tar.gz` des binaires
  construite sur Ubuntu 22.04 (glibc ancienne) ; une AppImage, puis
  Flatpak, plus tard — jalon 6
- Signature et notarisation macOS, association du type `.loo`, icône
  soignée — jalon 6

## Jalon 1 — Fondations ✅

- ✅ Format `.loo` complet (versions 1 à 51), réécriture identique à l'octet en v51
- ✅ Modèle MCD : entités, associations, pattes, cardinalités, rubriques et types
- ✅ Table de correspondance types → SQL pour 9 SGBD (reprise de `0x1400a7f70`)
- ✅ Éditeur (macOS ARM, Linux x64) : affichage, zoom, déplacement, création et suppression, annuler/rétablir
- ✅ Couleurs du thème du document (`+0xa04`/`+0x394` pour les entités, `+0xa08`/`+0x398` ou `+0xa0c`/`+0x800` pour les associations), bandeaux de titre, associations en rectangle arrondi

## Jalon 2 — Génération MLD/SQL identique à Looping ✅

1. ✅ Mise en forme du script (`CREATE TABLE Nom(`, indentation de 3 espaces, `PRIMARY KEY(…)`, `FOREIGN KEY(…) REFERENCES …(…)`, `);`) et du MLD textuel (`Nom = (a, b, #c);`), confirmée par les littéraux de `0x140066620` et `0x14010bb90`
2. ✅ Conversion des noms (`0x140103be0`) : Standard, Snake, Screaming Snake, Camel, Pascal, identique au nom conceptuel, suppression des accents (`+0xab8`), suffixe `_` des mots réservés ; le MLD suit la même convention
3. ✅ Options du document décodées et appliquées, modifiables depuis le menu « Options SQL » :
   - convention de nommage (`+0xa78`), noms de contraintes (`+0x7f4`) ;
   - placement en `CREATE` ou `ALTER TABLE` (`+0xb40`), CASCADE (`+0x888`) ;
   - commentaires (`+0x7f8`).
4. ✅ Règles de passage vérifiées sur les sorties de Looping (résumé en tête de `mld.rs`) :
   - ordre des tables, ordre et nommage des FK (rôles, rang `_1`), mots réservés ;
   - identification relative et MLD groupé, 1-1, 1,1–1,1, n-aires avec une patte x,1 ;
   - entités sans identifiant, références circulaires, message d'erreur du script.
5. ✅ Mise en forme confirmée :
   - noms de contraintes dédoublonnés et `ALTER TABLE` groupé ;
   - pas de `NOT NULL` sur la clé, `UNIQUE(…)` en clause ;
   - CASCADE par lien (`CDrawArc+0x3c8/+0x3cc`, garde `+0x888`).
   - commentaires de colonne (`CUnParametre+0xf8`) et de table (texte d'emplacement 2), complément (`+0x118`), action « Autre » (`CDrawArc+0x3d8`) ;
   - tables forcées (`CDrawRelation+0x3ac`) et entités de modèle externe.
   - ✅ héritage, CIF, options de lien et masque des FK (voir jalon 3) ; Looping 4.1 n'émet aucun `CREATE INDEX`
6. ✅ Exports fichier, identiques à l'octet aux fichiers de Looping (ISO-8859-1, CRLF) :
   - « Script LDD au format TXT » : le script, ou pour Access un module VBA (`Sub Create_Tables()`, un `DoCmd.RunSQL` par `CREATE TABLE` / `ALTER TABLE`) ;
   - « MLD textuel au format XML » : `<Database>`, `<Table>`, `<Column>` (`<Type>`, `<Property>`, `<Comment>`), `<PK>` / `<AK>` / `<FK>` (`<Name>` de contrainte, `<Clause>` par action), caractères non ASCII en `&#N;` ;
   - les deux sont refusés quand le MLD a des erreurs ; Looping 4.1 n'émet jamais de `CREATE INDEX`.
   Corrections du script au passage : `ALTER TABLE` après les scripts et la fin de script, commentaire de script `/*  */` ou `--` selon `+0x7f8`, pas de ligne vide après le début de script sans règle « avant ».
7. ✅ Sorties de référence : `tools/wine/make-refs.sh` lance le vrai Looping sous Wine (sans fenêtre) et lit ses volets « MLD » et « LDD » pour chaque exemple et chaque SGBD → `samples/ref/` ; test `looping-core/tests/reference.rs`

## Jalon 3 — Sémantique restante ✅

- ✅ Codes de type `CDrawObj+0x140` de toutes les classes (`loo_format::kind`), distincts du code de classe de la vtable `+0x238`
- ✅ Héritage (`CDrawHeritage` 0x100, `CDrawArcHeritage` 0x200) : surtype = lien dont le paramètre de rôle 3 a `+0x114`, contrainte X/T/XT en `+0x110` (sans effet sur le MLD), « migration de toutes les rubriques » `CDrawHeritage+0x418` ; sous-types avec ou sans identifiant, deux niveaux ; rendu avec flèche et lettre de contrainte
- ✅ CIF (`CDrawCif` 0x10, `CDrawArcCif` 0x2000) : une association n-aire (cardinalités multiples), cible = lien de rôle 4 avec `+0x114` ; la clé de la cible sort de la clé primaire ; diagnostics en tête du MLD
- ✅ Contraintes inter-associations (`CDrawContrainte` 0x800) : type `+0x110`, diagnostics, script associé
- ✅ Scripts SQL : règles (avant/après les tables `CDrawRegle+0x3ac`, description en commentaire `+0x3a8`), héritage et contraintes (lignes `CDrawTexte+0x4a0`, ordre `+0x4d8`), « Compléments script LDD » (chaînes 0xea0 : début/fin de script, début/fin de table avec `[Nom_Table]`)
- ✅ Champs `CUnParametre` : NOT NULL (`+0x88`), UNIQUE (`+0x8c`), commentaire (`+0xf8`), complément (`+0x118`) ; index (`Index`/`Autre index`) sans effet sur le script LDD de Looping 4.1 (aucun `CREATE INDEX` émis)
- ✅ Options du document : catalogue `looping_core::options` (titres et valeurs de la grille de propriétés), menu « Paramètres » de l'application ; `+0x7f0` « Types dans MLD textuel » vérifié ; modèle d'identifiant `+0x7fc` utilisé à la création des entités
- ✅ « Nom clé étrangère » : masque `[FK]` / `[Classe]_[FK]` / `[FK]_[Classe]` (10 premiers caractères en `+0x7cc`, la suite en `+0xa52`, écrits par `0x140079fd0`), appliqué aux colonnes propres de l'entité référencée, puis rôle ; MLD groupé sous les noms migrés ; vérifié sous Wine sur tous les exemples (variantes `.fk-1` / `.fk-2`)
- ✅ Options de lien (dialogue « Lien », `0x14002c960`) : paramètre de rôle `+0x12c` afficher, `+0x130` suffixer (stockés inversés), `+0x134` renommer ; paramètre de cardinalité `+0x14c` place dans la clé primaire (tri stable des clés d'une table d'association, sans effet sur l'identification relative), `+0x148` « Récup clé primaire » (sans effet observé sur le MLD/LDD) ; ordre des identifications relatives corrigé (ordre du document)
- ✅ Bloc 0x88c : catalogue complet de la grille « Propriétés » (`looping_core::options::FIELDS`) — 11 polices `LOGFONTW` avec leur couleur, couleurs par type d'objet (fond par défaut `0x1400b6270` : héritage `+0x7b0`, contrainte `+0x7b4`, CIF `+0x7b8`, règle `+0x7bc`, modèle externe `+0x804`), dimensions, caractères de protection / masquage ; rendu des fonds par défaut
- ✅ Codes de type corrigés : `CDrawEntiteUML` = 2, `CDrawFleche` = 0x20000 (relus dans les constructeurs)
- ✅ Objets libres : traits et flèches (`CDrawTrait` / `CDrawFleche`, dessin `0x1400f11e0` : un point par segment via l'ancrage `+0x390`, pointillé `+0x3ac`), création (`edit::add_free_line`) vérifiée sous Wine ; rectangles, ellipses, crayon, liens entre illustrations (`CDrawArcObjetLibre`) déjà rendus
- ✅ Classes d'association UML (`CDrawEntiteUML`, possédée par sa `CDrawRelation`, textes copiés du nom et des rubriques, créée par Looping à l'usage) et liens d'héritage UML : sans effet sur le MLD, ignorés par la sémantique ; leur affichage relève de la vue UML (jalon 4)

## Jalon 4 — Fidélité de l'affichage ⏸ (en pause)

Reportée après la parité fonctionnelle (jalon 5). Les écarts d'interface
sont notés dans [FONCTIONS.md](FONCTIONS.md), colonne « Écart d'interface ».


- ✅ Oracle image : export « Exporter le modèle au format image » de Looping sous Wine (`tools/wine/make-image-refs.sh` → `samples/ref/*.png`, Carlito à la place de Calibri) ; test `looping-cli/tests/images.rs`
- ✅ Métriques GDI de Calibri reproduites avec Carlito embarqué (`looping_core::text`, vérifiées par `tools/wine/metrics.py`) ; taille de police dessinée `lfHeight × 0.78 × 100/72` (`0x140012dc0`)
- ✅ Mise en page des textes des entités, associations et cardinalités (`0x1400e9670`, `0x1400e5a20`, ancres `0x1400c59c0` / `0x1400e0710` / `0x14009cf30`) : polices du document ou propres au texte (`+0x390`), décalages `+0x458/+0x45c`, identifiants en gras soulignés, cardinalité opaque au milieu du premier segment ; boîtes des associations aux dimensions stockées ; ombres (`+0x548..+0x55c`)
- Textes des autres objets (règles, titres, contraintes, héritage), rôles et notations de cardinalité (`0x268`), répartition des pattes (`CRattach`)
- Notations alternatives : flèches DF, *crow's foot*, association simplifiée (`+0x7c0`)
- Vue MLD graphique et vue UML (2e et 3e blocs de style des objets)
- Affichage des images (`CDrawImage`) chargées depuis le dossier du document

## Jalon 5 — Parité fonctionnelle 🔜

Priorité : rendre **toutes** les fonctions de Looping accessibles dans
Rollercoaster, de façon logique, sans reproduire la disposition de
l'interface. L'inventaire complet (ruban, menus, dialogues, interactions),
avec l'état de chaque fonction, est dans [FONCTIONS.md](FONCTIONS.md).

1. ✅ Édition des objets du MCD : dialogues Entité, Association, Rubrique
   (types complets, propriétés), Lien (toutes les options) ; héritage, CIF,
   contraintes, règles et leurs liens (outils et dialogues). Noms logiques
   pris en compte dans le MLD et le script (cas c44), type libre corrigé
   (`+0x40`), liens de contrainte et de règle (cas c45)
2. ✅ Interactions : sélection multiple, redimensionnement, points de cassure,
   déplacement des textes, couper/copier/coller (presse-papiers interne),
   tout sélectionner, inverser, ordre des objets, alignement ; fichier
   produit rouvert par Looping (cas c46)
3. ✅ Illustrations : texte, ligne, flèche, rectangle, ellipse, crayon, image
   (fichier affiché), liens entre illustrations (libellé), sous-modèle
   (vignette) ; fenêtres de propriétés (dialogues 171, 182–186, 191, 203,
   204) et valeurs reprises d'un objet à l'autre ; fichier produit rouvert
   par Looping et exporté en image (cas c48)
4. 🟡 Affichage : ✅ rubriques, types, illustrations (enregistrés avec le
   document), zoom (curseur, 50/100/150 %, enregistré), magnétisme (grille,
   demi-grille), vues nommées (filtre, appartenance des objets), notation
   MCD (flèches DF dessinées) ; cas c49 rouvert et exporté par Looping.
   Reste : les vues graphiques MLD et UML et leurs notations (MLD standard
   ou crow's foot, UML standard ou simplifiée), qui sont des représentations
   à part entière du modèle
5. Document : options complètes (polices, couleurs, dimensions), SGBD
   personnalisé, types libres prédéfinis, compléments script LDD,
   remplacement de caractères, nouveau document aux paramètres neutres,
   fichiers récents, fermeture avec demande d'enregistrement
6. Panneaux : recherche dans le MLD et le script, grille de propriétés
7. Exports et impression : image (bitmap), presse-papiers, impression et
   aperçu
8. Urbanisation et rétroconception SQL → MCD (analyseur `0x1400567d0`)
9. Vérification que les fichiers écrits s'ouvrent dans Looping (sous Wine)

## Jalon 6 — Distribution ⏳

- Empaquetage de Rollercoaster : macOS ARM (`Rollercoaster.app`, icône, association du type de fichier `.loo`) et Linux x64 (`.desktop`, type MIME, AppImage ou Flatpak)
- CI : construction et tests sur `aarch64-apple-darwin` et `x86_64-unknown-linux-gnu`

## Pistes pour la suite 💡

Idées propres à Rollercoaster, au-delà de Looping, à étudier après la
v0.1.0 :

- **Serveur MCP pour piloter l'application** : exposer le modèle et les
  commandes de Rollercoaster par le Model Context Protocol, pour qu'un
  assistant (Claude…) puisse lire le MCD, créer ou modifier entités,
  associations, rubriques et liens, générer le MLD et le script SQL,
  exporter — en s'appuyant sur `looping_core::edit` (une étape
  d'annulation par opération, le document restant ouvert dans la fenêtre).
- **Bouton « Lancer la base de données »** : démarrer un conteneur Docker
  du SGBD choisi (MySQL, MariaDB, PostgreSQL, SQL Server, Oracle, SQLite…,
  image et version configurables) et y appliquer le script LDD généré,
  pour essayer le schéma tout de suite ; afficher les erreurs du SGBD,
  donner les paramètres de connexion, recréer la base quand le modèle
  change.
- **Découpler le modèle de la vue, puis un client web** : isoler l'état du
  document et ses commandes (édition, annulation, génération, exports)
  derrière une interface indépendante de l'interface graphique, au-delà de
  ce que `loo-format` et `looping-core` séparent déjà ; l'application de
  bureau et un client web (eframe se compile en WebAssembly) en seraient
  deux clients. Le serveur MCP ci-dessus profiterait du même découplage.
- **Plus tard, une application web** : modèles enregistrés côté serveur,
  partage et travail à plusieurs, sans installation.
