Première version de **Rollercoaster**, réécriture en Rust de Looping 4.1 pour
macOS (Apple Silicon) et Linux x64 : concevoir un schéma de base de données
par un MCD, de bout en bout.

Fichiers : `Rollercoaster-0.1.0-macos-arm64.dmg` (l'application) et
`Rollercoaster-0.1.0-linux-x86_64.tar.gz` (l'éditeur `rollercoaster` et
l'outil en ligne de commande `looping-cli`).

- Ouvre et enregistre les fichiers `.loo` de Looping (formats 1 à 51) ;
  Looping rouvre les fichiers enregistrés.
- Édition complète du MCD : entités, rubriques et types, associations,
  cardinalités, identification relative, rôles, héritage, CIF, contraintes
  inter-associations, règles ; annuler, copier, aligner.
- MLD textuel et script SQL **identiques à Looping** pour les 9 SGBD
  (Access, MySQL, Oracle, PostgreSQL, SQL Server, Sybase, DB2, MariaDB,
  SQLite) et leurs options, messages d'erreur du MLD compris.
- Exports : script LDD (TXT, VBA pour Access), MLD XML, image PNG ou SVG.

Vérifié par plus de 600 comparaisons avec le vrai Looping (exemples,
cas synthétiques, modèles aléatoires) et une campagne de dizaines de
milliers d'opérations d'édition sans panique.

Reporté : vues graphiques MLD et UML, impression, urbanisation,
rétroconception SQL, paquet Linux AppImage / Flatpak (voir `docs/ROADMAP.md`).

**Installation** : l'application n'est pas notarisée ; au premier lancement,
l'autoriser dans Réglages Système › Confidentialité et sécurité ›
« Ouvrir quand même » (voir le README).
