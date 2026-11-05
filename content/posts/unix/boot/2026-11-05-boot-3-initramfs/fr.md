+++
title = "Le démarrage 3 : l'initramfs — comment le noyau atteint la vraie racine"
description = "Ce que fait l'initramfs entre le noyau et le vrai système de fichiers racine : le format cpio et la façon dont le noyau le décompresse, quand on peut s'en passer et quand on ne peut pas, un /init busybox fait main avec switch_root, la structure d'initramfs-tools sous Ubuntu et le débogage avec break=, et sa personnalisation par hooks et scripts."
tags = ["linux", "initramfs", "boot", "embedded", "kernel"]
date = 2026-11-05T12:00:00
+++

Dans la [partie 1](/fr/posts/unix/boot/boot-1-firmware/) et la [partie 2](/fr/posts/unix/boot/boot-2-u-boot/),
le chargeur d'amorçage a chargé un initramfs en mémoire avec le noyau et indiqué au noyau où il se trouvait. Une
fois le matériel initialisé, le noyau doit monter le système de fichiers racine et lancer `/sbin/init`. Mais si
le pilote du disque racine est compilé en module, si la racine est sur LVM, RAID ou LUKS, ou sur le réseau
(NFS), le noyau ne peut pas le faire seul. L'initramfs comble ce vide : **un petit système de fichiers racine en mémoire**.

Les expériences ont tourné sur QEMU x86 avec le noyau 6.8 d'Ubuntu, avec l'image disque (ESP, `rootfs_A`,
`rootfs_B`) construite dans l'[article sur GRUB et l'A/B](/fr/posts/unix/boot/grub-ab/).

## Comment le noyau traite un initramfs

Un fichier initramfs est une **archive cpio** (format newc), en général compressée avec gzip ou zstd. Une
archive non compressée commence par le nombre magique de newc, sur six octets :

```text
$ head -c 6 initrd.img-6.8.0-147-generic
070701
```

Tôt au démarrage, le noyau décompresse l'archive dans **rootfs**, une instance de tmpfs (ou ramfs), et lance
son `/init` comme PID 1. `rdinit=` sur la ligne de commande désigne un autre programme. Quelques détails :

- On peut **concaténer** plusieurs archives. Le noyau les décompresse dans l'ordre, les fichiers suivants
  écrasant les précédents au même chemin. Archives compressées et non compressées peuvent se mélanger.
- L'image du noyau contient elle-même un initramfs (`CONFIG_INITRAMFS_SOURCE`). Par défaut, c'est une archive
  minimale avec seulement `/dev`, `/dev/console` et `/root`, et celle du chargeur est décompressée par-dessus.
  Certains systèmes embarqués y placent tout leur initramfs et livrent un seul fichier noyau.
- S'il n'y a **pas** de `/init` après décompression, le noyau revient à l'ancienne méthode : il monte lui-même
  le périphérique de `root=` et lance `/sbin/init` (ou `init=`).

Le mécanisme précédent était l'**initrd**, une image de périphérique bloc (un disque en RAM). Le nom de fichier
`initrd.img` et la commande `initrd` des chargeurs sont des restes de cette époque ; presque tout utilise cpio aujourd'hui.

## Quand on peut s'en passer

Avec les pilotes intégrés au noyau, on peut démarrer sans initramfs. Le noyau x86 d'Ubuntu intègre (`=y`) le
bloc virtio et ext4 : ce disque convient pour un test. En démarrant sans initramfs avec
`root=PARTLABEL=rootfs_A`, le noyau monte lui-même la racine :

```text
[    2.455157] EXT4-fs (vda2): mounted filesystem cb87ce6f-... ro with ordered data mode.
[    2.455654] VFS: Mounted root (ext4 filesystem) readonly on device 253:2.
[    2.742791] Run /sbin/init as init process
rootfs: pid 1, running release 1.0 from /dev/root
```

Désigner la même partition par l'étiquette de son système de fichiers (`root=LABEL=rootA`) provoque une panique :

```text
[    2.394034] Disabling rootwait; root= is invalid.
[    2.401243] VFS: Cannot open root device "LABEL=rootA" or unknown-block(0,0): error -6
[    2.401369] Please append a correct "root=" boot option; here are the available partitions:
[    2.401879]   fd01           32768 vda1 001b70e0-a665-42fc-af81-cecafcc05824
[    2.401987]   fd02           65536 vda2 a8d14487-02a0-491b-9ae9-d75f471a987c
[    2.402051]   fd03           65536 vda3 402dbd4e-e479-4cc9-bc1c-079d1ece0630
[    2.402506] Kernel panic - not syncing: VFS: Unable to mount root fs on unknown-block(0,0)
```

