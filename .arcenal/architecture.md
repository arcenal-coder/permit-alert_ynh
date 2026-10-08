# Architecture

Statut : VALIDE

## Structure

- Monolithe Rust empaquete pour YunoHost.
- Service systemd unique : interface web, imports, alertes et reception e-mail.
- SQLite locale pour les permis, reglages, destinataires et journaux.
- PDF limites a un espace temporaire puis supprimes apres traitement.

## Integrations

- Permission YunoHost attribuee au groupe LDAP `coordinateur`.
- Boite d'import : `epermit@onyx-ingenierie.com`.
- Messagerie YunoHost pour les alertes et confirmations.
- Tache planifiee pour alertes et expiration automatique.

## Cycle de vie

- Le paquet sauvegarde SQLite et sa configuration privee, sans PDF.
- La restauration restitue permis, reglages, destinataires et journaux.
- Le service, les donnees et les permissions suivent les helpers YunoHost v2.

## Points a definir pendant implementation

- Releve securisee de la boite dediee.
- Limites de taille et controles de contenu des PDF.
- Dependances Debian d'extraction E-Permit.
