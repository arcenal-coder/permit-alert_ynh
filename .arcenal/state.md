# État du projet

Statut : ACTUEL

## Lot actif

- Aucun lot validé.

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
- config/permit-alert.service
- manifest.toml
- conf/
- scripts/
- .github/workflows/release.yml
- README.md

## Tests exécutés

- Controle documentaire : `arcenal-project-state validate` reussi.
- `cargo fmt --check` reussi.
- `cargo clippy -- -D warnings` reussi.
- `cargo test` reussi : 1 test.
- Syntaxe Bash des scripts YunoHost verifiee avec `bash -n`.

## Résultats

- Le socle Rust initialise SQLite et expose une page d'accueil et une sonde de
  sante. Le paquet YunoHost v2 prepare un service, un proxy, une permission et
  la sauvegarde des donnees. Les imports et alertes ne sont pas implementes.

## Blocages

- Instance YunoHost de test requise pour valider le Lot 1.
- Instance YunoHost amd64 de test requise pour valider le Lot 1.

## Décisions récentes

- Voir .arcenal/decisions.md.

## Prochaine action

- Autoriser le Lot 1 - Socle YunoHost avant tout developpement.