La différence tient à l'endroit où se trouve l'information. Les seules formes de `root=` que le noyau comprend
seul sont les noms de périphériques comme `/dev/vda2`, ainsi que `PARTUUID=` et `PARTLABEL=` issus de la table
des partitions. `LABEL=` et `UUID=` sont stockés **dans le système de fichiers** : il faut ouvrir chaque
périphérique et lire son superbloc. C'est le travail de `blkid` et d'udev en espace utilisateur, d'où
l'initramfs. La valeur affichée à côté de chaque partition dans le message de panique est son PARTUUID.

Les raisons typiques d'avoir besoin d'un initramfs :

- le pilote du périphérique ou du système de fichiers racine est un module (le cas de la plupart des noyaux de distribution) ;
- la racine est désignée par `UUID=` ou `LABEL=` ;
- la racine est sur LVM, RAID logiciel, chiffrement LUKS ou NFS ;
- la racine doit passer un fsck avant montage, ou est assemblée en couches comme dans
  l'[article sur OverlayFS](/fr/posts/architecture/os/ota-update/overlayfs/).

## En fabriquer un à la main

Un seul busybox lié statiquement suffit pour un initramfs fonctionnel. C'est celui des expériences de
l'article sur GRUB :

```bash
mkdir -p initramfs/{bin,dev,proc,sys,newroot}
cp /usr/bin/busybox initramfs/bin/        # le binaire statique du paquet busybox-static
install -m755 init initramfs/init
(cd initramfs && find . | cpio -o -H newc | gzip -9) > initrd.img
```

Et son `/init` :

```sh
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys

echo "initramfs: pid $$, cmdline: $(cat /proc/cmdline)"

for arg in $(cat /proc/cmdline); do
  case "$arg" in
    root=PARTLABEL=*) want=${arg#root=PARTLABEL=} ;;
  esac
done

for ev in /sys/class/block/*/uevent; do
  if grep -qx "PARTNAME=$want" "$ev"; then
    dev=/dev/$(sed -n 's/^DEVNAME=//p' "$ev")
  fi
done
echo "initramfs: root PARTLABEL=$want is $dev"

mkdir -p /newroot
mount -o ro "$dev" /newroot
umount /proc /sys
mount --move /dev /newroot/dev
exec switch_root /newroot /sbin/init
```

1. Créer les liens des applets busybox et monter `/dev`, `/proc` et `/sys`. `/dev` est un devtmpfs, où le
   noyau crée lui-même les nœuds de périphériques.
2. Lire `root=` dans la ligne de commande du noyau. Le noyau laisse aussi dans `/proc/cmdline` les arguments
   qu'il ne connaît pas : des valeurs comme `rauc.slot=` se lisent donc ici également.
3. Trouver le périphérique racine. Le `/sys/class/block/*/uevent` de chaque partition contient un `PARTNAME=`
   (le nom de partition GPT) : pas besoin d'udev.
4. Monter la racine sur `/newroot`, y déplacer `/dev`, puis **`switch_root`**.

```text
initramfs: pid 1, cmdline: BOOT_IMAGE=/boot/vmlinuz-6.8.0-147-generic root=PARTLABEL=rootfs_A ro rauc.slot=A console=ttyS0
initramfs: root PARTLABEL=rootfs_A is /dev/vda2
rootfs: pid 1, running release 1.0 from /dev/vda2
```

`switch_root` supprime tous les fichiers de l'initramfs pour rendre la mémoire, déplace la nouvelle racine sur
`/` (`mount --move` puis `chroot`) et fait un `exec` de l'init de la nouvelle racine. Comme c'est un `exec`, le
PID reste 1 : sur la dernière ligne, le `/sbin/init` de la vraie racine affiche lui aussi le PID 1. On n'utilise
pas `pivot_root` parce que rootfs est un montage spécial qu'on ne peut ni démonter ni déplacer : on le vide et
on monte la nouvelle racine par-dessus. C'est aussi pourquoi `switch_root` ne fonctionne qu'en PID 1.

## L'initramfs d'une distribution : initramfs-tools

Les distributions ne fabriquent pas leurs initramfs à la main ; elles utilisent un générateur. Debian et Ubuntu
utilisent **initramfs-tools**, Fedora et RHEL **dracut**, Arch Linux **mkinitcpio**. Sous Ubuntu, installer un
paquet de noyau lance `update-initramfs`, qui crée `/boot/initrd.img-<version>`, le travail étant fait par `mkinitramfs`.

