# Format `.loo` (Looping 4.1, x64)

Rétro-ingénierie de `Looping.exe` (SHA-256 `3ba42157…dabe3631`, MFC statique,
PDB `c:\tmp_compilLooping\looping\Release64\looping.pdb`) avec REA/Ghidra 12.1.4,
validée sur 10 fichiers réels (formats 8, 10 et 51).
Les adresses entre parenthèses sont celles des fonctions d'origine.

## Vue d'ensemble

Un `.loo` est un flux MFC `CArchive` produit par `CLoopingDoc::Serialize`
(`0x140074c20`). Petit-boutiste. Chaînes : `CString` Unicode
(`FF FE FF` + longueur [1/3/7 octets] + UTF‑16LE ; les très vieux fichiers
peuvent contenir des chaînes ANSI). Objets : protocole standard
`WriteObject`/`ReadObject` (`FFFF` + schéma + nom de classe pour une
nouvelle classe, `0x8000|index` pour une classe connue, index partagés
entre classes et objets, à partir de 1).

La version de format est le premier `int` du bloc de paramètres
(`CLoopingDoc+0x36c`). Version courante : **0x33 (51)**. Chaque champ
introduit après coup est conditionné par `version > N` ; à la lecture,
Looping applique des valeurs par défaut (reproduites dans
`Settings::upgrade_from`) puis réécrit toujours en 0x33.

## Document

| Ordre | Contenu | Condition |
|---|---|---|
| 1 | `int,int` taille de page (`+0x1910`) | |
| 2 | `int` couleur de fond (`+0x191c`), `int` (`+0x334`) | |
| 3 | `u16`, `u16` (`+0x338`, `+0x33c`), 4 octets (`+0x350`) | |
| 4 | **bloc brut 0x88c octets** (`+0x36c..0xbf8`) : version, polices `LOGFONTW`, couleurs, options MLD/SQL, SGBD cible (`+0x52c`), marqueur FK (`+0x7cc`, défaut `[FK]`)… | |
| 5 | 2 × (`int,int`) état de la fenêtre (zoom/défilement) | |
| 6 | `u16` (`+0x340`) | v > 0x1c |
| 7 | `CString` (`+0x348`) | v > 0x30 |
| 8 | `CObList` de `CUneVueUrbanisme` (`int` + `CString`) | v > 0x2b |
| 9 | 33 `CString` (`+0xbf8`) | v > 0xf |
| 10 | `CString` + 50 `CString` (`+0xd00`, `+0xd10`) | v > 0x12 |
| 11 | 3 `CString` puis 2 `CString` (`+0xea0` : indices 0,1,4 puis 2,3) | v > 0x12 / v > 0x23 |
| 12 | `CObList` des objets de dessin (`+0x17f8`) | |
| 13 | **passe des liens** : pour chaque objet, dans l'ordre de la liste, méthode virtuelle n°3 | |

## Objets (`CDrawObj`, `0x1400b2980`)

Base commune : code type `u32` (`u16` si v < 7 ; 1 association, 2 classe UML,
4 entité, 0x20 arc, 0x40 texte), 3 blocs de style (un par notation :
`u16` ×3, `LOGPEN` 16 octets, `LOGBRUSH` 12 octets ×2), **id** (`+0x10`),
4 `int` (`+0x308`), `int` (`+0x318`), `int` (`+0x338`, v ≥ 0x2d).

Passe des liens (`0x1400b3840`) : 6 `int` = ids de texte0, texte1, texte2,
segment précédent, segment suivant, propriétaire. Les arcs et traits
ajoutent un `CRattach` (`u16` mode, `int,int` point, `int`).

| Classe | Données propres (après la base) |
|---|---|
| `CDrawEntite`, `CDrawEntiteUML` (`0x1400c4210`) | `u16` drapeaux (bit 0 = entité fictive), `RECT` |
| `CDrawRelation` (`0x1400ddba0`) | centre, demi-largeur, demi-hauteur, … |
| `CDrawArc*` (`0x140099af0`) | (v > 0x26) 3 `int` + `CString` |
| `CDrawTexte` (`0x1400e52c0`) | 2 `int`, `CStringList` (nom), `CStringList`, `int`, `CString`, 3 `int`, 2 × (`int`+`LOGFONTW`+`int`), `u16` emplacement, `int` n + n × `CUnParametre` |
| `CUnParametre` (`0x1400a6800`) | rubrique ou cardinalité, environ 60 champs (voir `objects.rs`) |
| `CDrawImage`, `CDrawCif`, `CDrawContrainte`, `CDrawCrayon`, `CDrawEllipse`, `CDrawRectangle`, `CDrawTrait`/`CDrawFleche`, `CDrawHeritage`, `CDrawProf`, `CDrawRegle`, `CDrawTitre` | voir `crates/loo-format/src/objects.rs` |

