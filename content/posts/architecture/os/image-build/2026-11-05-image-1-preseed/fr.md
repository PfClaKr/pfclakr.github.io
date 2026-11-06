+++
title = "Construction d'images OS 1 : installer Debian sans intervention avec preseed"
description = "Comment l'installateur Debian (d-i) pose ses questions et comment preseed y répond à l'avance : la ligne de commande de démarrage, chaque partie de preseed.cfg, une recette partman pour une disposition A/B, late_command, et où regarder quand l'installation s'arrête. Vérifié par une installation dans QEMU."
tags = ["linux", "debian", "preseed", "packer", "ota"]
draft = true
date = 2026-11-05T13:00:00
+++

Les articles sur les [partitions A/B](/fr/posts/architecture/os/ota-update/ab-partitions/),
[GRUB A/B](/fr/posts/unix/boot/grub-ab/) et [RAUC](/fr/posts/architecture/os/ota-update/rauc/) partaient d'un
appareil déjà installé qui reçoit des mises à jour. Cette série traite l'étape d'avant : **construire l'image**
qui ira sur l'appareil.

1. **preseed** (cet article) : installer Debian netinst sans personne au clavier
2. **Packer** : un modèle qui lance l'installation dans VirtualBox et exporte un OVA et une image rootfs pour RAUC
3. **GitLab CI** : un pipeline qui construit l'image à chaque commit, signe un bundle RAUC et publie une release

Il y a deux livrables. L'**OVA** est l'image d'usine pour les nouvelles VM ou les nouveaux appareils. Le
**bundle RAUC** est la mise à jour des appareils déjà déployés. Les deux sortent du même build.

## Comment d-i pose ses questions

Chaque écran de l'installateur Debian (debian-installer, d-i) est une **question debconf**. Langue, clavier,
nom d'hôte, miroir, partitionnement, mot de passe root : chacune a une clé de la forme
`propriétaire nom type valeur`.

```text
d-i netcfg/get_hostname string debian
d-i partman/confirm boolean true
tasksel tasksel/first multiselect ssh-server
```

Un **preseed** est un fichier où ces réponses sont écrites à l'avance. Avant d'afficher une question, d-i
consulte la base debconf ; si une réponse s'y trouve déjà, la question est marquée comme vue et l'installation
continue. La première colonne est le propriétaire : `d-i` pour les questions de l'installateur lui-même, le nom
du paquet (`tasksel`, `popularity-contest`) pour celles des paquets installés.

Chaque question a aussi une **priorité** (low, medium, high, critical). Avec le paramètre de démarrage
`priority=critical`, tout ce qui est en dessous de critical n'est pas posé et prend sa valeur par défaut. Si une
question critical n'a pas de réponse dans le preseed, l'installation s'arrête dessus et attend. C'est la cause
de presque toutes les installations automatiques bloquées.

## Trois façons de transmettre un preseed

| Méthode | Utilisation |
|---|---|
| **initrd** : `preseed.cfg` à la racine de l'initrd | reconstruire soi-même l'image de l'installateur |
| **file** : un fichier sur l'ISO, `preseed/file=/cdrom/preseed.cfg` | ISO personnalisées |
| **network** : `preseed/url=http://…/preseed.cfg` | PXE, Packer |

Packer lance un petit serveur HTTP pendant le build et met son adresse sur la ligne de commande de démarrage.
On peut utiliser l'ISO netinst officielle telle quelle, sans la modifier : c'est pour cela que cette méthode est
la plus courante.

## La ligne de commande de démarrage

L'ISO netinst démarrée en UEFI affiche un menu GRUB. Au lieu de choisir une entrée, on appuie sur `c` pour
obtenir la ligne de commande GRUB et démarrer le noyau directement. Dans l'article suivant, Packer tape ces
touches à notre place.

```text
linux /install.amd/vmlinuz auto=true priority=critical preseed/url=http://10.0.2.2:8080/preseed.cfg hostname=debian domain=
initrd /install.amd/initrd.gz
boot
```

- `auto=true` **change l'ordre des questions.** Normalement, d-i demande la langue et le clavier d'abord, puis
  configure le réseau. Mais le fichier preseed ne peut être téléchargé qu'une fois le réseau en place.
  `auto=true` repousse les questions de langue et de clavier après le chargement du preseed, pour que ces
  réponses puissent aussi venir du fichier.
