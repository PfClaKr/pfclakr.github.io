+++
title = "Le démarrage 2 : U-Boot — environnement, bootcmd, FIT et un script A/B"
description = "Ce que fait vraiment U-Boot, le chargeur d'amorçage du Linux embarqué, exécuté sur QEMU arm64 : l'environnement et fw_setenv côté Linux, le standard boot et extlinux.conf, un démarrage à la main avec booti, des images FIT vérifiées par hachage, et le choix d'un slot A/B dans un script de démarrage."
tags = ["linux", "u-boot", "boot", "embedded", "arm64"]
date = 2026-11-05T11:00:00
+++

La [partie 1](/fr/posts/unix/boot/boot-1-firmware/) s'arrêtait sur la ROM de démarrage d'un SoC ARM qui charge
un petit chargeur de premier niveau (U-Boot SPL ou équivalent) en SRAM, ce chargeur initialisant la DRAM puis
chargeant l'étape suivante. Cette étape suivante est le plus souvent U-Boot. Cet article décrit ce que fait
U-Boot jusqu'à avoir trouvé le noyau, l'avoir chargé et y avoir sauté.

L'environnement de test : la machine `virt` de QEMU (Cortex-A57), U-Boot 2025.10 du paquet `u-boot-qemu`
d'Ubuntu 24.04, et le noyau arm64 6.8 d'Ubuntu. L'initramfs est un busybox minimal qui affiche ce qu'il a reçu
du chargeur puis éteint la machine.

## Lire la bannière de démarrage

```text
U-Boot 2025.10-0ubuntu0.24.04.4 (Oct 07 2026 - 20:58:59 +0000)

DRAM:  1 GiB
Core:  51 devices, 14 uclasses, devicetree: board
Flash: 64 MiB
Loading Environment from Flash... *** Warning - bad CRC, using default environment

In:    serial,usbkbd
Out:   serial,vidconsole
Err:   serial,vidconsole
Net:   eth0: virtio-net#32
Hit any key to stop autoboot:  2
```

- `DRAM` : la mémoire utilisable. Sur une vraie carte, c'est ce que SPL a initialisé.
- `devicetree: board` : U-Boot découvre lui aussi le matériel par un arbre de périphériques. Sous QEMU, il
  utilise le DTB généré par QEMU.
- `Loading Environment from Flash... bad CRC` : pas d'environnement enregistré (ou un environnement corrompu),
  donc U-Boot se rabat sur les valeurs par défaut compilées en lui. Un avertissement courant au premier
  démarrage d'une carte.
- `Hit any key to stop autoboot` : appuyer sur une touche dans les `bootdelay` secondes donne l'invite `=>` ;
  sinon U-Boot exécute le contenu de la variable `bootcmd`.

## L'environnement

L'essentiel du comportement d'U-Boot se règle par des variables d'environnement. Quelques-unes, depuis l'invite :

```text
=> printenv bootcmd boot_targets kernel_addr_r ramdisk_addr_r bootdelay
bootcmd=bootflow scan -lb
boot_targets=qfw usb scsi virtio nvme dhcp
kernel_addr_r=0x40400000
ramdisk_addr_r=0x44000000
bootdelay=2
```

| Variable | Signification |
|---|---|
| `bootcmd` | commande exécutée au démarrage automatique |
| `bootargs` | la ligne de commande du noyau ; elle va dans `/chosen/bootargs` de l'arbre de périphériques au démarrage |
| `bootdelay` | secondes d'attente avant le démarrage automatique ; `-1` le désactive |
| `kernel_addr_r`, `ramdisk_addr_r`, `fdt_addr_r` | noms conventionnels des adresses où charger le noyau, l'initramfs et le DTB |
| `boot_targets` | l'ordre dans lequel le standard boot examine les périphériques de démarrage |

`setenv` modifie une variable et `saveenv` enregistre l'environnement. L'emplacement dépend de la
compilation : flash SPI, un décalage sur l'eMMC, un fichier `uboot.env` sur une partition FAT, un volume UBI…
`env default -a` revient aux valeurs par défaut.

### Format de stockage et outils côté Linux

L'environnement enregistré a un format simple : un CRC32 sur 4 octets, puis des chaînes `nom=valeur`
séparées par des octets NUL, puis du remplissage jusqu'à une taille fixe :