## Sémantique

* Une entité ou une association possède 3 textes : emplacement 0 = nom,
  emplacement 1 = rubriques (`CUnParametre` de rôle 0).
* Une **patte** est une chaîne de segments `CDrawArc` (`prev`/`next`) :
  le propriétaire du premier et celui du dernier segment sont l'entité et
  l'association. Un segment intermédiaire sans propriétaire est un point
  de cassure (`CRattach` mode 0, coordonnées absolues).
* Cardinalité : paramètre de rôle 1 du texte 1 du premier segment,
  `+0xd8` : 0 `0,1`, 1 `1,1`, 2 `0,n`, 3 `1,n`, 4 personnalisée
  (`+0xe0`/`+0xe8`) (`0x14009dea0`). `+0x84` = identification relative.
* Rubrique : `+0x08` identifiant ; type `+0x20` (0 texte, 1 numérique,
  2 date, 3 booléen, 4 libre, 5 XML, 6 géométrie, 7 géographie) ; texte :
  `+0x28` jeu de caractères, `+0x24` fixe/variable/long, longueurs
  `+0x64..0x78` ; numérique `+0x2c` (entier/décimal/réel/monétaire/compteur),
  `+0x30` taille d'entier, `+0x34` réel simple/double, `+0x7c/+0x80`
  précision/échelle ; date `+0x38`, `+0x60` fuseau horaire.
  La table type → SQL par SGBD (`0x1400a7f70`) est reproduite dans
  `looping-core/src/types.rs`.
* SGBD (`+0x52c`) : 0 Access, 1 MySQL, 2 Oracle, 3 PostgreSQL,
  4 SQL Server, 5 Sybase, 6 DB2, 7 MariaDB, 8 SQLite, 0x14 personnalisé.

## Options de génération (bloc de paramètres)

| Offset | Option | Valeurs |
|---|---|---|
| `+0x52c` | SGBD cible | 0 Access … 8 SQLite, 0x14 personnalisé |
| `+0xa78` | Convention de nommage | 0 Standard, 1 Snake, 2 Screaming Snake, 3 Camel, 4 Pascal, 5 idem nom conceptuel |
| `+0xab8` | Supprimer les accents | 0/1 |
| `+0x7f4` | Noms des contraintes | 0 non défini, 1 préfixe `PK_/FK_/AK_`, 2 suffixe `_PK/_FK/_AK` |
| `+0xb40` | Placement des contraintes | 0 `CREATE TABLE`, 1 `ALTER TABLE`, 2 FK dans `ALTER TABLE` |
| `+0x888` | CASCADE | 0 non, 1 oui non validé, 2 delete, 3 update, 4 delete+update |
| `+0x7f8` | Commentaires | 0 non, 1 `/* */`, 2 `--`, 3 `COMMENT ON` |
| `+0x7fc` | Identifiant automatique | 0 `Id_[Classe]`, 1 `[Classe]_Id`, 2 `Id[Classe]`, 3 `[Classe]Id`, 10 personnalisé (`+0xa28`) |
| `+0x7cc` + `+0xa52` | Nom clé étrangère (masque) | `[FK]` (défaut), `[Classe]_[FK]`, `[FK]_[Classe]` ou texte libre ; 10 premiers caractères en `+0x7cc`, la suite en `+0xa52` |
| `+0xa6c` / `+0xabc` | Caractères de protection / de masquage | chaînes |
| `+0x7c0` | Affichage des associations fonctionnelles | 2 = association simplifiée |
| `+0xa00` | Notation de l'identification relative | `(1,1)`, `1,1 souligné`, `1,1(R)` |

Le reste de la grille « Propriétés » (polices `LOGFONTW` + couleur, couleurs,
dimensions) est catalogué dans `looping_core::options::FIELDS`, d'après la
fonction de rafraîchissement de la grille (`0x1400f9940..0x1400fb400`). Une
couleur à -1 signifie « Par défaut ». Fond par défaut d'un objet suivant les
couleurs du document (`0x1400b6270`, selon le code de classe) : héritage
`+0x7b0`, contrainte `+0x7b4`, CIF `+0x7b8`, règle `+0x7bc`, entité de modèle
externe `+0x804`, classe UML `+0x7ac`, table MLD `+0x884`.

