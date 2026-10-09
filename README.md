# Permit Alert

Application YunoHost de suivi des echeances de permis E-Permit.

## Installation

Installer le paquet depuis le catalogue d'applications personnalisees avec :

```bash
sudo yunohost app install https://github.com/arcenal-coder/permit-alert_ynh
```

Choisissez le groupe LDAP voulu dans le formulaire YunoHost. Sur le serveur
Onyx actuel, ce groupe est `coordinateurs`. Le paquet V1 initial ne prend en
charge que les serveurs YunoHost `amd64` et YunoHost 12.1 ou superieur.

## Creer une release

La publication d'un tag Git `v<version>` declenche GitHub Actions. Le workflow
compile le binaire Linux, produit l'archive YunoHost et l'ajoute a la release
privee correspondante.