- `preseed/url`, `priority`, `hostname` et `domain` sont des clés debconf passées directement comme paramètres
  de démarrage. `hostname` est un alias court de `netcfg/get_hostname`. Chaque `clé=valeur` de la ligne de
  commande du noyau entre dans debconf : n'importe quelle clé du fichier preseed peut donc être mise ici.
- **Pourquoi le nom d'hôte est sur la ligne de commande** : la configuration réseau (netcfg) s'exécute *avant*
  le téléchargement du preseed, et c'est à ce moment qu'elle demande le nom d'hôte. L'écrire seulement dans le
  fichier arrive trop tard, la question a déjà été posée. On le met donc aux deux endroits.

## preseed.cfg

Le fichier complet fait environ 70 lignes. Dans l'article suivant, Packer remplit les emplacements `${…}` avec
`templatefile()` avant de le servir.

### Locale, réseau, miroir

```text
d-i debian-installer/locale string en_US.UTF-8
d-i keyboard-configuration/xkb-keymap select us

d-i netcfg/choose_interface select auto
d-i netcfg/get_hostname string ${hostname}
d-i netcfg/get_domain string
d-i netcfg/hostname string ${hostname}
d-i hw-detect/load_firmware boolean false

d-i mirror/country string manual
d-i mirror/http/hostname string deb.debian.org
d-i mirror/http/directory string /debian
d-i mirror/http/proxy string ${apt_proxy}
d-i apt-setup/services-select multiselect security, updates
d-i apt-setup/cdrom/set-first boolean false
d-i apt-setup/disable-cdrom-entries boolean true
```

- `choose_interface select auto` prend la première interface qui a un lien. Inutile d'écrire en dur le nom de
  la carte réseau VirtualBox.
- `netcfg/hostname` impose ce nom même si le DHCP en fournit un. `get_hostname` n'est que la valeur par défaut
  de la question et cède devant celle du DHCP.
- `load_firmware false` supprime la question sur le chargement de micrologiciels manquants depuis un support
  amovible.
- **Le miroir reste fixe, seul le proxy change.** En CI, le runner héberge un apt-cacher-ng et `apt_proxy` vaut
  `http://10.0.2.2:3142`, pour que chaque build ne retélécharge pas les mêmes paquets. En local, c'est une
  chaîne vide. Faire pointer le miroir lui-même vers le cache laisserait cette adresse dans le `sources.list`
  du système installé. Le proxy, lui, tient sur une ligne de `/etc/apt/apt.conf`, facile à supprimer au
  nettoyage.
- `cdrom/set-first false` et `disable-cdrom-entries true` évitent que l'ISO reste dans les sources APT du
  système installé.

### Comptes et horloge

```text
d-i passwd/root-login boolean true
d-i passwd/make-user boolean false
d-i passwd/root-password password ${root_password}
d-i passwd/root-password-again password ${root_password}

d-i clock-setup/utc boolean true
d-i time/zone string Etc/UTC
d-i clock-setup/ntp boolean true
```

Pas d'utilisateur ordinaire, seulement root. Le mot de passe **ne sert qu'à Packer pour se connecter en SSH
pendant le build**. L'étape finale de nettoyage le verrouille, et l'image terminée n'accepte que des clés SSH.

:::note
`passwd/root-password-crypted` accepte un hachage à la place. Le texte en clair est utilisé ici parce que le
preseed n'est servi qu'en HTTP local pendant le build et que le mot de passe est verrouillé à la fin. La valeur
vient d'une variable CI masquée.
:::

### Partitionnement : la disposition A/B

C'est là que l'installation diffère le plus d'une installation normale. Elle construit la disposition de
l'[article GRUB A/B](/fr/posts/unix/boot/grub-ab/) : une ESP, le slot A, le slot B et une partition de données
qui survit aux mises à jour.

