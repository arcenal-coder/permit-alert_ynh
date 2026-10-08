# Cahier des charges - Permit Alert V1

## Objectif

Centraliser les permis E-Permit et alerter avant leur date de fin pour eviter
qu'une echeance ne soit oubliee.

## Faits confirmes

- L'application est un paquet web YunoHost developpe en Rust.
- L'acces est reserve au groupe LDAP YunoHost `coordinateur`.
- La base de donnees est locale au serveur YunoHost.
- Les PDF proviennent uniquement d'E-Permit.
- Les sources sont importees par interface ou recues a
  `epermit@onyx-ingenierie.com`.

## Perimetre V1

- Importer et analyser un PDF E-Permit.
- Afficher les donnees extraites pour correction et validation.
- Enregistrer et mettre a jour les permis dans la base locale.
- Supprimer le PDF apres validation ou rejet.
- Alerter avant l'echeance dans l'interface et par e-mail.
- Gerer les reglages et les destinataires des alertes.
- Journaliser les e-mails rejetes et les echecs d'envoi.
- Sauvegarder et restaurer les donnees applicatives.

## Hors V1

- Logiciel Windows.
- Tableau de bord de synthese.
- Export CSV ou Excel.
- Creation manuelle de permis.
- Historique des modifications de permis.
- Corbeille et restauration individuelle des permis supprimes.

## Exigences fonctionnelles

### Acces

**EXIGENCE AC-01**

Le systeme doit autoriser l'acces aux seuls membres du groupe LDAP YunoHost
`coordinateur`. Chaque membre peut importer, valider, supprimer des permis et
configurer les alertes.

**ACCEPTATION**

- Un membre du groupe accede a l'application.
- Un utilisateur hors groupe ne peut pas y acceder.
- Aucun role supplementaire n'est necessaire en V1.

### Import et validation

**EXIGENCE IM-01**

Le systeme doit accepter un PDF E-Permit importe par interface ou recu dans la
boite `epermit@onyx-ingenierie.com`, puis presenter les donnees extraites avant
tout enregistrement.

**ACCEPTATION**

- Un coordinateur peut importer un PDF E-Permit depuis l'interface.
- Un PDF recu par e-mail ouvre le meme ecran de validation.
- Les valeurs extraites sont modifiables avant validation.
- Un permis n'est enregistre qu'apres validation explicite.

**EXIGENCE IM-02**

Un e-mail doit etre traite seulement si son expediteur correspond a l'adresse
d'un coordinateur LDAP et si l'authentification du domaine expediteur reussit.

**ACCEPTATION**

- Un e-mail autorise avec PDF E-Permit ouvre une validation.
- Un e-mail non autorise ou non authentifie ne cree aucun permis.
- Sa piece jointe est supprimee sans etre conservee.
- Le rejet est journalise avec la date, l'expediteur et le motif.

**EXIGENCE IM-03**

Un PDF illisible, non E-Permit ou dont la structure E-Permit n'est pas reconnue
doit etre rejete, supprime et journalise.

**ACCEPTATION**

- Aucune fiche n'est creee pour un PDF rejete.
- Le coordinateur voit le motif de l'echec dans l'interface.
- Le PDF rejete n'est pas conserve.

### Donnees et conservation

**EXIGENCE D-01**

Le systeme doit enregistrer toutes les informations disponibles pour chaque
permis E-Permit, notamment numero, etat, unite, equipement, entreprises,
caracteristiques de l'intervention, dates de debut et de fin.

**ACCEPTATION**

- Les donnees extraites sont visibles et corrigibles avant validation.
- Une date de fin valide est disponible pour le calcul des alertes.

**EXIGENCE D-02**

Le numero de permis doit identifier une fiche. La validation d'un numero deja
present doit mettre a jour cette fiche.

**ACCEPTATION**

- Deux fiches distinctes n'existent pas pour un meme numero.
- Les modifications sont visibles avant la validation de la mise a jour.

**EXIGENCE D-03**

Le PDF doit etre supprime definitivement apres validation du permis, y compris
apres un echec d'extraction ou un rejet. Il ne doit jamais etre inclus dans une
sauvegarde.

