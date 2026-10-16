# Permit Alert

Application YunoHost de suivi des echeances de permis E-Permit.

## Etat actuel

Version applicative 0.2 : interface responsive en français, imports PDF avec
correction et validation, mise à jour par numéro, filtres et suppression,
rappels automatiques et destinataires confirmés, journal paginé sans purge.
La relève e-mail est optionnelle et nécessite les identifiants de la boîte dédiée.

Les tests utilisent des PDF synthétiques et une messagerie simulée. La recette
avec un E-Permit réel et la vérification des intégrations sur le serveur client
restent nécessaires. Voir `TEST_REPORT.md`, `WORKFLOWS.md` et `doc/ADMIN.md`.

## Installation

Installer le paquet depuis le catalogue d'applications personnalisees avec :

```bash
sudo yunohost app install https://github.com/arcenal-coder/permit-alert_ynh
```

Choisissez le groupe LDAP voulu dans le formulaire YunoHost. Sur le serveur
Onyx actuel, ce groupe est `coordinateurs`. Le paquet V1 initial ne prend en
charge que les serveurs YunoHost `amd64` et YunoHost 12.1 ou superieur.

## Mise à jour d'une installation existante

Créer d'abord une sauvegarde via YunoHost, puis mettre à jour `permit-alert`
depuis la même URL, sans supprimer l'application. La base 0.1.x est migrée
automatiquement. Ouvrir Réglages, enregistrer le délai/fréquence et ajouter
les destinataires. La relève e-mail est décrite dans `doc/ADMIN.md`.

## Développement et tests

Installer Rust et Poppler (`poppler-utils` sur Debian, `poppler` sur macOS).

```bash
cargo test --manifest-path app/Cargo.toml -- --include-ignored
cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path app/Cargo.toml
python3 tests/test_http.py
python3 tests/test_package.py
```

Les tests cryptographiques e-mail nécessitent `dkimpy`, `dnspython` et OpenSSL
dans l'environnement de test : `python3 tests/test_mailbox.py`.
Les dépendances de production sont gérées par les ressources APT du paquet.

## Creer une release

La publication d'un tag Git `v<version>` declenche GitHub Actions. Le workflow
compile le binaire Linux, produit l'archive YunoHost et l'ajoute a la release
du depot correspondant. Le workflow verifie egalement le demarrage du binaire
dans Debian 12. Apres publication, mettre a jour l'URL et l'empreinte SHA-256
de `resources.sources.main` dans le manifeste du paquet.
