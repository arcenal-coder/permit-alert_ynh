# Permit Alert

Application YunoHost de suivi des echeances de permis E-Permit.

## Installation

1. Telecharger l'archive `permit-alert-<version>-amd64.tar.gz` depuis une
   release GitHub avec un compte autorise.
2. Copier l'archive sur le serveur YunoHost.
3. Installer le paquet :

```bash
sudo yunohost app install /chemin/vers/permit-alert-<version>-amd64.tar.gz
```

Le groupe LDAP `coordinateur` doit exister avant l'installation. Le paquet V1
initial ne prend en charge que les serveurs YunoHost `amd64` et YunoHost 12.1
ou superieur.

## Creer une release

La publication d'un tag Git `v<version>` declenche GitHub Actions. Le workflow
compile le binaire Linux, produit l'archive YunoHost et l'ajoute a la release
privee correspondante.
