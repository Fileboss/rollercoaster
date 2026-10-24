# Handoff — Looping en Rust

Travail commencé sur macOS, poursuivi sur Fedora 44 avec Wine.
Rien n'est commité : le dépôt contient le premier commit vide et des fichiers non suivis.

## Objectif

Réécrire en Rust pur **Looping 4.1** (outil Merise MCD → MLD → SQL,
C++/MFC, `Looping.exe`) à partir de sa rétro-ingénierie.
**Cibles : macOS ARM et Linux x64**, deux plateformes où Looping n'existe pas.
Sous Linux, le vrai Looping tourne sous Wine et sert d'oracle de test.
Le travail courant est le **jalon 2** de [docs/ROADMAP.md](docs/ROADMAP.md) :
une génération MLD/SQL identique à celle de Looping.

## Où en est le projet

- `crates/loo-format` : lecture et écriture des `.loo`, versions de format 1 à 51.
  Réécriture identique à l'octet en v51 ; Looping rouvre les fichiers que nous écrivons.
- `crates/looping-core` :
  - `mcd.rs` : modèle sémantique (NOT NULL `+0x88`, UNIQUE `+0x8c`, actions par lien) ;
  - `types.rs` : types et correspondance SQL pour 9 SGBD, repris du binaire ;
  - `naming.rs` : conversion des noms et suffixe `_` des mots réservés ;
  - `mld.rs` : règles de passage de Looping, résumées en tête du fichier ;
  - `sql.rs` : script, options du document, message d'erreur si le MLD est invalide ;
  - `render.rs` : display list et SVG, couleurs du thème ;
  - `edit.rs` : édition qui conserve la structure d'objets de Looping ;
  - `examples/cases.rs` : modèles synthétiques pour les cas absents des exemples ;
  - `tests/reference.rs` : comparaison avec les sorties du vrai Looping.
- `crates/looping-cli` : `looping-cli <json|mcd|mld|sql|svg|png|resave> FICHIER [SORTIE] [--dbms N] [--set OFFSET=VALEUR]...`.
- `crates/rollercoaster` : l'application **Rollercoaster**, éditeur egui/eframe (binaire `rollercoaster`), testée sous Linux (Wayland).
- `tools/wine/` : exécution du vrai Looping sous Wine pour produire `samples/ref/`.
- `docs/FORMAT.md`, `docs/ROADMAP.md`, `docs/rea-evidence.json`, `tools/re/` : inchangés dans leur principe.

Fichiers à ne pas publier, désormais dans `.gitignore` :
- `Looping.exe` : logiciel propriétaire ;
- `samples/` : TP d'étudiants, `beeven.loo`, cas synthétiques et références ;
- les deux `.zip` à la racine.

Paramètres par défaut d'un nouveau document :
`crates/loo-format/assets/default_settings.bin` est le bloc 0x88c d'un
nouveau document de Looping (`tools/wine/newdoc.py`) ; `Document::default()`
s'enregistre à l'octet près comme ce document
(`crates/loo-format/tests/data/new_document.loo`).

## Installation sur Fedora

