# Architecture

Statut : VALIDE

## Structure

- Monolithe Rust empaquete pour YunoHost.
- Service systemd unique : interface web, imports, alertes et reception e-mail.
- SQLite locale pour les permis, reglages, destinataires et journaux.
- PDF limites a un espace temporaire puis supprimes apres traitement.

## Integrations

- Permission YunoHost attribuee au groupe LDAP choisi (`coordinateurs` sur Onyx).
- Boite d'import : `epermit@onyx-ingenierie.com`.
- Messagerie YunoHost pour les alertes et confirmations.
- Boucle Tokio chaque minute pour alertes, expiration et releve, sans cron concurrent.
- Poppler pour l'extraction ; cache prive sous /var/cache/permit-alert hors sauvegardes.
- IMAP TLS/UIDPLUS via helper Python ; LDAP local et verification DKIM alignee.

## Cycle de vie

- Le paquet sauvegarde SQLite et sa configuration privee, sans PDF.
- La restauration restitue permis, reglages, destinataires et journaux.
- Le service, les donnees et les permissions suivent les helpers YunoHost v2.

## Points restant a verifier en recette

- Parametres effectifs de la boite dediee et delivrabilite.
- Compatibilite d'un PDF E-Permit client avec le parseur.
- Cycle de vie complet du paquet et permissions sur le serveur.
