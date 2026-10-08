# Architecture - Permit Alert V1

## Choix valide

Permit Alert est un monolithe Rust empaquete pour YunoHost. Un unique service
applicatif fournit l'interface web, le traitement des imports, les alertes et
la reception e-mail. Aucun microservice ni base de donnees externe n'est requis.

## Composants

### Service Rust

Le service gere :

- les ecrans de consultation, import, validation et reglages ;
- l'autorisation des coordinateurs ;
- l'extraction et la validation des PDF E-Permit ;
- la suppression des PDF apres validation, rejet ou echec ;
- le traitement des e-mails recus ;
- le calcul et l'envoi des alertes ;
- les journaux applicatifs et les echecs d'envoi.

Le service fonctionne sous systemd avec une tache planifiee pour les alertes et
le passage automatique des permis a l'etat expire.

### Permission YunoHost

L'application utilise une permission YunoHost attribuee au groupe LDAP
`coordinateur`. Elle ne gere pas de comptes ni de mots de passe propres.

### Base locale

SQLite conserve localement :

- les permis et leurs donnees extraites ;
- les reglages d'alerte ;
- les destinataires et leur etat de confirmation ;
- les validations d'import en attente ;
- les journaux de rejet et les echecs d'envoi.

Les PDF ne sont pas stockes dans SQLite. Ils n'existent que dans un espace
temporaire pendant le traitement et la validation.

### Traitement PDF

Le service accepte uniquement les PDF E-Permit. Il verifie la structure
attendue, extrait les lignes de permis et les presente pour validation. Un PDF
illisible ou de format inattendu est rejete, journalise puis supprime.

### Messagerie

La boite `epermit@onyx-ingenierie.com` alimente les imports e-mail. Le service
traite seulement les expediteurs correspondant a un coordinateur LDAP et dont
l'authentification de domaine est valide. Les autres messages sont journalises
sans conserver leur piece jointe.

Les alertes et les e-mails de confirmation sont envoyes par la messagerie
YunoHost. Les secrets eventuellement necessaires a la releve de boite sont
conserves hors de la base, dans la configuration privee geree par le paquet.

## Flux principaux

### Import web

1. Un coordinateur importe un PDF E-Permit.
2. Le service extrait les permis dans un espace temporaire.
3. Le coordinateur corrige et valide les donnees.
4. La base cree ou met a jour le permis par son numero.
5. Le PDF temporaire est supprime.

### Import e-mail

1. Le service releve la boite dediee.
2. Il verifie l'expediteur et l'authentification du domaine.
3. Il traite le PDF comme un import web ou journalise son rejet.
4. Toute piece jointe traitee ou rejetee est supprimee.

### Alertes

1. La tache planifiee recherche les permis proches de leur date de fin.
2. Elle applique le delai et la frequence configures.
3. Elle inscrit les alertes dans l'interface et envoie les e-mails.
4. A la date de fin, elle marque le permis expire et arrete les rappels.

## Sauvegarde et rollback

Le paquet YunoHost sauvegarde la base SQLite et la configuration privee utile.
Les PDF ne sont ni sauvegardes ni restaures. Une restauration remet les permis,
reglages, destinataires et journaux. La suppression du paquet doit retirer le
service et ses permissions selon le cycle de vie YunoHost, avec confirmation
explicite avant toute destruction de donnees.

## Securite

- Acces web limite a la permission YunoHost du groupe `coordinateur`.
- PDF limites au traitement temporaire et supprimes selon le CDC.
- E-mails recus controles par expediteur et authentification de domaine.
- Adresses externes soumises a confirmation avant notification.
- Secrets exclus des journaux, de SQLite et des sauvegardes non privees.

## Decisions a prendre pendant l'implementation

- Mecanisme YunoHost exact de releve de la boite dediee.
- Limites de taille PDF et controles de contenu.
- Dependances Debian necessaires a l'extraction textuelle E-Permit.