Aucun paquet `-devel` n'est nécessaire : eframe charge X11/Wayland/GL dynamiquement
et rfd passe par le portail XDG.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
cargo test                          # tests/reference.rs utilise samples/ et samples/ref/
cargo run --release -p rollercoaster -- samples/beeven.loo
ROLLERCOASTER_SCREENSHOT=/tmp/x.png cargo run -p rollercoaster -- samples/beeven.loo   # capture puis quitte
```

## Sorties de référence avec Wine

```sh
sudo dnf install wine        # déjà présent ici (wine 11 Staging)
tools/wine/make-refs.sh      # ≈ 25 min pour tout ; résultats dans samples/ref/
```

- **Fonctionnement :** `extract.sh` lance Looping dans un préfixe dédié
  (`~/.cache/looping-wine`, pilote graphique `null`, donc sans fenêtre).
- **Python :** un Python Windows « embeddable » (`~/.cache/looping-winpy`,
  téléchargé au besoin) lit par `WM_GETTEXT` les deux RichEdit « MLD » et
  « LDD » que Looping remplit à l'ouverture d'un document.
- **Variantes :** elles sont écrites par notre `resave --set`, ce qui
  vérifie aussi que Looping rouvre nos fichiers :
  - SGBD `+0x52c` ;
  - options `+0x7f4`, `+0x7f8`, `+0x888`, `+0xa78`, `+0xab8`, `+0xb40` ;
  - leurs combinaisons.
- **Nommage :** `<stem>[.<sgbd>][.set-<off>-<val>].{mld.txt,sql}`.
- **Piège :** ne pas modifier un script `.sh` pendant qu'il tourne, car
  `sh` le relit au fil de l'exécution.

État : 299 documents (10 exemples, 23 cas synthétiques, variantes),
MLD et SQL **identiques** à Looping.

Établi grâce aux références, contrairement à certaines hypothèses précédentes :
- **Nommage dans le MLD :** il suit la convention de nommage ; les mots
  réservés reçoivent `_` (`user_`) ; les noms ne sont pas tronqués
  (`"Nécessiter "` → `Nécessiter_`).
- **Ordre des tables :** passes successives sur les entités, puis tables
  d'association. Les FK migrées suivent l'ordre inverse des associations.
- **Clés et contraintes :**
  - `***CLE_ABSENTE***`, « Attention ! Références circulaires » et le message
    « Impossible de générer le script SQL » ;
  - `UNIQUE(…)` en clause de table ;
  - `NOT NULL` jamais émis sur la clé primaire ;
  - rang `_1` ajouté après la convention (`idStudent_1`) ;
  - noms de contraintes `FK_<table>_<référencée>` dédoublonnés (`…_1`, ou
    `…_1_FK` en suffixe).
- **`ALTER TABLE` :** un seul par table, avec ses `ADD` séparés par des virgules.
- **Rôles et identification relative :**
  - le rôle sert de suffixe, sinon le rang ;
  - identification relative : FK en tête de clé, MLD groupé `#(#a, b)` ;
  - 1-1 : `UNIQUE` ; 1,1–1,1 : migration des deux côtés ;
  - ternaire avec une patte x,1 : pas de table ;
  - les attributs d'une association x,1 sont ignorés.
- **CASCADE :** se règle par lien, avec les drapeaux du premier segment de
  l'arc côté parent (`CDrawArc+0x3c8` DELETE, `+0x3cc` UPDATE, `+0x3d0` → une
  espace). L'option `+0x888` sert seulement de garde (≠ 0).

- **Commentaires (`+0x7f8`) :**
  - commentaire de colonne = `CUnParametre+0xf8`, émis comme `, /* x*/` ou `, --x` ;
  - commentaire de table = lignes du texte d'emplacement 2 de l'entité ou de
    l'association (méthode virtuelle `+0x2a0`, `0x1400c7d40`). Il est placé avant
    `CREATE TABLE`, ou en `COMMENT ON TABLE`/`COLUMN` juste après `);`.
- **Rubriques et liens :**
  - `CUnParametre+0x118` = « Complément », ajouté après `NOT NULL` ;
  - action « Autre » d'un lien = `" " + CDrawArc+0x3d8`.
- **Tables forcées et entités externes :**
  - table forcée (`CDrawRelation+0x3ac` ≠ 0) seulement si la patte x,1 est 0,1 ;
  - entité « modèle externe » (`Entite.flags` bit 1) : pas de table, mais sa
    clé migre.

Jalon 3 (en cours) — voir `docs/ROADMAP.md` pour le détail :
- **Héritage, CIF, contraintes, règles :** `edit.rs` sait créer ces objets
  (`add_heritage`, `add_cif`, `add_contrainte`, `add_regle`, `set_script`).
