# Rapport de validation — candidate 0.1.1

## RÉUSSI — contrôles locaux

- Formatage Rust : `cargo fmt --check`.
- Analyse Rust : `cargo clippy -- -D warnings`.
- Création des tables SQLite : `cargo test` (1 test).
- Métadonnées indispensables au formulaire YunoHost : `python3 tests/test_package.py`.
- Sauvegarde cohérente d'une base SQLite ouverte en mode WAL et relecture des
  données validées : même suite Python (2 tests au total).
- Syntaxe des scripts de cycle de vie : `bash -n`.
- Espaces et conflits de patch : `git diff --check`.

## Cause du formulaire figé

La réponse API fournie par le client ne contient pas `upstream`. Le composant
officiel `AppInstall.vue` accède à `manifest.upstream.demo` et
`manifest.upstream.license` sans tester l'existence de l'objet. La section a été
ajoutée au manifeste. Le formulaire sur le serveur client reste à vérifier.

Les exigences API ont toutes `passed: true`, notamment `install` : le message
générique « already installed » ne permet pas de conclure à une installation
existante. Aucune suppression de l'application n'est justifiée par ces données.

## NON_EXÉCUTÉ — à ce stade

- Exécution du nouveau binaire Linux dans Debian 12 (prévue par le workflow).
- Installation, retrait, réinstallation et mise à jour dans YunoHost.
- Sauvegarde/restauration complète du paquet par YunoHost.
- Contrôle des accès coordinateurs et utilisateurs hors groupe sur le serveur.
- Rechargement du formulaire et accès HTTP réel derrière Nginx/SSOwat.
- Recette métier PDF/e-mails/alertes : ces fonctions ne sont pas implémentées.

## Statut de livraison

Candidate technique du socle uniquement. La compilation et les tests locaux ne
constituent pas une validation de production de la V1 métier.

## Retour arrière

Avant une mise à jour sur un serveur, créer une sauvegarde YunoHost. En cas
d'échec, restaurer cette sauvegarde via YunoHost. Ne pas purger les données pour
résoudre un formulaire qui ne se charge pas.
