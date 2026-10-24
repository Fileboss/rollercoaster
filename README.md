# Rollercoaster

**Rollercoaster** est une réécriture en Rust pur de [Looping](https://www.looping.fr) (modélisation
Merise MCD → MLD → SQL), à partir de la rétro-ingénierie de `Looping.exe` 4.1.
Cibles : macOS ARM (Apple Silicon) et Linux x64, deux plateformes où Looping
n'existe pas. Les sorties de référence sont produites par le vrai Looping sous Wine
([tools/wine/](tools/wine/)).

## Installation

**macOS (Apple Silicon)** : télécharger `Rollercoaster-<version>-macos-arm64.dmg`
depuis la page [Releases](https://github.com/Fileboss/rollercoaster/releases),
l'ouvrir et glisser **Rollercoaster** dans **Applications**.

L'application n'est ni signée par un compte Apple Developer ni notarisée :
au premier lancement, macOS refuse de l'ouvrir (« Apple n'a pas pu vérifier… »).
Pour l'autoriser, une seule fois :

1. ouvrir Rollercoaster, fermer le message ;
2. aller dans **Réglages Système › Confidentialité et sécurité**, en bas :
   « Rollercoaster a été bloqué… » › **Ouvrir quand même**, puis confirmer.

Jusqu'à macOS 14, un **clic droit › Ouvrir** sur l'application suffit. En
ligne de commande, l'équivalent est
`xattr -dr com.apple.quarantine /Applications/Rollercoaster.app`.

**Linux x64** : télécharger `Rollercoaster-<version>-linux-x86_64.tar.gz`,
le décompresser et lancer `./rollercoaster` (l'éditeur) ou `./looping-cli`.
Aucune dépendance à installer : X11 ou Wayland et OpenGL sont chargés au
lancement. Pas encore de paquet (AppImage, Flatpak).

## Publier une version

Monter `version` dans `Cargo.toml` (section `[workspace.package]`), mettre
à jour `docs/RELEASE-NOTES.md`, puis merger dans `main` : la CI
(`.github/workflows/release.yml`) teste, construit le `.dmg` macOS ARM et
le `.tar.gz` Linux x64, crée le tag `v<version>` et la release GitHub avec
ces deux fichiers. Un merge sans changement de version ne publie rien.
Chaque push donne aussi ces livrables, téléchargeables depuis la page de
l'exécution de la CI (14 jours).

## Utilisation

Rollercoaster ouvre et enregistre les fichiers `.loo` de Looping (Fichier ›
Ouvrir, ou `rollercoaster fichier.loo`). Le MCD se dessine avec la barre
d'outils (Entité, Association, Héritage, Contrainte, CIF, Règle, Lien) ;
double-clic sur un objet pour ses propriétés. Le **MLD textuel** et le
**script SQL** du SGBD choisi s'affichent sous le schéma, à jour à chaque
modification, identiques à ceux de Looping ; Fichier › Exporter donne le
script (TXT ou VBA pour Access), le MLD en XML et l'image du schéma (PNG,
SVG). Les options de génération sont dans le menu **Paramètres**.

## Crates

| Crate | Rôle |
|---|---|
| `loo-format` | Lecture/écriture sans perte des fichiers `.loo` (formats 1 à 51), réécriture identique à l'octet pour la version courante |
| `looping-core` | Modèle MCD, types de données par SGBD, dérivation MLD, script SQL, rendu (display list et SVG), opérations d'édition |
| `looping-cli` | `looping-cli <json\|mcd\|mld\|sql\|svg\|png\|resave> FICHIER [SORTIE] [--dbms N]` |
| `rollercoaster` | L'application Rollercoaster, éditeur graphique (egui/eframe) : `rollercoaster [fichier.loo]` |

```sh
cargo run --release -p rollercoaster -- samples/beeven.loo
cargo run -p looping-cli -- sql samples/beeven.loo --dbms 3   # PostgreSQL
cargo test                                                    # utilise samples/*.loo et samples/ref/ s'ils sont présents
tools/wine/make-refs.sh                                       # régénère samples/ref/ avec Looping sous Wine
```

Spécification du format : [docs/FORMAT.md](docs/FORMAT.md) · Feuille de route : [docs/ROADMAP.md](docs/ROADMAP.md).