- **Cas synthétiques :** c24–c32 ; leurs sorties sont identiques à Looping.
- **Codes de type :** attention, `CDrawObj+0x140` n'est pas le code de classe
  renvoyé par la vtable `+0x238` (par exemple héritage `0x100` contre `0x80`) ;
  la table complète est dans `loo_format::kind`.
- **Options du document :** `looping_core::options` (menu « Paramètres »).
  `+0x888` est la valeur de CASCADE proposée aux nouveaux liens ; toute valeur
  non nulle active les actions par lien.
- **Outils :** `tools/re/vtable.py` (RTTI : vtable d'une classe, propriétaire
  d'une entrée) ; `looping-cli resave --set-str OFF=TEXTE`.

Jalon 3 terminé (voir ROADMAP) :
- **Masque « Nom clé étrangère » :** `Settings::fk_mask` (`+0x7cc` + `+0xa52`).
  Changer `+0x7cc` seul ne suffisait pas car Looping coupe le masque à 10
  caractères. Le masque s'applique aux colonnes propres de l'entité
  référencée ; une colonne déjà étrangère garde son nom.
- **Options de lien :** `Leg::{role_suffix, role_rename, place}` ; dialogue
  « Lien » `0x14002c960` (chargement) et `0x14002e2b1` (validation).
- **Bloc 0x88c :** `options::FIELDS` (titre, section, offset, type) ;
  script `grid2` : la fonction de rafraîchissement de la grille
  (`0x1400f9940..0x1400fb400`) lit chaque propriété dans le document, juste
  avant de charger le membre de la grille.
- **Objets libres :** traits et flèches rendus et créés ; classes UML
  et liens UML laissés à la vue UML (jalon 4).
- **Références :** 417 documents dans `samples/ref/` (cas c33–c41 et
  variantes `.fk-<n>`).
- **Exports VBA et XML (jalon 2.6) :** `sql::script_file`, `sql::xml`,
  `sql::to_latin1_crlf`. Références `samples/ref/*.vba` / `*.xml` produites
  par `tools/wine/make-export-refs.sh`, qui pilote le dialogue d'enregistrement
  de Looping via `tools/wine/export.py`.
- **Index :** Looping 4.1 n'en émet pas dans le script LDD (recherche bornée
  aux chaînes et à l'expérience c33, retirée depuis).
- **REA et Ghidra :** REA est configuré pour Claude Code (serveur MCP `rea`,
  skill `reverse-engineer-anything`) avec Ghidra 12.1.4 et le JDK 25 dans
  `~/.local/opt/`. REA 6.7.0 refuse Ghidra 12.2, également installé à côté
  pour un usage direct (`analyzeHeadless` de `tools/re/`).

## Rétro-ingénierie : outils et méthode

Sur le Mac : REA (serveur MCP) avec Ghidra 12.1.4, et `llvm-objdump`.
Sous Linux, on peut tout refaire avec `binutils` (`objdump`) et Ghidra.

**Tout est déjà produit dans `re-cache/`** (ignoré par git, ≈ 190 Mo) :
`gproj/` (projet Ghidra analysé), `dump/decomp.c` et `dump/symbols.tsv`,
`d5.c` (dump nettoyé), `looping.asm` et `looping_s.asm` (désassemblage
annoté), plus des extraits `*.c` de fonctions lues. Ne refaire les étapes
1 à 3 que si `re-cache/` a disparu. Ghidra refuse un chemin dont un
élément commence par un point (`~/.cache`, `.re`…).

1. Dump complet par Ghidra headless, environ 25 minutes (JDK :
   `JAVA_HOME=~/.local/opt/jdk-25.0.5+8`) :
   ```sh
   ~/.local/opt/ghidra_12.1.4_PUBLIC/support/analyzeHeadless re-cache/gproj looping \
     -import Looping.exe -overwrite -scriptPath tools/re -postScript DumpAll.java \
     "$PWD/re-cache/dump" -max-cpu 16
   ```
   Il produit `re-cache/dump/decomp.c` (≈18 Mo, toutes les fonctions) et `symbols.tsv`
   (avec les vtables RTTI, par exemple `CDrawEntite::vftable`).
2. Nettoyage du dump :
   ```sh
   cd tools/re
   sed -e 's/FUN_14013dc10/AR_Read/g; s/FUN_14013e024/AR_Write/g; s/FUN_14013d304/AR_WriteStrLen/g; s/FUN_140024c10/AR_ReadCString/g; s/FUN_140025930/AR_ReadInt/g; s/FUN_14013d6dc/AR_Fill/g; s/FUN_14013d890/AR_Flush/g; s/FUN_14013e440/AfxThrowArchiveException/g; s/FUN_14000df50(param_2 + 3)/ARNAME/g; s/FUN_1400229d0/AR_ReadLogBrush12/g' ../../re-cache/dump/decomp.c \
     | python3 simp.py | python3 strres.py | python3 clean.py > ../../re-cache/d5.c
   python3 fn.py ../../re-cache/d5.c 1400c9fb0 | python3 clean2.py   # une fonction, nettoyée
   python3 callers.py up 14010bb90                        # appelants
   ```
   `strres.py` annote les `DAT_…` avec la chaîne pointée.
   `clean.py` et `clean2.py` retirent le comptage de références des `CString` MFC
   et renomment les primitives (`CStr_Assign`, `CStr_Concat`…).
3. Désassemblage annoté. Il est indispensable quand Ghidra perd un argument
   chaîne (`CStr_AssignSz(&x)` sans second paramètre) :
   ```sh
   objdump -d --no-show-raw-insn -M intel Looping.exe > re-cache/looping.asm
   python3 tools/re/asmann.py re-cache/looping.asm re-cache/looping_s.asm   # ajoute "; 'chaîne'" après chaque lea
   ```
   Pour lister les littéraux d'une fonction dans l'ordre, filtrer `looping_s.asm`
   sur l'intervalle d'adresses de la fonction.
4. Divers :
   - `pe.py` convertit offset ↔ VA (base 0x140000000), `rtc.py` liste les
     `CRuntimeClass` ; ces deux scripts utilisent `$LOOPING_EXE`, ou à défaut `../../Looping.exe` ;
   - `loo_proto.py` est le prototype Python du parseur ;
   - `reserved.txt` contient les 301 mots réservés SQL ;
   - `fstart.py ADRESSE…` donne le début de la fonction qui contient une
     adresse (d'après `re-cache/looping.asm`) ; `dlg.py ID…` liste les
     contrôles d'un dialogue (`RT_DIALOG`) ; `vtable.py` demande `pefile`.

Astuces :
- **Résolution des vtables :** une méthode virtuelle `(*obj + 0x228)` se résout en
  lisant la vtable de la classe dans `symbols.tsv`, puis l'entrée à l'offset voulu
  (voir `pe.py`).
- **Gros résultats REA :** les réponses REA volumineuses (pseudo-code des grosses
  fonctions) saturent le contexte ; le dump local est plus pratique.

### Repères dans le binaire (Looping.exe x64)

| Adresse | Rôle |
|---|---|
| `0x140074c20` | `CLoopingDoc::Serialize` |
| `0x1400b2980` / `0x1400b3840` | `CDrawObj::Serialize` / passe des liens |
| `0x1400a6800` | `CUnParametre::Serialize` (rubrique, cardinalité) |
| `0x1400a7f70` | type d'une rubrique → SQL (`param_4` = SGBD) |
| `0x1400b60a0` / `0x1400b6260` / `0x1400b6530` | couleurs : trait, fond 1, fond 2 (vtable +0x220/+0x228/+0x230) |
| `0x1400c4540` / `0x1400ddf80` | dessin d'une entité / d'une association |
| `0x140066620` | génération globale du script SQL, du MLD textuel et du VBA |
| `0x1401134e0` | construction du modèle MLD (tables de 0x240 octets, contenu de 0x220 octets à 8 `CPtrList`) |
| `0x1400c9fb0` | **à lire** : remplissage d'une table depuis une entité (clés, migration des FK) |
| `0x1400e2810` | **à lire** : idem depuis une association |
| `0x14010bb90` | émission des contraintes d'une table (PK, FK, CONSTRAINT, CASCADE, XML) |
| `0x14010b730` | émission `ALTER TABLE` |
| `0x140111ec0` | émission des colonnes (` NOT NULL`, ` UNIQUE`) |
| `0x140104340` → `0x140103be0` | conversion des noms selon la convention |
| `0x1400c69e0` | nom d'une FK : nom de la clé référencée, gabarits `[Classe]` et `[FK]`, rôle |
| `0x140078640`… `0x14007d8e0` | lecture/écriture des options du document (offset ↔ libellé) |

## Pièges connus

- **Toshy (raccourcis façon macOS) avec un clavier AZERTY :** Toshy raisonne
  en touches physiques ; sans sa correction de disposition, Cmd+« A » devient
  Cmd+Q → Alt+F4 et ferme la fenêtre. Corrigé sur ce poste par
  `correction_enabled = True` dans `~/.config/toshy/toshy_config.py`.

- **Bloc de paramètres :** les offsets de `Settings::i32()` sont des offsets dans
  `CLoopingDoc` ; le bloc commence à 0x36c.
- **Polices des textes :** un `CDrawTexte` lit 100 puis 92 octets qui se
  chevauchent en mémoire (`int` + `LOGFONTW`). Les blocs sont conservés bruts
  pour garantir une réécriture à l'identique.
- **Associations :** ce sont des **rectangles arrondis** avec un bandeau, pas des ellipses.
  Bleu si elles génèrent une table, cyan sinon (`relation_is_functional`).
- **egui 0.37 :** utiliser `App::ui` et `Panel::*.show()` (pas `show_inside`).
  Ne pas appeler `ctx.egui_wants_keyboard_input()` dans `ctx.input(|i| …)`,
  ce qui provoque un interblocage.

## Priorité actuelle : parité fonctionnelle (jalon 5)

La fidélité visuelle (jalon 4) est en pause : d'abord rendre toutes les
fonctions de Looping accessibles, sans reproduire la disposition de
l'interface. Inventaire et état fonction par fonction :
[docs/FONCTIONS.md](docs/FONCTIONS.md) (relevé dans le ruban `RT_RIBBON`
307, les menus et les 75 dialogues de `Looping.exe`) ; chaque écart
d'interface y est noté pour plus tard.

Avancement du jalon 5 :
- **Étape 1 (édition des objets du MCD) :** `crates/rollercoaster/src/dialogs.rs`,
  une fenêtre par classe de dialogue de Looping (correspondance champ ↔
  offset en tête du fichier, relue dans `DoDataExchange` (vtable +0x200) et
  les fonctions d'enregistrement des dialogues `C…Dlg`). Double-clic ou
  « Propriétés… » ; « OK » applique en une étape d'annulation. Outils
  Héritage, Contrainte, CIF, Règle ; l'outil Lien choisit le type selon les
  extrémités (`App::link_kind`).
- **Établi sous Wine :** nom logique (`CDrawTexte+0x5f8`,
  `CUnParametre+0x18`) émis tel quel (marqueur `mld::LOGICAL`, cas c44) ;
  type libre en `+0x40` avec description après `#` ; liens de contrainte et
  de règle (cas c45).
- **Étape 2 (interactions) :** `edit::{resize, add_break, remove_break,
  copy, paste, reorder, align, move_text}` (testés) ; sélection multiple
  (`App::selection`), poignées, points de cassure, textes déplaçables dans
  `canvas.rs` ; cas c46 rouvert par Looping.
- **Étape 3 (illustrations) :** outils Texte, Ligne, Flèche, Rectangle,
  Ellipse, Crayon, Image, Sous-modèle (menu « Illustration » de la barre
  d'outils) ; `edit::{add_rectangle, add_ellipse, add_texte, add_image,
  add_crayon, add_illustration_link, remember_style, sync_free_line}` ;
  fenêtres de propriétés dans `dialogs.rs` ; rendu `render::Shape::Figure`
  et épaisseur des lignes, libellés des liens de règle et entre
  illustrations ; images affichées (`image`, chargées par `canvas.rs`).
  Cas c48 rouvert et exporté par Looping. Champs et offsets dans
  `docs/FORMAT.md`.
- **Étape 4 (affichage, en partie) :** `render::Display` (rubriques,
  types, illustrations, vues actives) et `render::build_with` ; menus
  Affichage et Vues, curseur de zoom ; `edit::{grid_step, snap, view_bit,
  set_views, add_view, remove_view}` ; flèches DF. Cas c49 rouvert et
  exporté par Looping. Reste : vues graphiques MLD et UML.
- **Outils RE ajoutés :** `tools/re/msgmap.py ID…` (gestionnaires
  `WM_COMMAND` / `UPDATE_COMMAND_UI` d'une commande), `re-cache/ribbon.xml`
  (ruban extrait).
- **Dump Ghidra :** gardé dans `re-cache/` (ignoré par git, voir
  « Rétro-ingénierie » plus bas) ; à ne pas refaire.
- Variables de débogage de l'application :
  - `ROLLERCOASTER_TRACE=1` : sur stderr, touches reçues, demandes de
    fermeture, paniques avec trace, et à chaque changement l'outil, la
    sélection, le nombre d'objets, les dialogues, les glissements et le MLD ;
    c'est ce qui a permis de déboguer les essais à la main ;
  - `ROLLERCOASTER_DIALOG=<id>` ouvre un dialogue, `ROLLERCOASTER_SELECT=<id,id…>`
    sélectionne, `ROLLERCOASTER_KEYS=ctrl+a,ctrl+c` simule des touches (une par
    image à partir de la 10e), avec `ROLLERCOASTER_SCREENSHOT`.
- Corrigé après les essais à la main : débordement de pile du MLD textuel
  quand une entité est surtype et sous-type du même héritage (garde contre
  les cycles, test `inheritance_cycle_does_not_overflow`) ; liens en double
  refusés (sauf entité–association) ; surtype sans identifiant : le sous-type
  garde sa propre `***CLE_ABSENTE***`, colonnes homonymes regroupées par
  position dans le MLD (cas c47, identique à Looping).

## Version 0.1.0 (en cours)

Périmètre et ordre de travail : `docs/ROADMAP.md`, section « Version
0.1.0 » ; tout le reste y est listé comme reporté. Avancement : points 1 à
9 faits (export PNG : `Scene::to_png`, option `png` de `looping-core`) ;
suivent la campagne de vérification (point 10) puis la release (CI et
`.dmg`, point 11).

## Choix faits (jalon 5)

Décisions prises en cours de route, à revoir si besoin :

- **Dépendance `image`** (png, jpeg, gif, bmp, sans les formats par
  défaut) dans `rollercoaster`, pour afficher les images des documents.
- **Cache de rétro-ingénierie** dans `re-cache/` (ignoré par git) plutôt
  que dans `/tmp` (effacé au redémarrage) ; scripts réutilisables dans
  `tools/re/`.
- **Images** : le chemin est gardé tel que choisi (absolu, comme Looping) ;
  à l'affichage, il est cherché tel quel, puis relativement au document,
  puis par son nom dans le dossier du document (chemins écrits sous
  Windows).
- **Sous-modèle** : « Ouvrir le modèle » lance une nouvelle fenêtre
  Rollercoaster (une fenêtre par document) ; Looping l'ouvre au double-clic.
- **Double-clic sur une image** : ouvre sa fenêtre de propriétés (avec
  « Parcourir… ») ; Looping propose directement un autre fichier.
- **Outil Texte** : la fenêtre « Texte » s'ouvre à la création, au lieu du
  petit dialogue 131 « Entrer le texte » ; texte initial « Texte ».
- **Cadres** : un clic sans glisser donne 120 × 60 (taille choisie ici,
  non relevée dans Looping).
- **Lignes et flèches** : un clic par point, double-clic ou Entrée pour
  finir, Échap pour annuler ; extrémités non rattachées à un objet.
- **Noms internes** des illustrations (« Texte N », « N » pour une image) :
  compteur = nombre d'objets de la même classe + 1 (Looping garde des
  compteurs non enregistrés, `CLoopingDoc+0x18cc…`).
- **Lien entre illustrations** : son paramètre de rôle 6 n'est créé qu'à la
  validation de sa fenêtre, comme le fait le dialogue de Looping.
- **Police d'un texte** : nom, taille, gras, italique, souligné saisis à la
  main (pas de sélecteur de police système) ; écrits dans le `LOGFONTW` du
  texte, initialisé depuis la police « Textes » (`+0x3b0`).
- **Libellés des liens de règle / entre illustrations** : dessinés au
  milieu du premier segment ; `TP 1` passe de 2,89 % à 3,04 % de pixels
  différents (textes dessinés en plus, jamais au pixel près).
- **Cases d'affichage** (rubriques, types, illustrations) et **magnétisme** :
  modifiés sans étape d'annulation (Looping ne les annule pas), mais le
  document est marqué modifié ; ils sont enregistrés avec lui.
- **Zoom** : rétabli à l'ouverture depuis `frame_state[0]`, le diagramme
  placé en haut à gauche (la position de défilement n'est pas enregistrée
  de façon exploitable) ; écrit à l'enregistrement.
- **Types affichés** : largeur ajoutée à l'entité = type le plus large + 15,
  centrée (relevé sur `c49`) ; les associations ne s'élargissent pas.
- **Flèches DF** : la patte sans flèche est la première 1,1, sinon la
  première de maximum 1 (règle déduite de `c05` et `c13`).
- **Vues** : gérées dans une petite fenêtre « Vues » (ajouter, renommer,
  supprimer) en attendant l'Urbanisation (étape 8) ; jusqu'à 20 vues
  (bits 1 à 20) ; supprimer une vue retire son bit des objets, sauf des
  liens marqués pour toutes les vues (-1).
- **Magnétisme** : appliqué à la fin d'un déplacement seulement (pas à la
  création ni au redimensionnement).
- **Agrandissement automatique** (`edit::fit_all`) appliqué à chaque
  rafraîchissement de l'application et avant les exports d'image de
  `looping-cli`, comme Looping à l'ouverture : un document ouvert puis
  enregistré peut avoir des boîtes agrandies.
- **« Récupérer les propriétés d'un modèle existant »** : tout le document
  choisi sauf ses objets et ses vues (règle déduite de `0x14006e770`, non
  vérifiée sous Wine).
- **Rôles** : la cardinalité remonte d'une ligne et le rôle s'écrit
  dessous ; position approchée.
- **Écarts d'affichage acceptés** (jalon 4) : pointe de flèche ouverte,
  taille et forme de pointe sans effet ; rectangle arrondi en pointillé
  tracé plein à l'écran ; icône de vignette absente ; texte d'image sans
  fichier non coupé au cadre (cas c48 dans `KNOWN` du test d'images).

## Jalon 4 (en pause) — fidélité de l'affichage

- **Oracle image :** `tools/wine/make-image-refs.sh [FICHIERS]` produit
  `samples/ref/<stem>.png` avec la commande « Exporter le modèle au format
  image » (33136, `0x140087b20`) ; `export.py` choisit le type PNG dans
  `cmb1`, le préfixe Wine remplace Calibri par Carlito. Test
  `looping-cli/tests/images.rs` : pixels sans couleur proche dans leur
  voisinage 3 × 3 (dans les deux sens), ≤ 0,5 % pour les exemples, ≤ 1,2 %
  pour les cas ; liste `KNOWN` des documents qui utilisent ce qui n'est pas
  encore reproduit. Tous les exemples d'étudiants sauf `TP 1` sont à
  0,14–0,47 %.
- **Métriques :** `looping_core::text` (Carlito embarqué, licence OFL dans
  `assets/fonts/`). `tmHeight` = round(winAscent·h/upm) + round(winDescent·h/upm),
  avances arrondies par glyphe ; vérifié contre GDI sous Wine
  (`tools/wine/metrics.py`). Taille dessinée : `lfHeight × 96/72 × 0.78 ×
  100/96` en `f32` puis tronquée (`0x140012dc0`) ; -12 → 13 px.
- **Mise en page (`render.rs`) :** rectangle de texte calculé par
  `CDrawTexte` +0x120 (`0x1400e9670`) : ancre et alignements du propriétaire
  (vtable +0x1c0), décalage `+0x458/+0x45c`, taille (largeur + 2, lignes ×
  hauteur). Six rectangles = 3 vues × (avec/sans types) ; la vue MCD sans
  types est le 4e (`+0x1f8`).
  - entité : nom centré dans le bandeau `+0x530` ; rubriques à gauche + 5
    (`doc+0x308`), sous le bandeau + 1, `TA_BOTTOM` ligne par ligne ;
    gras pour l'identifiant (souligné à bas − 2) et selon « Usage du gras »
    `+0x7ec` (UNIQUE `+0x8c` / NOT NULL `+0x88`).
  - association : boîte aux dimensions stockées ; nom au milieu du bandeau ;
    rubriques alignées à gauche + 10 (2 × `doc+0x308`), sous le bandeau + 1.
  - cardinalité (`0x14009cf30`, vue MCD) : centrée au milieu du premier
    segment côté entité (le suivant s'il fait ≤ 40), points découpés par les
    deux boîtes ; texte transparent, la patte est interrompue sous lui ;
    identification relative selon `+0xa00` (0 `1,1(R)`, 1 `(1,1)`, 2 souligné).
  - pattes tracées de centre à centre (`0x1400c5910`), objets dessinés dans
    l'ordre du document : les boîtes dessinées ensuite cachent les bouts.
  - ombres d'abord : entités `+0x550`/`+0x554` décalées de profondeur + 1,
    associations `+0x558`/`+0x55c` décalées de la profondeur, images
    `+0x3b0`/`+0x3b4` si remplies (`+0x310`).
  - image absente : fond, cadre, « Veuillez sélectionner une image » (police
    `+0x3b0`) au centre − 10 (`0x1400d2d70`). Valeurs par défaut des images
    d'un ancien format : largeur 1, ombre 5, `0xc0c0c0` (`0x1400d2250`).
  - entité de modèle externe : trait pointillé `PS_DOT` (`0x1400b48f0`).
  - l'export commence au plus à l'origine de la page (objets en coordonnées
    négatives coupés), marge de 5.
- **Corrigé au passage :** `Texte::default()` met l'entier `+0x390` à 1
  (police du document), comme dans les fichiers de Looping ; les cas
  synthétiques ont été régénérés.
- **Reste :** règles (pentagone à coin replié `doc+0x304`, coin `0xc4c4c4`,
  `0x1400d8ad0`), liens de règle avec libellés, rectangles et autres objets
  libres ; symboles d'héritage (triangle) et de CIF ; rôles sur les pattes ;
  répartition des pattes réflexives (`CRattach`) ; agrandissement des
  entités trop petites à l'ouverture (cas c10, c11, c14, c20) ; notations
  alternatives ; vues MLD et UML ; affichage des fichiers image dans
  l'application.
