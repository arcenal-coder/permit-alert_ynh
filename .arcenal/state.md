# État du projet

Statut : ACTUEL

## Lot actif

- Lot 1 — Socle YunoHost : corrections en cours ; recette serveur non effectuée.

## Dernière étape validée

- Architecture V1 valide.

## Fichiers importants modifiés

- cahier-des-charges.md
- .arcenal/project.md
- .arcenal/decisions.md
- .arcenal/state.md
- architecture.md
- plan-de-realisation.md
- app/Cargo.toml
- app/Cargo.lock
- app/src/main.rs
- conf/systemd.service
- manifest.toml
- conf/
- scripts/
- .github/workflows/release.yml
- README.md
- TEST_REPORT.md
- tests/test_package.py
- scripts/backup-sqlite.py

## Tests exécutés

- Controle documentaire : `arcenal-project-state validate` reussi.
- `cargo fmt --check` reussi.
- `cargo clippy -- -D warnings` reussi.
- `cargo test` reussi : 1 test.
- Syntaxe Bash des scripts YunoHost verifiee avec `bash -n`.
- Regression du manifeste et sauvegarde SQLite WAL : 2 tests Python reussis.

## Résultats

- Le socle Rust initialise SQLite et expose une page d'accueil et une sonde de
  sante. Le paquet YunoHost v2 prepare un service, un proxy, une permission et
  la sauvegarde des donnees. Les imports et alertes ne sont pas implementes.

## Blocages

- Aucun acces serveur YunoHost disponible ; installation et recette non verifiees.
- Fonctions PDF, reception e-mail et alertes non implementees.

## Décisions récentes

- Voir .arcenal/decisions.md.

## Prochaine action

- Publier la candidate 0.1.1, puis verifier le formulaire et le cycle de vie sur YunoHost.
