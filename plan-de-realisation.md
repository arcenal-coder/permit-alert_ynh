# Plan de realisation - Permit Alert V1

Ce plan est une autorisation de preparation, pas une autorisation de deployer.

## Lot 1 - Socle YunoHost

### Objectif

Installer les fondations du paquet YunoHost et proteger l'acces aux donnees.

### Perimetre

- Paquet YunoHost v2, service Rust et cycle de vie install, upgrade, backup,
  restore et remove.
- Permission YunoHost limitee au groupe `coordinateur`.
- Base SQLite locale et configuration privee.
- Sauvegarde et restauration des donnees sans PDF.

### Exigences couvertes

- AC-01
- JS-02

### Dependances

- Architecture validee.
- Environnement YunoHost de test.

### Criteres de fin

- Le paquet s'installe et se desinstalle proprement.
- Un coordinateur seul accede au service.
- Une sauvegarde et une restauration restituent les donnees de test.

### Risque principal

- Conformite du cycle de vie YunoHost.

### Preuve attendue

- Tests d'installation, sauvegarde, restauration et suppression.

## Lot 2 - Import PDF

### Objectif

Importer un PDF E-Permit depuis l'interface, le faire valider puis supprimer
la source.

### Perimetre

- Controle du format E-Permit.
- Extraction des permis et ecran de correction.
- Creation ou mise a jour par numero de permis.
- Suppression apres validation, rejet ou echec.

### Exigences couvertes

- IM-01
- IM-03
- D-01 a D-04

### Dependances

- Lot 1.
- Echantillon PDF E-Permit representatif.

### Criteres de fin

- Un PDF valide cree ou met a jour un permis.
- Le PDF ne reste pas dans le stockage ni les sauvegardes.
- Un PDF invalide ne cree aucune fiche et son echec est visible.

### Risque principal

- Evolution de la structure E-Permit.

### Preuve attendue

- Tests sur PDF valide, invalide et incomplet.

## Lot 3 - Alertes

### Objectif

Envoyer les alertes configurees avant echeance et permettre leur suivi.

### Perimetre

- Reglage du delai et de la frequence.
- Etat expire automatique.
- Alertes interface et e-mail.
- Destinataires internes et externes confirmes.
- Visibilite des echecs d'envoi.

### Exigences couvertes

- AL-01 a AL-03

### Dependances

- Lots 1 et 2.
- Service de messagerie YunoHost fonctionnel.

### Criteres de fin

- Une echeance de test genere les rappels attendus.
- Un permis expire reste consultable sans rappel actif.
- Une adresse externe non confirmee ne recoit rien.

### Risque principal

- Fiabilite de l'envoi d'e-mails externes.

### Preuve attendue

- Tests de calendrier, destinataire confirme et echec d'envoi.

## Lot 4 - Reception e-mail et securite

### Objectif

Traiter de facon securisee les PDF recus a `epermit@onyx-ingenierie.com`.

### Perimetre

- Releve de la boite dediee.
- Verification de l'expediteur LDAP et de l'authentification de domaine.
- Rejets, suppression des pieces jointes et journal sans purge.

### Exigences couvertes

- IM-02
- JS-01

### Dependances

- Lots 1 et 2.
- Mecanisme YunoHost valide pour la releve securisee de la boite.

### Criteres de fin

- Un e-mail autorise ouvre une validation.
- Un e-mail non autorise ou non authentifie est rejete, supprime et journalise.
- Aucun PDF recu ne persiste apres traitement.

### Risque principal

- Integration securisee avec la boite de reception YunoHost.

### Preuve attendue

- Tests d'e-mails autorises, non autorises et non authentifies.