Voici un initramfs pour le noyau 6.8 construit avec le `mkinitramfs` d'Ubuntu 24.04 :

```text
$ lsinitramfs initrd.img-6.8.0-147-generic | wc -l
523
$ lsinitramfs initrd.img-6.8.0-147-generic | grep -c '\.ko'
148
$ unmkinitramfs initrd.img-6.8.0-147-generic x && du -sh x/*
12M	x/early
27M	x/main
```

Le fichier contient deux archives. La première, `early`, est un cpio non compressé, qui contient des modules du
noyau déjà compressés en zstd (`*.ko.zst`) ; les garder à part évite de recompresser des fichiers compressés.
Vient ensuite `main`, tout le reste, compressé en zstd. Les mises à jour de microcode Intel et AMD vont elles aussi
dans un cpio non compressé placé en tête (`kernel/x86/microcode/…`), parce que le noyau les y récupère très tôt,
avant de décompresser quoi que ce soit.

`main` contient busybox, des utilitaires klibc, `systemd-udevd`, les modules, et un `/init` écrit en script
shell. Au démarrage, ce `/init` affiche un message par étape :

```text
Loading, please wait...
Starting systemd-udevd version 255.4-1ubuntu8.18
Begin: Loading essential drivers ... done.
Begin: Running /scripts/init-premount ... done.
Begin: Mounting root file system ... Begin: Running /scripts/local-top ... done.
Begin: Running /scripts/local-premount ... done.
Warning: fsck not present, so skipping root file system
done.
Begin: Running /scripts/local-bottom ... done.
Begin: Running /scripts/init-bottom ... done.
mount: mounting /run on /root/run failed: No such file or directory
rootfs: pid 1, running release 1.0 from /dev/vda2
```

