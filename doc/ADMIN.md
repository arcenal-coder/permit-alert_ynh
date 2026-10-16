# Administration — Permit Alert 0.2

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

## Réception e-mail

Utiliser uniquement une boîte **dédiée** : les messages traités ou rejetés sont
définitivement supprimés après persistance du résultat. Un PDF par e-mail, 16 Mo
maximum par message. Les éventuels autres types de pièces jointes ne sont pas
importés. La relève traite un message par minute.

Créer `/var/www/permit-alert/mail.env` depuis `conf/mail.env.example`, propriété
`root:root`, mode `0600`, puis redémarrer le service via YunoHost. systemd charge
ce fichier avant de lancer le compte non privilégié de l'application.

Configurer `PERMIT_ALERT_IMAP_HOST`, `PERMIT_ALERT_IMAP_USER`,
`PERMIT_ALERT_IMAP_PASSWORD` et `PERMIT_ALERT_LDAP_GROUP=coordinateurs`.
Le serveur IMAP doit fournir TLS avec un certificat valide et UIDPLUS pour
la suppression sélective. Aucun contournement TLS n'est prévu.

Les expéditeurs sont recherchés dans le LDAP local YunoHost et doivent appartenir
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
configuration privée est sauvegardée avec le répertoire d'installation. Les
archives doivent donc être réservées aux administrateurs.

Avant mise à jour, sauvegarder l'application via YunoHost. La migration depuis
0.1.x conserve les fiches existantes. En cas d'échec, restaurer la sauvegarde ;
ne pas supprimer ou purger les données comme procédure de dépannage.
