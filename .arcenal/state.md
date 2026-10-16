# État du projet

Statut : ACTUEL

## Lot actif

- Candidate applicative 0.2.1 : fonctions métier développées ; recette client à effectuer.

## Dernière étape validée

- Socle 0.1.x affiché sur YunoHost (capture client).
- Carte blanche client pour développer la première version opérationnelle.

## Fichiers importants modifiés

- app/src/{main,model,db,pdf,web,mail,tests}.rs et style.css
- app/Cargo.toml et Cargo.lock
- manifest.toml, conf/, scripts/, .github/workflows/release.yml
- tests/test_{http,mailbox,package}.py
- doc/, README.md, WORKFLOWS.md et TEST_REPORT.md

## Tests exécutés

- Tests Rust avec Poppler, Clippy strict et formatage.
- Parcours HTTP réel : import, correction, validation, redémarrage, annulation.
- Snapshot SQLite WAL sans brouillons ni PDF.
- Rejets e-mail et vérifications DKIM cryptographiques avec DNS simulé.
- CI v0.2.1 réussie : 20 tests, compilation Linux et démarrage dans Debian 12.
- Voir TEST_REPORT.md pour les preuves et la recette non exécutée.

## Résultats

- Interface métier, filtres, imports avec validation transactionnelle et
  comparaison des doublons, suppression définitive, réglages, confirmations,
  rappels et journal paginé implémentés.
- Relève e-mail optionnelle : nécessite les identifiants IMAP du serveur.
- Les données 0.1.x sont migrées, les PDF ne sont jamais sauvegardés.

## Blocages

- Aucun PDF E-Permit réel fourni ; parseur testé sur documents synthétiques.
- Accès serveur indisponible : LDAP, IMAP, SMTP et permissions à vérifier en recette.

## Décisions récentes

- Voir .arcenal/decisions.md : groupes, cache, DKIM, confirmations et planification.

## Prochaine action

- Mettre à jour l'installation client vers 0.2.1~ynh2 et réaliser la recette documentée (PDF réel, messagerie, permissions et sauvegarde).