Le squelette de `/init` (l'essentiel de ses 388 lignes) :

```sh
export init=/sbin/init
export rootmnt=/root
. /scripts/functions
# … analyse de la ligne de commande du noyau (root=, init=, break=, debug, …)
maybe_break top
run_scripts /scripts/init-top          # démarrage d'udev, etc.
maybe_break modules
# … chargement des modules listés dans /etc/initramfs-tools/modules
maybe_break premount
run_scripts /scripts/init-premount
maybe_break mount
. /scripts/local                       # local-top → attente du périphérique → local-premount → fsck → montage → local-bottom
. /scripts/nfs
maybe_break mountroot
mountroot
maybe_break bottom
run_scripts /scripts/init-bottom       # arrêt d'udev, /dev déplacé vers ${rootmnt}/dev
mount -n -o move /run ${rootmnt}/run
maybe_break init
mount -n -o move /sys ${rootmnt}/sys
mount -n -o move /proc ${rootmnt}/proc
exec run-init ${drop_caps} "${rootmnt}" "${init}" "$@" <"${rootmnt}/dev/console" >"${rootmnt}/dev/console" 2>&1
```

Le `run-init` final est l'équivalent klibc de `switch_root`. Il fait le même travail que l'initramfs fait main,
avec udev, le chargement des modules et des répertoires de scripts par étape en plus. Comme `/run`, `/sys`,
`/proc` et `/dev` sont déplacés vers la vraie racine, celle-ci doit avoir ces points de montage ;
l'avertissement `mounting /run on /root/run failed` du journal vient de la racine de test busybox, qui n'a pas
de répertoire `/run`.

`root=PARTLABEL=` est traité par `resolve_device` dans `scripts/functions`, avec `LABEL=`, `UUID=` et
`PARTUUID=`, via les liens créés par udev comme `/dev/disk/by-partlabel/`.

## Déboguer : break=

Quand l'initramfs ne trouve pas la racine, il tombe en général dans un shell avec l'invite `(initramfs)`. Pour
s'arrêter volontairement, on ajoute `break=<étape>` à la ligne de commande du noyau ; les noms d'étapes sont les
arguments de `maybe_break` dans le squelette ci-dessus (`top`, `modules`, `premount`, `mount`, `mountroot`,
`bottom`, `init`). Démarrage avec `break=premount` :

```text
Begin: Loading essential drivers ... done.
Spawning shell within the initramfs
BusyBox v1.36.1 (Ubuntu 1:1.36.1-6ubuntu3.1) built-in shell (ash)
(initramfs) echo $$; cat /proc/cmdline
139
console=ttyS0 root=PARTLABEL=rootfs_A ro break=premount
(initramfs) cat /proc/mounts
rootfs / rootfs rw 0 0
sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
udev /dev devtmpfs rw,nosuid,relatime,size=477888k,nr_inodes=119472,mode=755,inode64 0 0
devpts /dev/pts devpts rw,nosuid,noexec,relatime,gid=5,mode=620,ptmxmode=000 0 0
tmpfs /run tmpfs rw,nosuid,nodev,noexec,relatime,size=98456k,mode=755,inode64 0 0
(initramfs) ls -l /dev/disk/by-partlabel/
lrwxrwxrwx    1        10 esp -> ../../vda1
lrwxrwxrwx    1        10 rootfs_B -> ../../vda3
lrwxrwxrwx    1        10 rootfs_A -> ../../vda2
(initramfs) exit
Begin: Running /scripts/init-premount ... done.
Begin: Mounting root file system ... Begin: Running /scripts/local-top ... done.
```

- Le PID du shell est 139, pas 1. Le PID 1 reste `/init`, et le shell tourne comme son enfant ; `exit` laisse
  `/init` reprendre là où il s'était arrêté.
- `/` est `rootfs`, le système de fichiers dans lequel le noyau a décompressé l'initramfs.
- udev a déjà tourné : les liens de `/dev/disk/by-partlabel/` existent. Pour une racine introuvable, on commence
  ici avec `blkid`, `ls /dev` et `cat /proc/partitions` pour voir si le noyau voit seulement le périphérique.

Autres arguments utiles :

| Argument | Effet |
|---|---|
| `debug` | initramfs-tools : trace `/init` comme `sh -x` dans `/run/initramfs/initramfs.debug` |
| `rdinit=/bin/sh` | un shell en PID 1 à la place de `/init` ; aucun script n'est exécuté |
| `panic=10` | redémarrage 10 secondes après une panique du noyau, pour qu'un appareil distant ne reste pas bloqué |
| `rd.break`, `rd.shell`, `rd.debug` | les mêmes rôles dans dracut (étapes nommées comme `rd.break=pre-mount`) |

## Personnaliser : hooks et scripts

initramfs-tools offre deux types de points d'extension :

- les **hooks** (`/etc/initramfs-tools/hooks/`) s'exécutent quand `mkinitramfs` **construit** l'image et décident de son contenu ;
- les **scripts** (`/etc/initramfs-tools/scripts/<étape>/`) sont placés dans l'initramfs et s'exécutent à
  l'étape correspondante **pendant le démarrage**.

Par exemple, un hook qui ajoute `blkid`, et un script qui affiche les informations de slot juste après le
montage de la racine :

```sh
#!/bin/sh
# /etc/initramfs-tools/hooks/slotinfo -- exécuté par mkinitramfs
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /usr/share/initramfs-tools/hook-functions
copy_exec /usr/sbin/blkid /sbin
```

```sh
#!/bin/sh
# /etc/initramfs-tools/scripts/local-bottom/slotinfo -- au démarrage, après le montage de la racine sur ${rootmnt}
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /scripts/functions

for x in $(cat /proc/cmdline); do
  case "$x" in rauc.slot=*) slot=${x#rauc.slot=} ;; esac
done
log_success_msg "slot ${slot:-?}, root ${ROOT} = $(blkid -o value -s PARTLABEL "$(resolve_device "$ROOT")")"
```

- Les lignes `PREREQ`/`prereqs` sont un modèle imposé ; initramfs-tools s'en sert pour ordonner les scripts.
- `copy_exec` copie un binaire avec les bibliothèques partagées auxquelles il est lié ; l'image contient bien
  `usr/sbin/blkid` et `libblkid.so.1`. Les modules s'ajoutent avec `manual_add_modules`.
- Les deux fichiers doivent être exécutables, et une modification ne prend effet qu'après reconstruction avec
  `update-initramfs -u`.

En démarrant avec `rauc.slot=A`, le message apparaît à l'étape `local-bottom` :

```text
Begin: Running /scripts/local-premount ... done.
Warning: fsck not present, so skipping root file system
done.
Begin: Running /scripts/local-bottom ... Success: slot A, root /dev/vda2 = rootfs_A
done.
Begin: Running /scripts/init-bottom ... done.
```

On voit aussi que `ROOT` est déjà résolu en `/dev/vda2` à l'étape `local-bottom`. C'est à cet endroit qu'on
transformerait la racine en overlay, qu'on choisirait une partition de données par slot ou qu'on enregistrerait
un démarrage raté.