Options de lien (dialogue « Lien », chargement `0x14002c960`, validation
`0x14002e2b1`) :

* paramètre de rôle (rôle 2) : libellé `+0xf0`, `+0x12c` afficher, `+0x130`
  suffixer les FK (ces deux-là sont stockés inversés), `+0x134` renommer la FK
  (l'emporte sur suffixer) ;
* paramètre de cardinalité (rôle 1) : `+0x128` lissage, `+0x140` entité ciblée
  par CIF, `+0x144` clé étrangère dans l'index (si `+0xa1c`), `+0x14c` place
  dans la clé primaire, `+0x148` « Récup clé primaire ».

Les traits et flèches libres (`CDrawTrait` / `CDrawFleche`) sont des chaînes
de segments comme les liens : un point par segment (ancrage `+0x390`),
épaisseur `+0x3a8`, style `+0x3ac`, forme et taille de la pointe
`+0x3b0`/`+0x3b4` (-1 : `+0x9fc` du document). La pointe est dessinée par
`0x1400f11e0`. `CDrawEntiteUML` (type 2) est la classe d'association de la vue
UML, possédée par sa `CDrawRelation`.

Illustrations (dialogues dérivés de `CDlgTexteForme`, partie commune
chargée par `0x14003ee80` et appliquée par `0x14003fd00`) :

* communs : `+0x308` fond « par défaut », `+0x30c` contour « par défaut »,
  `+0x310` fond affiché, `+0x314` contour affiché ; styles `a` (`+0xd8`,
  stylo « par défaut »), `b` (`+0xe4`, fond « par défaut »), `c` (`+0xe8`) ;
  l'outil « Texte » crée un `CDrawTitre` (pas un `CDrawTexte`) ;
* rectangle / ellipse : rectangle `+0x390`, épaisseur `+0x3a0`, style
  `+0x3a4` (index `PS_*`), ombre `+0x3a8`, couleur d'ombre `+0x3ac`, coins
  arrondis `+0x3b0` (rectangle) ;
* image : fichier `+0x390`, épaisseur `+0x3a8`, style `+0x3ac`, ombre
  `+0x3b0`, couleur d'ombre `+0x3b4`, modèle `+0x3b8` et vignette `+0x3c0`
  (outil « Sous-modèle », `0x140122bc0`) ;
* texte (`CDrawTitre`) : alignement vertical `+0x390` (0, 6, 8), horizontal
  `+0x318` (0, 6, 2), rectangle `+0x394`, coins `+0x3a4`/`+0x3a8`
  (« par défaut » / valeur), épaisseur `+0x3ac`, ombre `+0x3b0`, couleur
  d'ombre `+0x3b4`, et leurs « par défaut » `+0x3b8`, `+0x3bc`, `+0x3c0` ;
  valeurs du document : épaisseur `+0x370`, contour `+0x380`, fond `+0x390`
  et `+0x740`, contour affiché `+0x744`, ombre `+0x548`/`+0x54c`, coins
  `+0x7c8`, police `+0x3b0` ;
* trait / flèche : épaisseur `+0x3a8`, style `+0x3ac`, taille et forme de
  la pointe `+0x3b0`/`+0x3b4`, lissage `+0x3b8` ; crayon : épaisseur
  `+0x464`, style `+0x468`, sans textes ;
* lien entre illustrations (`CDrawArcObjetLibre`) : paramètre de rôle 6,
  libellé `+0x120`, lissage `+0x128` ; créé par l'outil Lien dès qu'une
  extrémité est un rectangle, une ellipse, une image ou un texte.

Affichage (ruban « Affichage », `CLoopingView`) : rubriques `+0x3ec`, types
`+0x3f0`, illustrations `+0x3e8`, recopiés dans les paramètres du document à
l'enregistrement (`+0x540`, `+0x544`, `+0x748`) et relus à l'ouverture ;
zoom en pourcentage `+0x2f0` (10 à 800, notre `frame_state[0]`). Vue UML
`+0x3f4`, vue MLD `+0x3f8`. Magnétisme : `CLoopingDoc+0x33c` activé,
`+0x340` demi-grille, pas `+0x334`. Vues : masque des vues actives de la
fenêtre `+0x130` (bit 0 = tous les objets) ; vue `CUneVueUrbanisme` = bit
`+8`, nom `+0x10` ; un objet est masqué si le document a des vues, si le
masque n'a pas le bit 0 et si `CDrawObj+0x338 & masque == 0`
(`0x1400b41f0`). Notation MCD `+0x7c0` (0 standard, 1 flèche pour DF,
2 association simplifiée), taille des flèches DF `+0x9f8`.

Le document garde les dernières valeurs validées dans chaque dialogue, que
prennent les objets suivants : crayon `+0x808..+0x810`, trait
`+0x814..+0x824`, flèche `+0x828..+0x83c`, rectangle `+0x840..+0x860`,
ellipse `+0x864..+0x880`, texte `+0x88c..+0x934` (police en `+0x890`),
image `+0xa90..+0xaac`.

Exports fichier (générateur `0x140066620`) : « Script LDD au format TXT »
(commande 33154) écrit, pour Access, un module VBA. Chaque `CREATE TABLE`
devient `DoCmd.RunSQL "CREATE TABLE T(" & _`, suivi de lignes
`   "…," & _` et terminé par `");"   `. Un `ALTER TABLE` se termine par
`" & _";"   `. Le reste du script (compléments, commentaires, scripts) est
inchangé, à une exception près : une ligne « fin de table » nommée d'après
chaque contrainte inter-associations est ajoutée avant les tables.
« MLD textuel au format XML » (commande 33187) : voir `sql::xml`. Les deux
exports sont refusés (« Erreur dans le MLD : impossible d'exporter le MLD
textuel ») si le MLD a des erreurs.

Les mots réservés SQL (301 mots, tableau `0x1404611a0`, comparaison
`0x1401041e0`) reçoivent un suffixe `_` après conversion, dans le MLD comme
dans le script : `user` → `user_`, `USER_` en Screaming Snake. La lecture
statique laissait penser le contraire ; les sorties de Looping sous Wine
(`samples/ref/`) font foi.

## Encore inconnu / non vérifié

* Sens de la plupart des champs du bloc de paramètres 0x88c (seuls les
  correctifs de version sont reproduits ; le bloc est conservé tel quel).
* Index et collations par cas. Confirmés par les sorties de Looping :
  - `CUnParametre` : `+0x88` = NOT NULL, `+0x8c` = UNIQUE, `+0xf8` =
    commentaire, `+0x118` = complément SQL ;
  - `CDrawArc` : `+0x3c8`/`+0x3cc`/`+0x3d0` = ON DELETE CASCADE / ON UPDATE
    CASCADE / Autre, `+0x3d8` = texte « Autre » ;
  - `CDrawRelation+0x3ac` = table de correspondance forcée ;
  - `Entite.flags` bit 1 = modèle externe ;
  - commentaire d'un objet = lignes de son texte d'emplacement 2 ;
  - `CDrawObj+0x140` (type) : Relation 1, EntitéUML 2, Entité 4, Image 8, CIF 0x10, Arc
    0x20, Texte 0x40, Titre 0x80, Héritage 0x100, ArcHéritage 0x200,
    ArcHéritageUML 0x400, Contrainte 0x800, ArcContrainte 0x1000, ArcCIF
    0x2000, Règle 0x4000, ArcRègle 0x8000, Trait 0x10000, Flèche 0x20000,
    Rectangle 0x40000, Ellipse 0x80000, ArcObjetLibre 0x100000, Prof
    0x200000, Crayon 0x400000 ;
  - paramètre de lien d'héritage (rôle 3) : `+0x114` surtype, `+0x120`
    critère, `+0x128` afficher, `+0x12c/+0x130/+0x134` options de FK,
    `+0x138/+0x144` renommage, `+0x14c` place dans la clé ; paramètre de lien
    CIF (rôle 4) : `+0x114` entité ciblée ;
  - symboles héritage / contrainte : type en `CUnParametre+0x110` de leur
    paramètre, script en `CDrawTexte+0x4a0`, ordre `+0x4d8` ;
    `CDrawHeritage+0x418` migration des rubriques ;
  - `CDrawRegle` : `+0x3a0`/`+0x3a8` afficher / description en commentaire,
    `+0x3ac` après les tables (stocké inversé), description = noms des
    paramètres ;
  - document : chaînes 0xea0 = compléments du script LDD ; `+0x7f0` types
    dans le MLD textuel ; `+0x7fc` modèle d'identifiant (10 = texte `+0xa28`).
* Règles du générateur MLD/SQL hors des cas couverts par `samples/ref/` :
  héritage, CIF, tables forcées. Les règles vérifiées sont résumées en tête
  de `crates/looping-core/src/mld.rs`.
