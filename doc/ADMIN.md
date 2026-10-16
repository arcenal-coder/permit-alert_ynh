# Administration — Permit Alert 0.3

## Utilisation

Le groupe choisi dans les permissions YunoHost autorise l'ensemble des fonctions
métier. Sur Onyx, choisir `coordinateurs`. Le lien public `/confirm` ne donne
accès à aucune donnée de permis : il sert uniquement au consentement d'un
destinataire invité.

Les réglages de délai/fréquence doivent être enregistrés une première fois pour
activer les envois. Chaque adresse, y compris interne, reçoit un lien valable
7 jours à confirmer explicitement. Le journal indique les invitations et alertes
en échec. Un envoi accepté par sendmail n'est pas une preuve de réception : les
rejets SMTP ultérieurs relèvent également de la supervision de la messagerie.

## PDF

Extraction locale par Poppler, sans service distant ni OCR. Deux structures sont
prises en charge : champs `Libellé : valeur`, et tableau à colonnes séparées par
au moins deux espaces, tabulations ou `|`. Numéro et date de fin sont obligatoires.
Toutes les colonnes supplémentaires reconnues sont conservées et corrigibles.
Les structures ambiguës sont rejetées, jamais enregistrées partiellement.
Ce parseur doit encore être confronté à un échantillon E-Permit réel du client.

Les PDF en attente sont dans `/var/cache/permit-alert/imports`, privé au compte
du service. Ils survivent à un redémarrage, sont supprimés après validation,
annulation ou rejet, et expirent après 24 heures (expiration journalisée).
Les fichiers orphelins interrompus sont nettoyés après une heure.
Ni ce cache ni les brouillons ne figurent dans les sauvegardes. Une restauration
nécessite donc de réimporter les documents qui n'étaient pas encore validés.

## Paramétrage des connecteurs sur place

Ouvrir **Connecteurs**, également accessible depuis **Réglages**. Le même groupe
coordinateurs gère les connexions ; la route publique de confirmation n'y donne
aucun accès.

- **Envoi** : messagerie locale YunoHost, ou serveur SMTP avec STARTTLS obligatoire
  (souvent 587) ou TLS implicite (souvent 465). Choisir l'expéditeur et renseigner
  les identifiants si le serveur les exige. Les certificats sont toujours vérifiés.
- **Réception** : serveur IMAP TLS, port, identifiant, secret et dossier de la
  boîte dédiée. Préparer les champs avant d'activer la relève automatique.
- **LDAP** : URL, bases utilisateurs/groupes, groupe des coordinateurs et, si
  nécessaire, DN/mot de passe de connexion. La recherche peut être anonyme.
  LDAP sans TLS est limité à la boucle locale ; un annuaire distant nécessite LDAPS.

Enregistrer avant de tester. Les changements s'appliquent sans redémarrage ; la
relève et les rappels utilisent les nouveaux paramètres à leur prochain passage.
Une opération déjà engagée conserve sa configuration jusqu'à son achèvement.
Le test SMTP négocie TLS et l'authentification sans envoyer de message ; le test
IMAP ouvre le dossier en lecture seule sans lire de message ni le supprimer ;
le test LDAP vérifie les bases et le groupe sans afficher de données utilisateur.
Le test d'envoi local vérifie seulement la présence du programme sendmail.

Un mot de passe n'est jamais réaffiché. Le laisser vide conserve le secret pour
le même serveur/port/compte. Un changement de serveur ou compte nécessite de le
ressaisir. Pour le supprimer, cocher « Effacer le secret enregistré » ; un
connecteur actif doit rester complet, sinon l'enregistrement est refusé.

La configuration est enregistrée atomiquement dans `$data_dir/private/connectors.json`,
hors SQLite, avec dossier `0700` et fichier `0600`. Elle est sauvegardée dans les
archives privées YunoHost et ses droits sont réappliqués lors de la restauration.
Ne pas placer ce fichier dans Git. Les résultats de tests et les erreurs utilisent
des messages prédéfinis, sans réponse brute susceptible de contenir des secrets.

Le fichier système `mail.env` reste pris en charge pour les installations 0.2 :
ses variables initialisent les valeurs tant qu'aucune configuration n'a été
enregistrée depuis l'interface. Ensuite, le fichier privé des connecteurs fait
autorité ; modifier `mail.env` ne remplace pas les réglages enregistrés.

## Réception e-mail

Utiliser uniquement une boîte **dédiée** : les messages traités ou rejetés sont
définitivement supprimés après persistance du résultat. Un PDF par e-mail, 16 Mo
maximum par message. Les éventuels autres types de pièces jointes ne sont pas
importés. La relève traite un message par minute.

Configurer la réception et le groupe d'expéditeurs depuis l'écran Connecteurs.
Le serveur IMAP doit fournir TLS avec un certificat valide et UIDPLUS pour
la suppression sélective. Aucun contournement TLS n'est prévu.

Les expéditeurs sont recherchés dans le LDAP configuré (YunoHost local par défaut) et doivent appartenir
au groupe indiqué. Au moins une signature DKIM doit vérifier le message entier
et avoir un domaine `d=` égal au domaine From. Les signatures partielles `l=`
sont refusées. Aucun en-tête Authentication-Results fourni par l'expéditeur
n'est considéré comme une preuve. Les e-mails sans DKIM aligné sont rejetés,
même si SPF seul réussit. Les erreurs réseau/LDAP laissent les messages en boîte.

## Service et sauvegardes

Le service effectue sa maintenance et ses rappels chaque minute ; aucune tâche
cron concurrente n'est nécessaire. Après échec d'envoi, nouvelle tentative après
30 minutes. Les succès sont dédupliqués par permis et destinataire ; en cas de
crash exactement après acceptation SMTP et avant écriture SQLite, un doublon
peut survenir à la reprise (pas de garantie d'envoi exactement une fois).

Le snapshot SQLite est cohérent, y compris avec WAL actif. Il contient les permis,
réglages, destinataires, états d'envoi et journaux, sans brouillon ni PDF. La
configuration privée des connecteurs est sauvegardée séparément sous `$data_dir/private` ;
l'ancien `mail.env`, s'il existe, est conservé avec le répertoire d'installation. Les
archives doivent donc être réservées aux administrateurs.

Avant mise à jour, sauvegarder l'application via YunoHost. La migration depuis
0.1.x conserve les fiches existantes. En cas d'échec, restaurer la sauvegarde ;
ne pas supprimer ou purger les données comme procédure de dépannage.