```text
d-i partman-auto/method string regular
d-i partman-partitioning/choose_label select gpt
d-i partman-partitioning/default_label string gpt
d-i partman-auto/expert_recipe string \
    ab ::                                               \
      256 256 256 free                                  \
        $iflabel{ gpt } $reusemethod{ }                 \
        method{ efi } format{ }                         \
      .                                                 \
      3072 3072 3072 ext4                               \
        method{ format } format{ }                      \
        use_filesystem{ } filesystem{ ext4 }            \
        mountpoint{ / }                                 \
      .                                                 \
      3072 3072 3072 ext4                               \
        method{ keep }                                  \
      .                                                 \
      1024 1024 -1 ext4                                 \
        method{ format } format{ }                      \
        use_filesystem{ } filesystem{ ext4 }            \
        mountpoint{ /data }                             \
      .
d-i partman-auto/choose_recipe select ab
d-i partman-basicfilesystems/no_swap boolean false
d-i partman-basicmethods/method_only boolean false
d-i partman-partitioning/confirm_write_new_label boolean true
d-i partman/choose_partition select finish
d-i partman/confirm boolean true
d-i partman/confirm_nooverwrite boolean true
```

Une `expert_recipe` s'écrit `nom ::` suivi des partitions séparées par `.`. La première ligne de chaque
partition est `minimum priorité maximum système_de_fichiers`, en Mo.

- **Minimum, priorité, maximum** : partman donne à chaque partition son minimum, puis répartit l'espace restant
  proportionnellement à la priorité, jusqu'au maximum de chacune. Les slots A et B ont 3072 partout : ils font
  exactement 3 Go. La partition de données a un maximum de `-1` (sans limite) et prend le reste.
- `$iflabel{ gpt } $reusemethod{ } method{ efi }` : sur un disque GPT, créer une ESP, ou réutiliser celle qui
  existe déjà. Les mots commençant par `$` sont des conditions ou des drapeaux ; `nom{ valeur }` règle quelque
  chose sur la partition.
- **Le slot B est `method{ keep }`** : la partition est créée mais ni formatée ni montée. B est vide dans
  l'image d'usine et RAUC le remplit à la première mise à jour. Une partition sans système de fichiers fait
  demander à d-i « Aucun système de fichiers n'est indiqué pour la partition n° 3. Revenir au menu ? ».
  `method_only false` est la réponse « ne pas revenir ».
- Pas de swap. d-i demande alors s'il faut continuer sans swap, d'où `no_swap false`.
- Il y a quatre confirmations : écrire une nouvelle table de partitions (`confirm_write_new_label`), choisir
  « Terminer » sur l'écran de partitionnement (`choose_partition`), écrire sur le disque (`confirm`), et la
  confirmation quand aucune partition n'est formatée (`confirm_nooverwrite`). S'il en manque une, l'installation
  s'arrête sur cet écran.

:::warning Noms et ordre des partitions
Les recettes partman n'ont pas d'entrée pour les noms de partition GPT (comme `rootfs_A`). Elles créent les
partitions 1, 2, 3, 4 dans l'ordre où elles sont écrites ; l'étape de provisionnement de l'article suivant les
nomme donc par numéro avec `sgdisk -c`. Le sélecteur de l'ESP utilise `(hd0,gpt2)` et `(hd0,gpt3)` : l'ordre ne
doit pas changer.
:::

### Paquets et chargeur d'amorçage

```text
tasksel tasksel/first multiselect ssh-server
d-i pkgsel/include string ca-certificates
d-i pkgsel/upgrade select full-upgrade
popularity-contest popularity-contest/participate boolean false

d-i grub-installer/only_debian boolean true
d-i grub-installer/bootdev string default
d-i grub-installer/force-efi-extra-removable boolean true
```

- Seulement `ssh-server` dans tasksel. Ajouter `standard` grossit l'image de plusieurs centaines de Mo. Le
  reste est installé à l'étape Packer : l'installateur ne s'occupe que d'« un système minimal qui démarre et
  accepte SSH ».
- `pkgsel/upgrade full-upgrade` applique pendant l'installation les mises à jour de sécurité publiées après
  l'ISO netinst.
