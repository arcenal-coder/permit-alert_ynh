# Rapport de validation — version 0.3.0

## RÉUSSI — préparation des connecteurs 0.3

- 16 tests Rust, 6 tests de connecteurs Python, 6 tests d'import e-mail/DKIM,
  2 tests de paquet et 1 parcours HTTP réel : **31 tests réussis**.
- Conservation des secrets à champ vide, effacement explicite et absence de
  réutilisation sur un autre serveur/compte ; HTML et journal sans secret.
- Fichier privé `0600`, répertoire `0700`, enregistrement atomique et contrôle
  de révision contre les écrasements concurrents.
- Refus CSRF, refus d'activation incomplète et refus des transports non sécurisés.
- Envoi SMTP via la configuration enregistrée, transmise au helper par stdin
  sans secret dans argv ; pas de redémarrage nécessaire.
- Tests SMTP simulés : TLS et authentification, aucune commande DATA ni envoi
  lors d'un diagnostic, erreurs serveur remplacées par un message non sensible.
- Tests IMAP simulés : ouverture du dossier en lecture seule, aucun FETCH,
  STORE ou EXPUNGE lors d'un diagnostic.
- Test LDAP : mot de passe dans un fichier temporaire `0600`, supprimé après
  utilisation ; connexion TLS vérifiée et absence de secret dans argv.
- Parcours HTTP réel : enregistrement des connecteurs désactivés avec valeurs
  de test, conservation après redémarrage, champs secrets toujours masqués.
- Clippy strict, formatage et syntaxe Bash réussis.

La version 0.3 est publiée sous `v0.3.0`, référencée par le paquet `0.3.0~ynh2`.
Les 31 tests et le démarrage du binaire dans Debian 12 ont également réussi dans
la CI : https://github.com/arcenal-coder/permit-alert_ynh/actions/runs/38885517993
Les connexions réseau sont testées par simulation ; le paramétrage et les
diagnostics réels sont prévus in situ. Les preuves CI ci-dessous concernent
l'historique de validation de la version 0.2.1.

## Historique de validation applicative 0.2.1

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

## RÉUSSI — CI Linux 0.2.1

- https://github.com/arcenal-coder/permit-alert_ynh/actions/runs/38861924365
- 12 tests Rust (dont extraction réelle), 5 tests e-mail dont DKIM réel,
  2 tests de paquet et 1 parcours HTTP bout en bout : 20 tests réussis.
- Compilation du binaire Linux et démarrage/réponses HTTP dans Debian 12.
- Archive publiée : release `v0.2.1`, référencée par le paquet `0.2.1~ynh2`.
- Paquet final téléchargeable : release `v0.2.1-ynh2`, avec le même ensemble
  de tests réussi : https://github.com/arcenal-coder/permit-alert_ynh/actions/runs/38862477221
- Empreinte de l'archive binaire `v0.2.1` téléchargée et contrôlée localement :
  `936a1f3750852ee77a834ee8f606f75c0ac3ac885f7bdbce13c6505f8f7f498e`.

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
