# Rapport de validation — candidate applicative 0.2.0

## RÉUSSI — contrôles locaux

- Formatage Rust et Clippy avec avertissements traités comme erreurs.
- Tests Rust incluant l'extraction d'un vrai PDF synthétique avec Poppler :
  parsing, champs supplémentaires, dates, rejets, migration 0.1.x, parcours
  HTTP interne, validation explicite, remplacement par numéro, concurrence,
  suppression, confirmations et rejeu, CSRF, rappel/déduplication/expiration,
  échec d'envoi, expiration du cache et journal paginé.
- `tests/test_http.py` : processus serveur réel, upload multipart d'un PDF,
  correction et validation, persistance des permis et imports en attente après
  redémarrage, relecture du PDF, annulation, suppression des sources.
- `tests/test_mailbox.py` : refus hors groupe, refus d'en-têtes d'authentification
  forgés, pièces jointes ambiguës et échappement LDAP. Vérification cryptographique
  réelle avec une clé RSA générée à l'exécution et DNS simulé : signature valide,
  message modifié, domaine non aligné et signature partielle.
- `tests/test_package.py` : contrat du manifeste et snapshot SQLite WAL ; les
  réglages sont restitués, les brouillons sont exclus, la base source reste intacte.
- Syntaxe Bash des scripts de cycle de vie et `git diff --check`.

Les e-mails des tests Rust sont capturés par un faux sendmail, sans envoi externe.
Les tests DKIM ne dépendent pas du DNS de production.

## RÉUSSI — preuve client antérieure

La capture client montre le socle 0.1.x accessible sur YunoHost. Cela prouve
le démarrage de cette ancienne version, pas la recette de la version 0.2.

## NON_EXÉCUTÉ — recette serveur

- Mise à jour réelle 0.1.x → 0.2, retrait/réinstallation et restauration complète
  du paquet sur YunoHost ; l'environnement local est macOS sans serveur YunoHost.
- Autorisations SSOwat pour coordinateur, compte hors groupe et visiteur ;
  confidentialité du lien public de confirmation.
- Extraction d'un PDF E-Permit réel du client (aucun échantillon fourni).
- Livraison réelle d'une notification, délivrabilité externe et relève de
  `epermit@onyx-ingenierie.com` avec LDAP/DNS/IMAP de production.
- Vérification visuelle dans un navigateur réel ; le HTML et les formulaires
  ont été vérifiés par tests HTTP, sans outil navigateur dans cette session.

## Statut

Première version métier candidate, vérifiée localement. La recette client sur
les documents et services réels reste à réaliser. Ne pas confondre publication
d'une release, mise à jour du serveur et réception fonctionnelle.

## Retour arrière

Créer une sauvegarde YunoHost avant mise à jour. Restaurer cette sauvegarde
en cas d'échec. Ne pas supprimer l'application pour tenter une mise à jour.
Les PDF en attente ne sont pas sauvegardés : les valider avant sauvegarde ou
prévoir de les réimporter après restauration.
