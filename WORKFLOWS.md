# Parcours V1

Un seul rôle applicatif : coordinateur, autorisé par la permission SSOwat
YunoHost affectée à `coordinateurs`. Aucun profil métier supplémentaire.

| Acteur | Déclencheur / écran | Action et données visibles | Sortie / preuve |
| --- | --- | --- | --- |
| Coordinateur | Permis | Filtrer état, unité, entreprise, dates ; consulter les champs validés | Liste filtrée ou fiche |
| Coordinateur | Importer | Envoyer un PDF E-Permit | Brouillon extrait ou rejet journalisé |
| Coordinateur | Imports à valider | Comparer le PDF et les champs ; voir une fiche existante de même numéro | Validation transactionnelle puis suppression du PDF |
| Coordinateur | Permis / fiche | Supprimer définitivement avec confirmation | Fiche et rappels supprimés |
| Coordinateur | Réglages | Délai, fréquence, destinataires | Réglages persistants ; confirmation envoyée |
| Destinataire externe | Lien personnel reçu | Seulement sa confirmation d'inscription ; aucun permis | Confirmation explicite par POST |
| Coordinateur | Journal | Motifs de rejet et échecs d'envoi, sans contenu des PDF | Diagnostic persistant |
| Tâche de fond | Chaque minute | Expiration, échéances, relève e-mail configurée | Envois tracés et imports en attente |
| Visiteur / utilisateur hors groupe | Toute route métier | Aucun accès via SSOwat | Refus YunoHost |

Les fichiers PDF sont placés dans un répertoire temporaire privé hors des
sauvegardes, supprimés à validation/rejet/annulation et expirent après 24 heures.
Le lien de confirmation est la seule route publique du paquet.

## Recette serveur

Tester un coordinateur, un utilisateur hors groupe et un navigateur anonyme.
Importer un PDF réel, corriger, comparer un doublon, valider, filtrer et supprimer.
Vérifier un destinataire non confirmé puis confirmé, les rappels et l'expiration.
Vérifier le rejet des e-mails non LDAP ou sans signature DKIM alignée valide.
Restaurer une sauvegarde : retrouver permis/réglages/journaux, aucun PDF.