**ACCEPTATION**

- Apres validation, seules les donnees du permis restent consultables.
- Aucun PDF valide, rejete ou en erreur ne subsiste dans le stockage.
- Une sauvegarde ne contient aucun PDF source.

**EXIGENCE D-04**

Un coordinateur doit pouvoir supprimer definitivement un permis valide. Le
permis supprime ne doit plus etre consulte ni alerte.

**ACCEPTATION**

- La suppression retire la fiche de la liste et des alertes.
- Aucune corbeille ni restauration individuelle n'est disponible.

### Consultation et alertes

**EXIGENCE AL-01**

La liste des permis doit etre filtrable par etat, unite, entreprise et periode
de fin. Les permis expires restent consultables et filtrables.

**ACCEPTATION**

- Les filtres peuvent etre combines.
- Un permis expire peut etre affiche apres sa date de fin.

**EXIGENCE AL-02**

Le systeme doit permettre aux coordinateurs de configurer le delai avant
echeance et la frequence des rappels. Une alerte doit etre affichee dans
l'interface et envoyee aux destinataires configures, jusqu'a la date de fin.

**ACCEPTATION**

- La modification des deux reglages s'applique aux alertes a venir.
- Une alerte est visible dans l'interface pendant la periode configuree.
- Des e-mails sont envoyes suivant la frequence configuree jusqu'a l'echeance.
- A la date de fin, le permis prend automatiquement l'etat `expire`.

**EXIGENCE AL-03**

Les coordinateurs doivent pouvoir gerer des destinataires internes ou externes.
Une adresse externe doit confirmer son inscription avant de recevoir une alerte.

**ACCEPTATION**

- Un coordinateur peut ajouter et retirer un destinataire.
- Une adresse externe non confirmee ne recoit aucune alerte.
- Une adresse externe confirmee recoit les alertes.
- Un echec d'envoi est visible dans l'interface.

### Journal et sauvegarde

**EXIGENCE JS-01**

Le journal des e-mails rejetes doit etre conserve sans purge et ne doit contenir
ni PDF ni contenu de PDF.

**ACCEPTATION**

- Les rejets restent consultables apres redemarrage.
- Le journal contient au minimum la date, l'expediteur et le motif.

**EXIGENCE JS-02**

La sauvegarde et la restauration doivent couvrir les permis, les reglages, les
destinataires et le journal des e-mails rejetes.

**ACCEPTATION**

- Une restauration retrouve ces donnees et reglages.
- Aucun PDF n'est restaure.

## Decisions prises

- La V1 est une application YunoHost, non une application Windows.
- La reception e-mail utilise `epermit@onyx-ingenierie.com`.
- Les alertes sont repetees jusqu'a la date de fin.
- Le delai et la frequence sont configurables.
- Les destinataires externes sont autorises apres confirmation.
- Les permis peuvent etre supprimes definitivement a la demande.

## Hypotheses et questions a traiter en architecture

- Le paquet utilisera les mecanismes YunoHost adaptes pour LDAP, e-mail et
  sauvegarde.
- Les valeurs initiales du delai et de la frequence seront definies au premier
  parametrage par un coordinateur.
- Les limites de taille et les controles de securite des PDF seront definis a
  l'etape architecture.

## Lots proposes

### Lot 1 - Fondations YunoHost

Objectif : installer le paquet, authentifier les coordinateurs et stocker les
donnees localement.

Condition de fin : un coordinateur accede a une application protegee avec une
base sauvegardable.

### Lot 2 - Import et validation E-Permit

Objectif : traiter les PDF par interface et e-mail, puis enregistrer les permis
valides sans conserver les sources.

Condition de fin : un permis E-Permit valide est stocke, mis a jour par numero
et son PDF est supprime.

### Lot 3 - Alertes

Objectif : configurer et envoyer les rappels, avec gestion des destinataires.

Condition de fin : une echeance genere les alertes attendues dans l'interface et
par e-mail.

### Lot 4 - Fiabilite

Objectif : tracer les rejets et assurer sauvegarde et restauration.

Condition de fin : les rejets sont journalises et les donnees sont restaurables
sans aucun PDF.
