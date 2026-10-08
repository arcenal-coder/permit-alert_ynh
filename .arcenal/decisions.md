# Decisions

## ACTUEL

- Cible : application web empaquetee pour YunoHost, developpee en Rust.
- Acces : membres LDAP YunoHost du groupe `coordinateur`.
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
- Autorisation : permission YunoHost attribuee au groupe `coordinateur`.

## A REVALIDER

- Mecanisme precis de releve de la boite dediee dans YunoHost.
- Limites de taille et controles de securite pour les PDF.