```text
000000   +   K 305   S   a   r   c   h   =   a   r   m  \0   b   a   u
000010   d   r   a   t   e   =   1   1   5   2   0   0  \0   b   o   a
```

Cette version QEMU utilise les 256 Kio (`0x40000`) du début de la deuxième banque de flash. La dernière ligne
de `printenv`, `Environment size: 514/262140 bytes`, donne 256 Kio moins les 4 octets du CRC. Certaines
configurations utilisent un **environnement redondant** : deux copies écrites en alternance, avec un octet de
drapeau en plus, pour qu'une copie intacte survive à une coupure de courant pendant une écriture.

Sous Linux, `fw_printenv` et `fw_setenv` de `libubootenv` (ou de `tools/env` dans les sources d'U-Boot) lisent
et écrivent la même zone. Son emplacement se déclare dans `/etc/fw_env.config` :

```text
# périphérique/fichier  décalage  taille env  taille secteur
/dev/mtd1               0x0       0x40000     0x20000
```

Un environnement redondant ajoute une seconde ligne avec l'emplacement de la deuxième copie. L'environnement
initial gravé en usine peut se construire directement depuis un fichier texte avec `mkenvimage` :

```bash
mkenvimage -s 0x40000 -o env.img env.txt       # env.txt : lignes clé=valeur, comme printenv
fw_setenv -c fw_env.config BOOT_ORDER "A B"
fw_setenv -c fw_env.config BOOT_A_LEFT 3
```

U-Boot lit au démarrage suivant les valeurs ainsi écrites depuis Linux :

```text
Loading Environment from Flash... OK
=> printenv BOOT_ORDER BOOT_A_LEFT
BOOT_ORDER=A B
BOOT_A_LEFT=3
```

C'est ainsi qu'un agent de mise à jour sous Linux (RAUC ou autre) modifie l'état du chargeur dans les mises à
jour A/B. À noter : avec l'émulation de flash de QEMU, le `saveenv` de cet U-Boot échoue sur `Flash buffer
write timeout` ; dans ces expériences, l'environnement a donc toujours été écrit avec les outils côté Linux.

## bootcmd et le standard boot

Les anciens `bootcmd` d'U-Boot étaient de longs scripts propres à chaque carte. Aujourd'hui, le **standard
boot** (bootstd) est la norme et `bootcmd=bootflow scan -lb` tient en une ligne. Il parcourt les
périphériques de démarrage (bootdevs) dans l'ordre de `boot_targets`, cherche sur chacun quelque chose que
reconnaît une méthode de démarrage (bootmeth) (`-l` affiche la liste, `-b` démarre dès qu'il trouve) et
démarre sur la première qui fonctionne.

| bootmeth | Cherche |
|---|---|
| `extlinux` | `/extlinux/extlinux.conf` ou `/boot/extlinux/extlinux.conf` sur une partition |
| `script` | `boot.scr` (une image de script U-Boot) |
| `efi_mgr`, `efi` | les entrées de démarrage UEFI, ou `\EFI\BOOT\BOOTAA64.EFI` sur une ESP |
| `pxe` | une configuration PXE sur le réseau |

Avec un disque contenant `/boot/extlinux/extlinux.conf`, le démarrage automatique donne :

```text
Scanning for bootflows in all bootdevs
Seq  Method       State   Uclass    Part  Name                      Filename
---  -----------  ------  --------  ----  ------------------------  ----------------
Scanning global bootmeth 'efi_mgr':
  0  efi_mgr      ready   (none)       0  <NULL>
** Booting bootflow '<NULL>' with efi_mgr
EFI boot manager: Cannot load any image
Scanning bootdev 'virtio-blk#33.bootdev':
  1  extlinux     ready   virtio       2  virtio-blk#33.bootdev.par /boot/extlinux/extlinux.conf
** Booting bootflow 'virtio-blk#33.bootdev.part_2' with extlinux
Retrieving file: /boot/Image
Retrieving file: /boot/initrd.img
append: console=ttyAMA0 root=/dev/vda2 rauc.slot=A
   Loading Ramdisk to 7d41b000, end 7d5108c1 ... OK
   Loading Device Tree to 000000007d2fd000, end 000000007d3fffff ... OK
Starting kernel ...
```

Il a d'abord essayé le gestionnaire de démarrage UEFI, sans succès, puis trouvé `extlinux.conf` sur la deuxième
partition du disque virtio et démarré dessus. Le fichier :

```text
default linux
timeout 1
label linux
    kernel /boot/Image
    initrd /boot/initrd.img
    append console=ttyAMA0 root=/dev/vda2 rauc.slot=A
```

Le format vient de syslinux : il est presque identique à une configuration syslinux sur x86, et il est facile
de faire mettre à jour ce fichier par la distribution à l'installation d'un noyau. Pour désigner un fichier
d'arbre de périphériques, on ajoute une ligne `fdt /boot/dtbs/carte.dtb`.

## Démarrer à la main

Faire à l'invite le travail du standard boot montre chaque étape :

```text
=> ls virtio 0:2 /boot
 59079048   Image
            extlinux/
 19607676   fit.itb
  1005761   initrd.img
=> load virtio 0:2 ${kernel_addr_r} /boot/Image
59079048 bytes read in 266 ms (211.8 MiB/s)
=> load virtio 0:2 ${ramdisk_addr_r} /boot/initrd.img
1005761 bytes read in 10 ms (95.9 MiB/s)
=> setenv bootargs console=ttyAMA0 rauc.slot=B
=> booti ${kernel_addr_r} ${ramdisk_addr_r}:${filesize} ${fdtcontroladdr}
## Flattened Device Tree blob at 7e546dc0
   Booting using the fdt blob at 0x7e546dc0
   Loading Ramdisk to 7d440000, end 7d5358c1 ... OK
   Loading Device Tree to 000000007d33d000, end 000000007d43ffff ... OK
Starting kernel ...

init: cmdline = console=ttyAMA0 rauc.slot=B
init: dt /chosen/bootargs = console=ttyAMA0 rauc.slot=B
init: initrd = 000000007d440000 .. 000000007d5358c1
```

- `load interface périphérique:partition adresse fichier` charge un fichier en mémoire et laisse sa taille dans `filesize`.
- `booti` sert au format `Image` arm64. Les `zImage` ARM 32 bits utilisent `bootz` ; les `uImage` historiques
  et FIT, `bootm`.
- Les trois arguments sont l'adresse du noyau, l'`adresse:taille` de l'initramfs et l'adresse du DTB.
  `fdtcontroladdr` est le DTB qu'utilise U-Boot lui-même, généré ici par QEMU. Sur une vraie carte, on charge
  plutôt le DTB compilé avec le noyau à `fdt_addr_r` et on passe celui-là.

Les trois dernières lignes viennent du `/init` de l'initramfs lancé par le noyau. Juste avant le démarrage,
U-Boot copie le DTB et écrit `bootargs` et l'emplacement de l'initramfs dans son nœud `/chosen`, où le noyau
les lit. Le `Loading Ramdisk to 7d440000, end 7d5358c1` d'U-Boot correspond exactement à `linux,initrd-start`
/ `-end` dans l'arbre de périphériques du noyau. Avant le démarrage, le `/chosen` du DTB d'origine n'en contient rien :

```text
=> fdt addr ${fdtcontroladdr}
=> fdt print /chosen
chosen {
	stdout-path = "/pl011@9000000";
	rng-seed = <...>;
	kaslr-seed = <...>;
};
```

## Les images FIT

Avec le noyau, l'initramfs et le DTB en fichiers séparés, des fichiers incompatibles peuvent se retrouver
ensemble, et rien ne permet de vérifier leur intégrité. **FIT** (Flattened Image Tree) les regroupe dans un
seul fichier au format arbre de périphériques, avec un hachage (et éventuellement une signature) pour chacun.
La source est un fichier `.its` :

```dts
/dts-v1/;

/ {
    description = "kernel + initramfs";
    #address-cells = <1>;

    images {
        kernel {
            description = "Linux 6.8";
            data = /incbin/("Image.gz");
            type = "kernel";
            arch = "arm64";
            os = "linux";
            compression = "gzip";
            load = <0x40400000>;
            entry = <0x40400000>;
            hash { algo = "sha256"; };
        };
        ramdisk {
            description = "busybox initramfs";
            data = /incbin/("initrd.img");
            type = "ramdisk";
            arch = "arm64";
            os = "linux";
            compression = "none";
            hash { algo = "sha256"; };
        };
    };

    configurations {
        default = "conf-1";
        conf-1 {
            description = "default";
            kernel = "kernel";
            ramdisk = "ramdisk";
        };
    };
};
```

```bash
mkimage -f image.its fit.itb
```

Sur une vraie carte, on ajouterait un nœud `fdt` sous `images` et `fdt = "fdt";` à la configuration pour
inclure aussi le DTB ; QEMU génère son DTB à l'exécution, donc seuls le noyau et l'initramfs y figurent. Pour
gérer plusieurs cartes avec une seule image, on donne à chacune sa configuration et on la choisit avec
`bootm ${addr}#conf-carte`.

U-Boot affiche le contenu et vérifie les hachages avec `iminfo`, et `bootm` les vérifie à nouveau au
démarrage. Le noyau est compressé en gzip : `bootm` le décompresse à son adresse `load` :

```text
=> load virtio 0:2 0x50000000 /boot/fit.itb
=> iminfo 0x50000000
   FIT image found
   FIT description: kernel + initramfs
    Image 0 (kernel)
     Compression:  gzip compressed
     Load Address: 0x40400000
     Hash algo:    sha256
    Image 1 (ramdisk)
     Hash algo:    sha256
    Default Configuration: 'conf-1'
## Checking hash(es) for FIT Image at 50000000 ...
   Hash(es) for Image 0 (kernel): sha256+
   Hash(es) for Image 1 (ramdisk): sha256+
=> bootm 0x50000000 0x50000000 ${fdtcontroladdr}
   Using 'conf-1' configuration
   Verifying Hash Integrity ... sha256+ OK
   Uncompressing Kernel Image to 40400000
   Loading Ramdisk to 7d440000, end 7d5358c1 ... OK
Starting kernel ...
```

Modifier un octet des données du noyau dans le FIT chargé avec `mw.b`, puis redémarrer, provoque un refus :

```text
=> mw.b 0x50100000 0x00 1
=> bootm 0x50000000 0x50000000 ${fdtcontroladdr}
   Verifying Hash Integrity ... sha256 error!
Bad hash value for 'hash' hash node in 'kernel' image node
Bad Data Hash
ERROR -2: can't get kernel image!
```

Les hachages ne protègent que contre la corruption accidentelle : un attaquant n'a qu'à les recalculer. Contre
la falsification, le FIT reçoit une **signature** : `mkimage -k répertoire_clés -K u-boot.dtb -r` le signe et
intègre la clé publique dans le DTB d'U-Boot comme clé *obligatoire*, et U-Boot refuse de démarrer toute
configuration qu'il ne peut pas vérifier avec cette clé. U-Boot lui-même, clé publique comprise, est protégé
par la vérification de signature de la ROM de démarrage vue en partie 1. On obtient ainsi une chaîne de
confiance de la ROM de démarrage jusqu'au noyau.

## Choisir un slot A/B dans un script de démarrage

Une logique qui dépasse un `bootcmd` d'une ligne s'écrit dans un script U-Boot. Le langage est le shell hush
d'U-Boot (proche de sh, mais bien plus petit), et le fichier texte est enveloppé par `mkimage` en `boot.scr`.
La bootmeth `script` du standard boot trouve ce fichier sur une partition et l'exécute.

```bash
mkimage -A arm64 -O linux -T script -C none -n "A/B boot" -d boot.cmd boot.scr
```

Comme dans l'[article sur les partitions A/B](/fr/posts/architecture/os/ota-update/ab-partitions/) et
l'[article sur RAUC](/fr/posts/architecture/os/ota-update/rauc/), l'environnement contient l'ordre des slots
`BOOT_ORDER` et le nombre de tentatives restantes par slot, `BOOT_A_LEFT` et `BOOT_B_LEFT` :

```bash
test -n "${BOOT_ORDER}" || setenv BOOT_ORDER "A B"
test -n "${BOOT_A_LEFT}" || setenv BOOT_A_LEFT 3
test -n "${BOOT_B_LEFT}" || setenv BOOT_B_LEFT 3

setenv slot
for s in ${BOOT_ORDER}; do
  if test -n "${slot}"; then
    true
  elif test "${s}" = A && test ${BOOT_A_LEFT} -gt 0; then
    setexpr BOOT_A_LEFT ${BOOT_A_LEFT} - 1
    setenv slot A
    setenv part 2
  elif test "${s}" = B && test ${BOOT_B_LEFT} -gt 0; then
    setexpr BOOT_B_LEFT ${BOOT_B_LEFT} - 1
    setenv slot B
    setenv part 3
  fi
done

if test -z "${slot}"; then
  echo "No bootable slot left"
  poweroff
fi
saveenv

echo "ab: BOOT_ORDER=${BOOT_ORDER} -> slot ${slot} (left: A=${BOOT_A_LEFT} B=${BOOT_B_LEFT})"
setenv bootargs "console=ttyAMA0 root=PARTLABEL=rootfs_${slot} rauc.slot=${slot}"
load virtio 0:${part} ${kernel_addr_r} /boot/Image
load virtio 0:${part} ${ramdisk_addr_r} /boot/initrd.img
booti ${kernel_addr_r} ${ramdisk_addr_r}:${filesize} ${fdtcontroladdr}
```

Contrairement à GRUB dans l'[article sur GRUB](/fr/posts/unix/boot/grub-ab/), U-Boot dispose de `setexpr` : on
peut décompter les tentatives. Le disque contient `boot.scr` sur la partition 1 (FAT) et `rootfs_A`,
`rootfs_B` sur les partitions 2 et 3 ; avant chaque démarrage, l'environnement a été reconstruit côté Linux
pour essayer deux états :

```text
### fw_setenv BOOT_ORDER="A B"
** Booting bootflow 'virtio-blk#33.bootdev.part_1' with script
ab: BOOT_ORDER=A B -> slot A (left: A=2 B=3)
init: cmdline = console=ttyAMA0 root=PARTLABEL=rootfs_A rauc.slot=A

### fw_setenv BOOT_ORDER="B A" BOOT_B_LEFT=0
** Booting bootflow 'virtio-blk#33.bootdev.part_1' with script
ab: BOOT_ORDER=B A -> slot A (left: A=2 B=0)
init: cmdline = console=ttyAMA0 root=PARTLABEL=rootfs_A rauc.slot=A
```

Le second état est celui d'un B mis à jour qui a échoué ses trois tentatives : `BOOT_ORDER` désigne encore B en
premier, mais sans tentative restante, le script revient à A. Le travail de Linux est le même qu'avec GRUB :

| Action | Commande |
|---|---|
| Activer B après y avoir écrit une nouvelle image | `fw_setenv BOOT_ORDER "B A"` et `fw_setenv BOOT_B_LEFT 3` |
| Confirmer que le slot démarré est sain | `fw_setenv BOOT_B_LEFT 3` (recharger ses tentatives) |

Une particularité de hush mérite d'être connue. La documentation de RAUC, et le script de l'article sur RAUC,
mettent la variable entre guillemets : `for BOOT_SLOT in "${BOOT_ORDER}"`. Un sh POSIX ne ferait qu'un tour,
sur le mot unique `"A B"`, mais le hush de cet U-Boot (2025.10) l'a quand même découpée en mots : les deux
scénarios ci-dessus donnent le même résultat avec ce script. Une branche `then` vide qui ne contient qu'un
commentaire a aussi fonctionné sans problème dans ce hush.

### Le bootcount intégré à U-Boot

Plutôt que de compter soi-même dans l'environnement, on peut utiliser la fonction bootcount d'U-Boot
(`CONFIG_BOOTCOUNT_LIMIT`). U-Boot incrémente `bootcount` à chaque démarrage et, dès qu'il dépasse `bootlimit`,
exécute `altbootcmd` au lieu de `bootcmd`. Le compteur est en général placé à un endroit qui ne coûte rien à
écrire, comme un registre de RTC ou un registre du SoC qui survit à un reset ; quand il est stocké dans
l'environnement (`CONFIG_BOOTCOUNT_ENV`), il n'est compté que si `upgrade_available=1`, pour ménager la flash.
Une fois le bon démarrage confirmé, Linux remet `bootcount` et `upgrade_available` à 0. Des outils de mise à jour
comme SWUpdate utilisent ce schéma. La version QEMU utilisée ici ne l'active pas (il n'y a pas de commande `bootcount`).
