# Inventaire des fonctions de Looping 4.1

Référence pour la **parité fonctionnelle** de Rollercoaster : toutes les
fonctions de Looping doivent être accessibles, de façon logique. La forme et
l'emplacement des boutons, panneaux et dialogues peuvent différer ; chaque
écart est noté dans la colonne « Écart d'interface » pour être traité plus
tard (fidélité visuelle, jalon 4).

Sources, relues dans `Looping.exe` :

* ruban : ressource `RT_RIBBON` 307 (XML MFC), libellés et descriptions
  dans la table de chaînes (n° de commande = n° de chaîne) ;
* menus : ressources `RT_MENU` 119–201 ; tables des messages `WM_COMMAND`
  (gestionnaires notés quand utiles) ;
* dialogues : ressources `RT_DIALOG` 100–209.

Légende : ✅ fait · 🟡 partiel · ❌ absent.

Sémantique vérifiée sous Wine au passage : le **nom logique** d'une entité,
d'une association ou d'une rubrique remplace tel quel le nom dans le MLD et
le script (sans convention de nommage ni suppression des accents), y compris
dans les clés migrées et les noms de contraintes (cas c44) ; le type libre
est le texte avant `#` (`CUnParametre+0x40`, corrigé : on lisait `+0x18`, le
nom logique).

## Fichier (menu principal du ruban)

| Fonction | Looping | Rollercoaster | Écart d'interface / note |
|---|---|---|---|
| Nouveau | 57600, Ctrl+N | ✅ Fichier › Nouveau, Ctrl+N : document identique à celui de Looping ; propose de récupérer les propriétés d'un modèle existant | le modèle `Looping.loo` placé à côté de l'exécutable (repris automatiquement par Looping) n'a pas d'équivalent |
| Ouvrir… | 57601, Ctrl+O | ✅ Fichier › Ouvrir… | |
| Enregistrer | 57603, Ctrl+S | ✅ Fichier › Enregistrer, Ctrl+S | |
| Enregistrer sous… | 57604 | ✅ | |
| Fermer | 57602 | ❌ | une fenêtre = un document ; Looping est MDI |
| Fichiers récents | 57616 | ❌ | |
| Imprimer / Imprimer rapidement | 57607 (Ctrl+P) / 57608 | ❌ | |
| Aperçu avant impression | 57609 | ❌ | |
| Configuration de l'impression | 57606 | ❌ | |
| Exporter › Modèle graphique vers le presse-papiers | 33138 | ❌ | |
| Exporter › Modèle graphique au format image… | 33136 (jpeg, bmp, png, gif) | ✅ Fichier › Exporter l'image… : PNG ou SVG (`Scene::to_png`, `looping-cli png`) | jpeg, bmp, gif reportés après v0.1.0 |
| Exporter › MLD textuel au format XML… | 33187 | ✅ identique à l'octet | |
| Exporter › Script LDD au format TXT… | 33154 (VBA pour Access) | ✅ identique à l'octet | |
| Paramètres LDD › Configuration du SGBD personnalisé | 33191, dialogue 187 « SGBD personnalisé » | ❌ | type par type, « basé sur » un SGBD |
| Paramètres LDD › Types libres prédéfinis | 33194, dialogue 189 | ❌ | |
| Paramètres LDD › Compléments script LDD | 33192, dialogue 190 | ✅ Paramètres › Compléments script LDD… | |
| Urbanisation | 33200, dialogues 201 « Urbanisation », 209 | ❌ | données lues (`CUneVueUrbanisme`), pas d'interface |
| Rétroconception (SQL → MCD) | 33207, dialogue 206 | ❌ | analyseur `0x1400567d0` (jalon 6) |
| À propos | 33193, dialogue 100 | ❌ | |
| Annuler / Rétablir | 33139 / 33134 | ✅ Édition, Ctrl+Z / Ctrl+Maj+Z | |
| Quitter | 57665 | ✅ Fichier › Quitter ; fermer la fenêtre, Nouveau et Ouvrir demandent d'enregistrer un document modifié | |
| Barre d'accès rapide | Nouveau, Ouvrir, Enregistrer, Imprimer rapidement, Copier l'image, Annuler, Rétablir | ❌ | |
| Thème de l'interface | menu 200 (Office 2007 bleu/noir/argent/cyan, Windows 7) | — | non pertinent hors Windows |

## Onglet « Looping »