- `force-efi-extra-removable` installe GRUB au chemin des supports amovibles `\EFI\BOOT\BOOTX64.EFI` en plus de
  `\EFI\debian\`. Le système démarre même si l'entrée de démarrage de la NVRAM du micrologiciel disparaît. C'est
  important, car rien ne garantit que la NVRAM de la VM suive un export OVA. L'article suivant remplace ce GRUB
  par le sélecteur A/B, mais le premier démarrage après l'installation l'utilise.

### late_command

```text
d-i preseed/late_command string \
    in-target sh -c 'echo "PermitRootLogin yes" > /etc/ssh/sshd_config.d/90-packer.conf'

d-i finish-install/reboot_in_progress note
```

`late_command` s'exécute dans l'environnement de l'installateur à la fin de l'installation, juste avant le
redémarrage. Le système installé est monté sur `/target`, et `in-target` exécute la commande à l'intérieur, dans
un chroot.

Le sshd de Debian est réglé par défaut sur `PermitRootLogin prohibit-password` : root ne peut pas se connecter
en SSH avec un mot de passe. Seul ce réglage change ici pour que Packer puisse entrer, et le fichier est
supprimé au nettoyage. Tout le reste se fait côté Packer : `late_command` est une seule ligne de shell, qui
devient difficile à lire et à déboguer en grandissant.

`reboot_in_progress note` saute l'écran final « Installation terminée » et redémarre tout de suite.

## Trouver le nom des clés

Les clés preseed ne s'apprennent pas par cœur. On les trouve à trois endroits.

- **Le fichier d'exemple** : l'annexe du manuel d'installation Debian fournit un `example-preseed.txt` pour
  chaque version. Presque toutes les options y sont, avec des commentaires.
- **Un système installé une fois à la main** : `debconf-get-selections --installer`, du paquet
  `debconf-utils`, affiche toutes les réponses données pendant l'installation au format preseed. Les choix faits
  à l'écran se recopient directement.
- **`/var/log/installer/cdebconf/questions.dat`** : la base des questions laissée sur le système installé.

## Quand l'installation s'arrête

Une installation automatique bloquée affiche une seule question. Il faut ajouter la clé de cette question au
preseed.

- Pendant l'installation, **Alt+F4** montre le journal en direct et **Alt+F2** ouvre un shell. Dans le shell,
  `grep -A3 nom /var/lib/cdebconf/questions.dat` donne la clé.
- Après l'installation, le journal complet est dans `/var/log/installer/syslog`.
- Souvent oubliés : les quatre confirmations partman ci-dessus, l'avertissement d'absence de swap, et les
  questions posées avant que le réseau ne soit prêt (langue, clavier, nom d'hôte). Ces dernières se règlent avec
  `auto=true` et les paramètres de la ligne de commande.

C'est arrivé pendant l'écriture de cet article. La première version n'avait pas `method_only`, et au bout
d'environ 30 minutes l'installation attendait sur cet écran, à l'étape du partitionnement.

```text
[!!] Partition disks
No file system is specified for partition #3 of SCSI1 (0,0,0) (sda).
If you do not go back to the partitioning menu and assign a file system to this partition,
it won't be used at all.
Go back to the menu?                                    <Go Back>   <Yes>   <No>
```

Le texte des questions se trouve dans le fichier debconf `templates` de chaque udeb. En cherchant cette phrase
dans les udebs `partman-*` téléchargés par l'installateur, on trouve la clé.

```bash
ar x partman-basicmethods_80_all.udeb control.tar.xz
tar xJf control.tar.xz ./templates
grep -B3 "won't be used at all" templates
# Template: partman-basicmethods/method_only
# Type: boolean
# Description: Go back to the menu?
#  No file system is specified for partition #${PARTITION} of ${DEVICE}.
```

C'est un `boolean` et la question est « Revenir au menu ? » : la réponse est `false`.

:::info Ubuntu fonctionne autrement
Ubuntu est aussi dérivée de Debian, mais son installateur est passé à subiquity (Server depuis 20.04, desktop
depuis 23.04) et les installations automatiques utilisent **autoinstall** (un fichier YAML au format
cloud-init). preseed ne fonctionne pas sur les Ubuntu récentes.
:::

## Essai dans QEMU

@@RESULTS@@

L'article suivant place ce preseed dans un modèle Packer, transforme le système installé en disposition A/B, et
exporte un OVA et une image rootfs.
