# Rollercoaster

**Rollercoaster** est une réécriture en Rust pur de [Looping](https://www.looping.fr) (modélisation
Merise MCD → MLD → SQL), à partir de la rétro-ingénierie de `Looping.exe` 4.1.
Cibles : macOS ARM (Apple Silicon) et Linux x64, deux plateformes où Looping
n'existe pas. Les sorties de référence sont produites par le vrai Looping sous Wine
([tools/wine/](tools/wine/)).

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
