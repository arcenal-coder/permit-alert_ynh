# Decisions

## ACTUEL

- Cible : application web empaquetee pour YunoHost, developpee en Rust.
- Acces : permission YunoHost ; groupe reel du serveur `coordinateurs` (pluriel), choisi dans le formulaire.
- Reception : `epermit@onyx-ingenierie.com`.
- Sources : PDF E-Permit uniquement, par interface ou e-mail autorise.
- Conservation : PDF supprime apres validation, rejet ou echec ; jamais sauvegarde.
- Alertes : delai et frequence configurables, interface et e-mail, jusqu'a la
  date de fin.
- Destinataires : internes ou externes ; les adresses externes confirment leur
  inscription.
- Donnees : mise a jour par numero de permis ; suppression definitive possible.
- Journal : e-mails rejetes conserves sans purge, sans contenu de PDF.
- Architecture : monolithe Rust unique avec SQLite locale et tache planifiee.
- Autorisation : permission YunoHost attribuee au groupe choisi ; route de confirmation seule publique.
- Planification : boucle Tokio chaque minute dans le service unique, pour eviter les traitements concurrents d'un cron separe.
- PDF : Poppler local, 10 Mo, 50 pages, 200 permis ; formats libelles ou tableaux non ambigus. Pas d'OCR.
- Cache PDF prive hors sauvegarde ; conservation pendant validation et au plus 24 h, expiration journalisee.
- Reception : IMAP TLS/UIDPLUS, LDAP configurable et DKIM complet strictement aligne. Les en-tetes Authentication-Results ne sont pas une preuve.
- Parametrage in situ (demande client) : ecran Connecteurs pour envoi local/SMTP TLS, IMAP TLS et LDAP/LDAPS, sans identifiants preconfigures dans le depot.
- Secrets hors SQLite dans data_dir/private/connectors.json (0700/0600), necessaire pour ecrire depuis le service sans rendre le repertoire d'installation modifiable. Sauvegarde privee et restauration prevues.
- mail.env conserve comme amorcage compatible 0.2 ; apres enregistrement web, le fichier prive fait autorite. Modification a chaud, sans redemarrage.
- Diagnostics explicites sans envoi SMTP ni traitement IMAP ; secrets conserves uniquement pour le meme serveur et compte, effacement explicite et detection de modification concurrente.
- Destinataires : confirmation pour toutes les adresses, lien valable 7 jours ; simplification uniforme pour internes et externes.

## A REVALIDER

- Conformite du parseur au format PDF E-Permit reel du client, encore non fourni.
- Integration LDAP, IMAP et delivrabilite sur le serveur client.
