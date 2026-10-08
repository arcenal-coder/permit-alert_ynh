# Projet

## Nom

Permit Alert

## Objectif

Application web YunoHost qui importe des permis E-Permit, enregistre les
donnees validees et avertit avant leurs echeances.

## Perimetre valide

- Import PDF par interface et par e-mail.
- Validation humaine avant enregistrement.
- Alertes configurables dans l'interface et par e-mail.
- Sauvegarde des donnees, sans conservation des PDF.

## Contraintes structurantes

- Paquet YunoHost en Rust.
- Acces LDAP reserve au groupe `coordinateur`.
- Boite de reception : `epermit@onyx-ingenierie.com`.
- Base de donnees locale au serveur.

## Reference

- Cahier des charges : `cahier-des-charges.md`.
