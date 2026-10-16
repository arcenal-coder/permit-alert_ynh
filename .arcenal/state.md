# État du projet

Statut : ACTUEL

## Lot actif

- Candidate locale 0.3.0 : connecteurs préparés pour paramétrage in situ ; non publiée.

## Dernière étape validée

- Socle 0.1.x affiché sur YunoHost (capture client).
- Carte blanche client pour développer la première version opérationnelle.
- Demande client : préparer les connecteurs ; les paramètres réels seront renseignés sur place.

## Fichiers importants modifiés

- app/src/{main,model,db,pdf,web,mail,tests}.rs et style.css
- app/src/{connectors,connector_web}.rs
- app/Cargo.toml et Cargo.lock
- manifest.toml, conf/, scripts/, .github/workflows/release.yml
- tests/test_{http,mailbox,package}.py
- scripts/{connector_io,connectors}.py et tests/test_connectors.py
- doc/, README.md, WORKFLOWS.md et TEST_REPORT.md

## Tests exécutés

- Tests Rust avec Poppler, Clippy strict et formatage.
- Parcours HTTP réel : import, correction, validation, redémarrage, annulation.
- Snapshot SQLite WAL sans brouillons ni PDF.
- Rejets e-mail et vérifications DKIM cryptographiques avec DNS simulé.
- CI v0.2.1 réussie : 20 tests, compilation Linux et démarrage dans Debian 12.
- Paquet final v0.2.1-ynh2 publié, CI également réussie ; branche master des deux dépôts synchronisée.
- Voir TEST_REPORT.md pour les preuves et la recette non exécutée.
- Candidate 0.3 : 31 tests locaux réussis, Clippy strict, formatage, compilation
  et parcours HTTP avec rechargement des connecteurs après redémarrage.

## Résultats

- Interface métier, filtres, imports avec validation transactionnelle et
  comparaison des doublons, suppression définitive, réglages, confirmations,
  rappels et journal paginé implémentés.
- Relève e-mail optionnelle : nécessite les identifiants IMAP du serveur.
- Les données 0.1.x sont migrées, les PDF ne sont jamais sauvegardés.
- Écran Connecteurs : envoi local/SMTP TLS, IMAP TLS et LDAP/LDAPS, diagnostics
  sans envoi ni traitement des messages et activation explicite.
- Secrets hors SQLite dans data_dir/private/connectors.json (0700/0600),
  sauvegarde/restauration prévues ; aucun identifiant réel ajouté au dépôt.

## Blocages

- Aucun PDF E-Permit réel fourni ; parseur testé sur documents synthétiques.
- Accès serveur indisponible : LDAP, IMAP, SMTP et permissions à vérifier en recette.

## Décisions récentes

- Voir .arcenal/decisions.md : groupes, cache, DKIM, confirmations et planification.

## Prochaine action

- Produire une release 0.3 et mettre à jour l'URL/empreinte du manifeste avant
  publication ; renseigner ensuite les connecteurs et réaliser les diagnostics in situ.