### Formalisme

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Vue MCD et notation : Standard / Flèche pour DF / Asso simplifiée | 33180 › 33110, 33111, 33112 (`+0x7c0`) | ✅ Affichage › Notation MCD ; flèches DF dessinées comme Looping (`c05`, `c13`) : pleines, au bord de l'entité, sur les pattes autres que la patte x,1 d'une association fonctionnelle | taille de la pointe (`+0x9f8`) approchée |
| Vue UML et notation : Standard / Simplifié | 33148 › 33113, 33114 (vue `+0x3f4`, document `+0xa80`) | ❌ | vue UML graphique absente (ruban : « ID_UML ») |
| Vue MLD et notation : Standard / Crow's foot | 32788 › 33115, 33116 (vue `+0x3f8`) | ❌ | vue MLD graphique absente (ruban : « ID_MLD ») |

### Affichage

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Afficher les rubriques | 33108 (case ; vue `+0x3ec`, document `+0x540`) | ✅ Affichage › Rubriques, enregistré avec le document | |
| Afficher les types de données | 33126 (case ; vue `+0x3f0`, document `+0x544`) | ✅ Affichage › Types de données : type SQL du SGBD du document, police « type », entité élargie à l'affichage | largeur des associations inchangée ; hauteur des entités non agrandie |
| Afficher les illustrations | 33130 (case ; vue `+0x3e8`, document `+0x748`) | ✅ Affichage › Illustrations (la vignette d'un sous-modèle reste) | |
| Zoom (curseur, 50 %, 100 %, 150 %) | 32783, 33128, 32790, 33129 (10 à 800 %, vue `+0x2f0`) | ✅ curseur 10–200 % dans la barre d'outils, Affichage › Zoom 50/100/150 %, pincement ou Ctrl+molette ; zoom enregistré avec le document et rétabli à l'ouverture | le diagramme est replacé en haut à gauche à l'ouverture |
| Ajuster à la fenêtre | 33181 | ✅ Affichage | |
| Grille : afficher, utiliser, couleur, espacement | dialogue 123 « Grille » | — | dialogue présent dans les ressources mais jamais ouvert par Looping 4.1 ; aucune grille dessinée |
| Magnétisme : grille, demi-grille, sans | 33212, 33213, 33214 (document `+0x33c`, `+0x340`, pas `+0x334`) | ✅ Affichage › Magnétisme ; les objets déplacés sont arrondis au pas le plus proche (`0x14005bdf0`) | appliqué à la fin d'un déplacement seulement |

### Pointeur

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Sélection | 32773 | ✅ outil Sélection | |
| Prof | 33195 (objet `CDrawProf`) | ❌ | rôle exact à établir (annotation ?) |

### MCD (outils de création)

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Entité | 32771 | ✅ outil Entité | |
| Association | 32772 | ✅ outil Association | |
| Héritage | 33163 | ✅ outil Héritage | |
| Contrainte inter-associations | 33164 | ✅ outil Contrainte | |
| CIF | 33165 | ✅ outil CIF | |
| Règle | 33166 | ✅ outil Règle | |
| Lien | 33173 | ✅ outil Lien : son type suit les deux extrémités (entité–association, entité–héritage, entité ou association–CIF / contrainte, objet–règle) | le premier lien d'un héritage marque le surtype |
| Cassure (point de cassure d'un lien) | 32775 | ✅ outil Cassure (clic sur un lien : ajoute ; sur un point : retire), points déplaçables à la souris | |
| Sous-modèle | 33143, dialogue 204 « Vignette » | ✅ outil Illustration › Sous-modèle : un `CDrawImage` avec `+0x3c0` = 1, modèle `+0x3b8`, vignette `+0x390` ; « Ouvrir le modèle » dans sa fenêtre | Looping ouvre le modèle au double-clic ; ici, une nouvelle fenêtre Rollercoaster. Sans vignette, Looping dessine une icône de sous-modèle (bitmap), non reproduite |

### Illustration

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Texte | 32776, dialogues 131, 171 | ✅ outil Texte (cadre tracé à la souris) : un `CDrawTitre`, ses lignes dans son texte d'emplacement 0 | Looping demande d'abord le texte (dialogue 131) ; ici la fenêtre « Texte » s'ouvre directement |
| Ligne | 33167, dialogue 185 | ✅ outil Ligne : un clic par point, double-clic ou Entrée pour finir, Échap pour annuler | extrémités non rattachées à un objet |
| Flèche | 33171, dialogue 184 | ✅ outil Flèche (comme Ligne) | pointe dessinée ouverte, sa taille et sa forme ne changent pas encore le dessin |
| Rectangle | 33168, dialogue 182 | ✅ outil Rectangle (glisser, ou clic : 120 × 60) | coins arrondis pleins même en pointillé à l'écran |
| Ellipse | 33170, dialogue 183 | ✅ outil Ellipse | |
| Crayon | 33196, dialogue 191 | ✅ outil Crayon (glisser) | |
| Image | 33172, dialogue 203 | ✅ outil Image : cadre puis choix du fichier (jpg, jpeg, bmp, png, gif) ; fichier affiché, cherché tel quel, puis relativement au document, puis par son nom dans le dossier du document | Looping change d'image au double-clic ; ici « Parcourir… » dans la fenêtre « Image ». Le texte « Veuillez sélectionner une image » n'est pas coupé au cadre |
| Lien entre illustrations | dialogue 186 | ✅ outil Lien dès qu'une extrémité est un rectangle, une ellipse, une image ou un texte (l'autre : entité, association, règle ou illustration) ; libellé dessiné | |
| Valeurs reprises d'un objet à l'autre | `+0x808..+0x934`, `+0xa90..+0xaac` du document | ✅ chaque nouvelle illustration prend les dernières valeurs validées dans la fenêtre de son type (`edit::remember_style`) | |

### Édition

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Couper / Copier / Coller | 33132 / 33109 / 33133 (format « Looping Copier Coller ») | ✅ Édition, Ctrl+X / C / V ; liens copiés si leurs deux extrémités le sont | presse-papiers interne à l'application : pas d'échange avec Looping (format Windows) |
| Supprimer | 33250, Suppr | ✅ Suppr, bouton du panneau | |
| Tout sélectionner | 33197 | ✅ Édition, Ctrl+A | |
| Inverser la sélection | 33185 | ✅ Édition | |
| Ordre des objets (premier plan, arrière-plan, avancer, reculer) | 33120, 33121, 33122, 33123, 33198, 33199 (`0x14008f220`, `0x14008f750`) | ✅ Édition › Ordre | l'ordre agit aussi sur celui des tables du MLD, comme dans Looping (cas c46) |
| Aligner à gauche / au centre / à droite / en haut / au milieu / en bas | 33155, 33157, 33158, 33159, 33160, 33161 (`0x14008ece0`) | ✅ Édition › Aligner | bords extrêmes, ou centre du premier objet sélectionné |

### Fenêtres

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Vues (nommées) | 33222 « Vue » (tous les objets), 33223–33243 (une par vue, plusieurs actives ensemble ; masque de la fenêtre `+0x130`) ; dialogue 202 « Vues » | ✅ menu Vues (tous les objets, ou une ou plusieurs vues), fenêtre « Gérer les vues… » (ajouter, renommer, supprimer) ; appartenance d'un objet dans le panneau latéral (`CDrawObj+0x338`) ; un objet créé appartient aux vues actives | Looping crée les vues dans « Urbanisation » (étape 8) et règle l'appartenance dans le dialogue de chaque objet |
| Nouvelle fenêtre / Organiser | 57648, 131 | ❌ | |
| Panneau Propriétés | 32787 (grille de propriétés) | 🟡 panneau latéral (nom, rubriques, cardinalité) + menu Paramètres (options du document) | grille de propriétés par objet absente |
| Panneau MLD textuel | 33149 | ✅ onglet « MLD textuel » | |
| Panneau Script LDD | 33147 | ✅ onglet « Script SQL » | |
| Rechercher / Suivant dans ces panneaux | 33217 (Ctrl+F) / 33218 (F3), dialogue 193 | ❌ | |
| Masquer un panneau | 33152, 33153 | ❌ | |

## Dialogues d'objets (double-clic dans Looping)

| Dialogue | Champs | Rollercoaster | Écart / note |
|---|---|---|---|
| 165 Entité | nom, nom logique, modèle externe, rubriques (ajouter, dupliquer, modifier, renommer, supprimer, monter, descendre, clé, clé auto), commentaire, classe fictive | ✅ fenêtre « Entité » (double-clic ou « Propriétés… ») | « Renommer » ouvre chez Looping le dialogue 208 ; ici une ligne « Remplacer … par … » dans la fenêtre |
| 167 Association | nom, nom logique, rubriques, commentaire, table de correspondance forcée | ✅ fenêtre « Association » | |
| 172 Rubrique | nom, nom logique, type complet (texte, numérique, date/heure, booléen, XML, géo, libre…), longueurs, collation, propriétés (NOT NULL, UNIQUE, index…), commentaire, complément | ✅ fenêtre « Rubrique » | « Index : » (case de Looping selon `+0xa1c`) non repris, « Autre index » oui |
| 173–196 Lien (variantes selon le SGBD) | cardinalité (0,1 / 1,1 / 0,n / 1,n / min-max), identifiant relatif, récup clé primaire, entité ciblée par CIF, place dans la clé primaire, rôle (libellé, afficher, suffixer, renommer), index composé, FK dans index, CASCADE / Autre, lissage cassure | ✅ fenêtre « Lien » (toutes les variantes en une) | « Construction index composé » : champ non identifié, absent |
| 174 Héritage | nom, contrainte (aucune, X, T, XT), migration de toutes les rubriques, commentaire, script SQL, position | ✅ | |
| 175, 197–199 Lien héritage | surtype, critère, options de FK, CASCADE, lissage | ✅ | paramètre de rôle 3 (`0x1400304b0`) |
| 176 Contrainte inter-associations | nom, type (X, T, XT, S, I), commentaire, script, position | ✅ | |
| 177 / 178 Liens de contrainte / de CIF | cible, libellé, lissage | ✅ | Looping ne garde qu'une cible par CIF (décoche les autres) : non reproduit |
| 179 CIF | nom, commentaire | ✅ | |
| 180 Règle | nom, description, afficher, commentaire en tête du script, script SQL, position avant/après les tables | ✅ | |
| 181 Lien règle | libellé, lissage | ✅ | |
| 171 Texte | police, alignement, contour, fond, ombre, chacun avec « Par défaut » (section « Textes » du document) | ✅ fenêtre « Texte » | police : nom, taille, gras, italique, souligné (pas de sélecteur de police système) |
| 182 / 183 / 203 Rectangle, Ellipse, Image | contour, fond, ombre | ✅ | |
| 184 / 185 / 191 Flèche, Ligne, Crayon | épaisseur, style, couleur, pointe, lissage | ✅ appliqués à tous les segments | tailles de pointe nommées (Looping les dessine) |
| 186 Lien entre illustrations | libellé, lissage | ✅ (paramètre de rôle 6) | |
| 204 Vignette (sous-modèle) | modèle, vignette, contour, fond, ombre | ✅ | |
| 208 Remplacement de caractères dans les noms de rubriques | à remplacer, par | ❌ | |
| 200 Correction (« Corriger / Ignorer ») | | ❌ | rôle à établir |

## Interactions sur le schéma

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Déplacer un objet | glisser | ✅ | |
| Redimensionner | poignées | ✅ 8 poignées sur l'objet sélectionné | |
| Sélection multiple (rectangle, Maj/Ctrl) | | ✅ rectangle de sélection, Maj/Ctrl+clic | le défilement à la souris passe au bouton du milieu ou droit (et à la molette) |
| Double-clic : dialogue de l'objet | | ✅ fenêtre de l'objet (OK applique en une étape d'annulation) | fenêtres non modales egui ; disposition des champs différente |
| Déplacer les textes (cardinalités, noms : `+0x458/+0x45c`) | | ✅ glisser un texte d'un objet sélectionné | |
| Points de cassure : créer, déplacer | | ✅ | |
| Défilement (molette), zoom (pincement, Ctrl+molette) | | ✅ | |

## Options du document (grille « Propriétés » sans sélection)

| Fonction | Looping | Rollercoaster | Écart / note |
|---|---|---|---|
| Options MLD / SQL (SGBD, nommage, contraintes, CASCADE, commentaires, accents, types dans le MLD, masque FK, identifiant automatique…) | grille de propriétés (`0x1400f9940`) | ✅ menu Paramètres | présentées en menus, pas en grille |
| Polices, couleurs, dimensions (catalogue `options::FIELDS`) | même grille ; dialogue 170 « Propriétés » | 🟡 cases booléennes seulement | polices, couleurs, entiers non modifiables |
| SGBD cible | grille | ✅ liste dans la barre d'outils | |
