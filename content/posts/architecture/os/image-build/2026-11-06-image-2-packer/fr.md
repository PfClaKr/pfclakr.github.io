+++
title = "Construction d'images OS 2 : un OVA et un rootfs pour RAUC avec Packer"
description = "Un modèle Packer virtualbox-iso qui installe Debian netinst, ajoute la disposition A/B et RAUC, retire tout ce qui n'appartient qu'à la VM de build, puis exporte ensemble un OVA et un rootfs.ext4. Le modèle et les scripts de provisionnement ligne par ligne."
tags = ["linux", "debian", "packer", "virtualbox", "rauc", "ota"]
date = 2026-11-06T14:00:00
+++

Le preseed de la [partie 1](/fr/posts/architecture/os/image-build/image-1-preseed/) ne couvre que « Debian
s'installe tout seul ». Cet article enveloppe cette installation dans Packer, transforme la VM obtenue en image
produit, et exporte deux livrables.

- **OVA** : l'image d'usine pour les nouvelles VM ou les nouveaux appareils. Elle contient l'ESP, le slot A
  (rempli), le slot B (vide) et la partition de données.
- **rootfs.ext4** : uniquement le système de fichiers racine du même build. La partie 3 le signe dans un bundle
  RAUC pour les appareils déjà déployés.

## Organisation du dépôt

```text
os-image/
├── debian.pkr.hcl            source (la VM) et build (les étapes)
├── variables.pkr.hcl
├── http/
│   └── preseed.pkrtpl        le preseed de la partie 1, rempli par templatefile()
├── files/                    fichiers envoyés dans la VM et installés
│   ├── esp-grub.cfg          le sélecteur de slot sur l'ESP (article GRUB A/B)
│   ├── 10_ab                 le script update-grub dans un slot (article GRUB A/B)
│   ├── system.conf           définition des slots RAUC
│   ├── ca.cert.pem           certificat de CA qui vérifie la signature des bundles
│   ├── authorized_keys       clé publique SSH de l'image terminée
│   ├── rauc-mark-good.service
│   ├── 20-wired.network
│   └── ssh-hostkeys.service
├── scripts/
│   ├── 10-packages.sh
│   ├── 20-ab-layout.sh
│   ├── 90-cleanup.sh
│   └── 95-export-rootfs.sh
├── rauc/manifest.raucm.in    partie 3
└── .gitlab-ci.yml            partie 3
```

## Ce que fait Packer

`packer build` fait passer le builder `virtualbox-iso` par ces étapes :

1. Télécharger l'ISO, vérifier sa somme de contrôle et la garder en cache (`PACKER_CACHE_DIR`).
2. Créer la VM et y attacher le disque et l'ISO.
3. Lancer un **serveur HTTP** qui sert le preseed.
4. Démarrer la VM et **taper `boot_command` au clavier.**
5. Une fois l'installation terminée et la VM redémarrée, **attendre que SSH réponde**.
6. Exécuter les provisionneurs (envoi de fichiers, scripts shell) dans l'ordre.
7. Éteindre la VM avec `shutdown_command`, **l'exporter en OVA** et la supprimer.

## Plugins et variables

```hcl
packer {
  required_version = ">= 1.10.0"
  required_plugins {
    virtualbox = {
      source  = "github.com/hashicorp/virtualbox"
      version = "~> 1.1"
    }
  }
}

locals {
  vm_name    = "debian-ab-${var.image_version}"
  output_dir = "output/${local.vm_name}"
}
```

Les builders sont des plugins distribués séparément de Packer. `packer init .` lit `required_plugins`, les
télécharge et les installe dans `~/.config/packer/plugins`. Épingler `~> 1.1` évite qu'un jour la CI récupère
soudain une version 2.x.

```hcl
variable "image_version" {
  type    = string
  default = "0.0.0-dev"
}

variable "root_password" {
  type      = string
  sensitive = true
}

variable "apt_proxy" {
  type    = string
  default = ""
}

variable "iso_urls" {
  type = list(string)
  default = [
    "https://cdimage.debian.org/cdimage/release/13.7.0/amd64/iso-cd/debian-13.7.0-amd64-netinst.iso",
    "https://cdimage.debian.org/cdimage/archive/13.7.0/amd64/iso-cd/debian-13.7.0-amd64-netinst.iso",
  ]
}

variable "iso_checksum" {
  type    = string
  default = "sha256:a7ef94ac2fb9a7fec454552abd629b7cc9d5155c886165a45649f5ce6167e355"
}
```

- Packer n'affiche jamais dans ses journaux la valeur d'une variable `sensitive = true`. On la passe par la
  variable d'environnement `PKR_VAR_root_password` plutôt que par `-var`. Packer lit `PKR_VAR_<nom>` tout seul, ce
  qui va bien avec les variables CI.
- **Pourquoi deux URL d'ISO** : à chaque version intermédiaire (13.8), Debian déplace l'ISO précédente de
  `release/` vers `archive/`. `iso_urls` est essayée dans l'ordre : un modèle épinglé sur 13.7.0 continue de
  fonctionner après la version suivante. L'URL `current/` n'est pas utilisée : son contenu change et la somme de
  contrôle ne correspond plus.
- La somme de contrôle peut pointer vers un `file:…/SHA256SUMS` distant, mais l'écrire en clair permet de savoir,
  rien qu'avec le dépôt, quelle ISO a servi au build.

## source : la VM

```hcl
source "virtualbox-iso" "debian" {
  vm_name       = local.vm_name
  guest_os_type = "Debian_64"
  firmware      = "efi"
  cpus          = 2
  memory        = 2048
  headless      = true

  disk_size                = 10240
  hard_drive_interface     = "sata"
  hard_drive_discard       = true
  hard_drive_nonrotational = true
  iso_interface            = "sata"
  gfx_controller           = "vmsvga"
  gfx_vram_size            = 16
  rtc_time_base            = "UTC"
  guest_additions_mode     = "disable"

  iso_urls     = var.iso_urls
  iso_checksum = var.iso_checksum
  # (suite)
```

- **`firmware = "efi"`** : le sélecteur A/B se trouve à `\EFI\BOOT\BOOTX64.EFI` sur l'ESP, la VM doit donc démarrer
  en UEFI.
- **Disque de 10 Go** : selon la recette de la partie 1, une ESP de 256 Mo, deux slots de 3 Go et le reste pour
  les données. La partition de données sert aussi d'espace temporaire pour l'image rootfs pendant le build (voir
  plus bas).
- **`hard_drive_discard`, `hard_drive_nonrotational`** : présenter le disque comme un SSD et transmettre TRIM
  jusqu'à VirtualBox. Un `fstrim` final retire alors aussi les blocs effacés de l'image disque, et l'OVA est plus
  petit.
- **`guest_additions_mode = "disable"`** : les Additions invité VirtualBox n'ont rien à faire dans une image
  produit. Par défaut, Packer envoie leur ISO : on désactive.

### Servir le preseed : http_content

```hcl
  http_content = {
    "/preseed.cfg" = templatefile("${path.root}/http/preseed.pkrtpl", {
      hostname      = "debian"
      apt_proxy     = var.apt_proxy
      root_password = var.root_password
    })
  }
```

`http_directory` sert un répertoire tel quel ; `http_content` prend **directement des chemins et des contenus**.
`templatefile()` remplit les emplacements comme `${root_password}` du preseed et le résultat est servi depuis la
mémoire : aucun fichier preseed contenant le mot de passe n'arrive sur le disque.

### boot_command

```hcl
  boot_wait = "5s"
  boot_command = [
    "c<wait2>",
    "linux /install.amd/vmlinuz auto=true priority=critical ",
    "preseed/url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed.cfg ",
    "hostname=debian domain=<enter><wait>",
    "initrd /install.amd/initrd.gz<enter><wait5>",
    "boot<enter>",
  ]
```

- Après `boot_wait`, les chaînes sont envoyées **comme des frappes clavier**. Des touches spéciales comme
  `<enter>` et `<esc>` se mêlent à `<wait>` (1 seconde) et `<wait5>` (5 secondes). Le tableau est simplement
  concaténé ; les retours à la ligne ne servent qu'à la lecture.
- `c` fait passer du menu GRUB de l'ISO netinst à la ligne de commande, puis les trois lignes de la partie 1 sont
  tapées telles quelles.
- Packer remplit `{{ .HTTPIP }}` et `{{ .HTTPPort }}`. Avec le NAT de VirtualBox, l'invité voit l'hôte en
  `10.0.2.2`, et le port est choisi à chaque build entre 8000 et 9000.
- **Si un `<wait>` est trop court**, la ligne suivante arrive avant que GRUB ait reçu toutes les touches. Sur un
  runner lent, prévoyez de la marge pour le premier `boot_wait`. Un essai avec `headless = false` montre tout de
  suite si les frappes sont bien passées.

### Le NAT de VirtualBox 7

```hcl
  vboxmanage = [
    ["modifyvm", "{{ .Name }}", "--nat-localhostreachable1", "on"],
  ]
```

`vboxmanage` est une liste de commandes `VBoxManage` exécutées après la création de la VM et avant son démarrage.
Depuis VirtualBox 7.0, l'accès d'un invité NAT au loopback de l'hôte via `10.0.2.2` est **désactivé par défaut**.
Sans ce réglage, l'installateur ne peut pas récupérer le preseed et s'arrête sur l'écran « impossible de récupérer
le fichier de préconfiguration ». Un apt-cacher-ng sur le runner en a besoin pour la même raison.

### SSH et export

```hcl
  communicator     = "ssh"
  ssh_username     = "root"
  ssh_password     = var.root_password
  ssh_timeout      = "60m"
  shutdown_command = "shutdown -P now"

  output_directory = local.output_dir
  format           = "ova"
  export_opts = [
    "--manifest",
    "--vsys", "0",
    "--product", "pfclakr Debian A/B",
    "--version", var.image_version,
  ]
}
```

- `ssh_timeout` couvre **toute l'installation**. Packer attend SSH dès le démarrage de la VM, et sshd n'apparaît
  qu'une fois l'installation terminée et la VM redémarrée. Avec un miroir lent, cela peut prendre 20 à
  30 minutes : soyez large.
- Avec `format = "ova"`, le `VBoxManage export` final regroupe le descripteur OVF, le VMDK et un manifeste dans un
  seul fichier. `export_opts` est ajouté tel quel à cette commande. `--manifest` met dans l'OVA les sommes SHA256
  de chaque fichier, et `--product` et `--version` apparaissent dans la fenêtre d'importation.

## build : les étapes

```hcl
build {
  sources = ["source.virtualbox-iso.debian"]

  provisioner "shell" {
    inline = ["mkdir -p /tmp/files"]
  }

  provisioner "file" {
    source      = "${path.root}/files/"
    destination = "/tmp/files"
  }

  provisioner "shell" {
    environment_vars = [
      "COMPATIBLE=${var.compatible}",
      "IMAGE_VERSION=${var.image_version}",
    ]
    scripts = [
      "${path.root}/scripts/10-packages.sh",
      "${path.root}/scripts/20-ab-layout.sh",
      "${path.root}/scripts/90-cleanup.sh",
      "${path.root}/scripts/95-export-rootfs.sh",
    ]
  }

  provisioner "file" {
    direction   = "download"
    source      = "/data/rootfs.ext4"
    destination = "${local.output_dir}/rootfs.ext4"
  }

  provisioner "shell" {
    inline = ["rm -f /data/rootfs.ext4", "fstrim -av"]
  }
}
```

- Quand `source` se termine par `/`, c'est **le contenu du répertoire** qui va dans `destination`. Sans la barre
  oblique, le répertoire `files` lui-même se retrouve dedans.
- `/tmp` est un tmpfs depuis Debian 13 : les fichiers envoyés ne restent pas dans l'image.
- Le provisionneur shell envoie chaque script et l'exécute précédé de `environment_vars`. Si l'un d'eux sort avec
  un code non nul, le build s'arrête et, par défaut, supprime la VM. Pour examiner une VM en échec, lancez
  `packer build -on-error=abort` en local.
- **`direction = "download"`** fait l'inverse et copie un fichier de la VM vers la machine de build. C'est ainsi
  que sort l'image rootfs pour RAUC.

## 10-packages.sh

```sh
#!/bin/sh
set -eu
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
    rauc rauc-service gdisk efibootmgr grub-efi-amd64-bin systemd-resolved
```

- Debian découpe RAUC en deux paquets. `rauc` est l'outil en ligne de commande ; `rauc-service` contient
  `rauc.service`, activé par D-Bus. La commande `rauc` de Debian fonctionne en envoyant ses requêtes à ce service :
  `rauc install` et `rauc status` ont besoin des deux.
- `gdisk` fournit `sgdisk` pour nommer les partitions, et `grub-efi-amd64-bin` les modules pour construire le GRUB
  de l'ESP. `systemd-resolved` sert au changement de réseau de l'étape de nettoyage. Tous les paquets sont installés
  ici pour que cette étape n'ait plus besoin du réseau.

## 20-ab-layout.sh : d'une installation normale à A/B

L'installateur a produit une Debian ordinaire. Ce script la transforme selon la disposition de
l'[article GRUB A/B](/fr/posts/unix/boot/grub-ab/). Les raisons de chaque partie sont dans cet article ; ici, on ne
regarde que ce qui change quand on construit une image.

```sh
F=/tmp/files   # là où le provisionneur file les a envoyés

# 1. Nommer les partitions. Tout ce qui suit les désigne par leur nom, jamais par UUID.
disk=/dev/$(lsblk -no PKNAME "$(findmnt -no SOURCE /)")
sgdisk -c 1:esp -c 2:rootfs_A -c 3:rootfs_B -c 4:data "$disk"

cat > /etc/fstab <<'EOT'
/dev/root       /          ext4  errors=remount-ro  0 1
PARTLABEL=esp   /boot/efi  vfat  umask=0077         0 1
PARTLABEL=data  /data      ext4  defaults           0 2
EOT
```

**Pourquoi réécrire fstab** est le point le plus important de cette série. L'installateur écrit les UUID des
systèmes de fichiers dans fstab. Un UUID est généré à chaque `mkfs` : il **change d'un build à l'autre.** Écrivez
avec RAUC un rootfs 1.1 sur un appareil issu de l'OVA 1.0, et le fstab de la 1.1 désigne l'UUID de l'ESP de la VM de
build 1.1 : `/boot/efi` ne se monte pas, RAUC ne peut pas modifier grubenv, et mark-good échoue.

- L'ESP et les données sont montées par **nom de partition GPT**. La table de partitions est écrite une fois en
  usine et les mises à jour n'y touchent pas : elle est identique vue depuis le rootfs de n'importe quel build.
- La racine s'écrit simplement `/dev/root`. La vraie racine vient de `root=PARTLABEL=rootfs_$SLOT` sur la ligne de
  commande du noyau ; cette ligne de fstab ne fournit que les options de montage (`errors=remount-ro`) et le
  remontage en rw. Le fstab par défaut de Yocto fait pareil.

```sh
# 2. Retirer le chemin de démarrage EFI de la distribution. Le slot ne garde que grub-mkconfig/update-grub.
for n in $(efibootmgr | sed -n 's/^Boot\([0-9A-F]\{4\}\)\*\{0,1\} debian.*/\1/p'); do
    efibootmgr -q -b "$n" -B
done
apt-mark manual grub-common grub2-common grub-efi-amd64-bin
apt-get purge -y --allow-remove-essential grub-efi-amd64 grub-efi-amd64-signed shim-signed
apt-get autoremove -y --purge
rm -rf /boot/efi/EFI/debian /boot/efi/EFI/BOOT
```

- L'installateur crée une entrée de démarrage `debian` dans la NVRAM et la place en tête. Si on la laisse, le
  micrologiciel saute notre sélecteur et démarre `\EFI\debian\shimx64.efi`. En supprimant l'entrée et le
  répertoire, le micrologiciel se rabat sur le chemin par défaut `\EFI\BOOT\BOOTX64.EFI`.
- `grub-efi-amd64`, `-signed` et `shim-signed` lancent `grub-install` à l'installation et à la mise à jour. Dans
  un slot, un rootfs reçu en mise à jour pourrait écraser l'ESP : on les retire. `grub-efi-amd64-bin`, qui ne
  contient que les modules, reste : `grub-mkimage` s'en sert pour construire le GRUB de l'ESP.
- Les répertoires sont supprimés **après** la purge. Pendant la purge, les scripts de maintenance de
  `shim-signed` et `grub-efi-amd64` relancent `grub-install` (« Installing for x86_64-efi platform » dans le
  journal) et écrivent encore sur l'ESP.
- Sans le `apt-mark manual` préalable, `autoremove` retirerait aussi `grub-common`, installé seulement comme
  dépendance.
- **`--allow-remove-essential` est nécessaire.** Debian marque `grub-efi-amd64-signed` comme `Protected: yes` :
  apt le traite comme essentiel et `-y` seul ne le retire pas. C'est une protection contre la suppression
  accidentelle du chargeur d'amorçage ; ici, remplacer ce chargeur est justement le but, d'où l'autorisation
  explicite.

  ```text
  WARNING: The following essential packages will be removed.
  This should NOT be done unless you know exactly what you are doing!
    grub-efi-amd64-signed
  E: Essential packages were removed and -y was used without --allow-remove-essential.
  ```

```sh
# 3. Côté slot : update-grub écrit une seule entrée qui démarre root=PARTLABEL=rootfs_$SLOT.
for s in 00_header 10_linux 20_linux_xen 25_bli 30_os-prober 30_uefi-firmware; do
    [ -e /etc/grub.d/$s ] && chmod -x /etc/grub.d/$s
done
install -m 0755 $F/10_ab /etc/grub.d/10_ab
echo 'GRUB_CMDLINE_LINUX="console=tty0 console=ttyS0,115200"' > /etc/default/grub
update-grub

# 4. Côté ESP : notre propre GRUB, le sélecteur et grubenv.
install -d /boot/efi/EFI/BOOT
grub-mkimage -O x86_64-efi -p /EFI/BOOT -o /boot/efi/EFI/BOOT/BOOTX64.EFI \
    part_gpt fat ext2 normal configfile loadenv test echo linux gzio sleep halt
install -m 0644 $F/esp-grub.cfg /boot/efi/EFI/BOOT/grub.cfg
grub-editenv /boot/efi/EFI/BOOT/grubenv create
grub-editenv /boot/efi/EFI/BOOT/grubenv set ORDER="A B" A_OK=1 A_TRY=0 B_OK=0 B_TRY=0
```

C'est le contenu de l'article GRUB A/B. La seule différence dans un build d'image est que **`/etc/default/grub` est
réécrit entièrement**. Ce fichier appartenait au paquet `grub-efi-amd64` : la purge ci-dessus l'a supprimé. La
première version modifiait une ligne avec `sed`, et le build s'est arrêté sur « can't read /etc/default/grub ».
`10_ab` ne lit que `GRUB_CMDLINE_LINUX` : le fichier se réduit à cette ligne.

```sh
# 5. RAUC : slots, CA de confiance, mark-good après un démarrage réussi.
install -d /etc/rauc /data/rauc
sed "s/@COMPATIBLE@/$COMPATIBLE/" $F/system.conf > /etc/rauc/system.conf
install -m 0644 $F/ca.cert.pem /etc/rauc/ca.cert.pem
install -m 0644 $F/rauc-mark-good.service /etc/systemd/system/rauc-mark-good.service
systemctl enable rauc-mark-good.service

printf 'IMAGE_VERSION=%s\n' "$IMAGE_VERSION" > /etc/os-image-release
```

Le `system.conf` :

```ini
[system]
compatible=@COMPATIBLE@
bootloader=grub
grubenv=/boot/efi/EFI/BOOT/grubenv
data-directory=/data/rauc
bundle-formats=-plain

[keyring]
path=/etc/rauc/ca.cert.pem

[slot.rootfs.0]
device=/dev/disk/by-partlabel/rootfs_A
type=ext4
bootname=A
resize=true

[slot.rootfs.1]
device=/dev/disk/by-partlabel/rootfs_B
type=ext4
bootname=B
resize=true
```

- Avec `bootloader=grub`, RAUC modifie lui-même dans `grubenv` les variables `ORDER`, `X_OK` et `X_TRY` de
  l'article GRUB A/B.
- `bootname` est comparé à `rauc.slot=A` sur la ligne de commande du noyau. C'est ainsi que RAUC sait sur quel slot
  il a démarré, et il écrit les mises à jour sur l'autre.
- `data-directory` est sur la partition de données. L'état des slots et l'historique des installations, gardés sur
  la racine, disparaîtraient à chaque changement de slot.
- **`resize=true`** : l'image ext4 du bundle est réduite à sa taille minimale (voir plus bas). Après l'avoir écrite
  dans un slot, RAUC l'agrandit avec `resize2fs` jusqu'à la taille de la partition (3 Go). Sans cela, le nouveau slot
  démarre avec une racine presque pleine.
- Les sources de RAUC contiennent un `rauc-mark-good.service`, mais les paquets Debian ne le fournissent pas : on le
  fournit ici.

  ```ini
  [Unit]
  Description=Mark the booted RAUC slot as good
  ConditionKernelCommandLine=rauc.slot
  Requires=boot-complete.target
  After=boot-complete.target
  RequiresMountsFor=/boot/efi

  [Service]
  Type=oneshot
  RemainAfterExit=yes
  ExecStart=/usr/bin/rauc status mark-good

  [Install]
  WantedBy=multi-user.target
  ```

  `boot-complete.target` est le point où systemd exprime « le démarrage a réussi ». Branchez les unités de contrôle
  de santé avec `Before=boot-complete.target` et `RequiredBy=boot-complete.target` : si un contrôle échoue, la cible
  n'est jamais atteinte et mark-good ne s'exécute pas. grubenv garde `X_TRY=1` et le démarrage suivant revient au
  slot précédent. C'est la place des contrôles de santé de l'[article RAUC](/fr/posts/architecture/os/ota-update/rauc/).
- `/etc/os-image-release` indique quel build contient le slot démarré.

## 90-cleanup.sh : retirer ce qui appartient à la VM de build

Une VM fraîchement installée contient plusieurs **valeurs qui ne sont justes que pour elle**. Les centaines
d'appareils issus de l'OVA, et chaque rootfs livré plus tard par RAUC, en héritent.

| Ce qui reste | Problème | Correction |
|---|---|---|
| nom de carte réseau dans la config ifupdown (`enp0s3`) | Nom différent sur un autre hyperviseur ou du vrai matériel : pas de réseau | systemd-networkd, DHCP sur tout `en*`, `eth*` |
| proxy apt (`/etc/apt/apt.conf`) | Les appareils cherchent l'apt-cacher-ng du runner CI | Supprimer la ligne |
| `/etc/machine-id` | Le même ID partout ; identifiants client DHCP, journald et autres se télescopent | Le laisser vide, généré au premier démarrage |
| clés d'hôte SSH | Les mêmes clés partout ; une fuite permet d'usurper tous les appareils | Supprimer, `ssh-keygen -A` au premier démarrage |
| mot de passe root, `PermitRootLogin yes` | N'importe qui entre avec le mot de passe de build | Le verrouiller, clés publiques uniquement |
| journaux d'installation, historique shell | Fuite d'informations sur le build, place perdue | Les vider |

```sh
F=/tmp/files

install -m 0644 $F/20-wired.network /etc/systemd/network/20-wired.network
systemctl enable systemd-networkd systemd-resolved
apt-get purge -y ifupdown dhcpcd-base
rm -f /etc/network/interfaces

sed -i '/Acquire::http::Proxy/d' /etc/apt/apt.conf 2>/dev/null || true
apt-get clean
rm -rf /var/lib/apt/lists/*

truncate -s 0 /etc/machine-id
rm -f /var/lib/dbus/machine-id /etc/ssh/ssh_host_*
install -m 0644 $F/ssh-hostkeys.service /etc/systemd/system/ssh-hostkeys.service
systemctl enable ssh-hostkeys.service

install -d -m 0700 /root/.ssh
install -m 0600 $F/authorized_keys /root/.ssh/authorized_keys
rm -f /etc/ssh/sshd_config.d/90-packer.conf
passwd -l root

find /var/log -type f -exec truncate -s 0 {} +
rm -rf /var/log/installer /root/.bash_history
```

- `machine-id` est laissé sous forme de **fichier vide** plutôt que supprimé. Quand systemd trouve un
  `/etc/machine-id` vide, il considère qu'il s'agit d'un premier démarrage et y écrit un nouvel ID.
- L'unité qui génère les clés d'hôte SSH ne s'exécute que si elles manquent
  (`ConditionPathExists=!/etc/ssh/ssh_host_ed25519_key`).
- **Le build continue alors que le script verrouille le mot de passe root et retire le réglage sshd.** Packer
  exécute les étapes restantes sur la connexion SSH déjà ouverte, et le changement de sshd ne vaut que pour les
  nouvelles connexions.

:::warning Elles sont aussi régénérées lors des mises à jour
L'image rootfs contient elle aussi un `machine-id` vide et un `/etc/ssh` sans clés d'hôte. Quand RAUC met à jour
l'autre slot, **le machine-id et les clés d'hôte SSH sont donc régénérés** au premier démarrage de ce slot. Dans le
test de la partie 3, le machine-id du slot A était `f5c02aef…` et celui du slot B après la mise à jour
`fc311b22…`. Si l'identité d'un appareil repose sur le machine-id, ou si les clients SSH épinglent la clé d'hôte,
les deux changent à chaque mise à jour. Pour les conserver, il faut les stocker sur `/data`, que les mises à jour ne
touchent pas, et les récupérer au démarrage. L'image de cette série ne va pas jusque-là.
:::

## 95-export-rootfs.sh : sortir l'image pour RAUC

```sh
mkdir -p /mnt/rootfs
mount --bind / /mnt/rootfs
size=$(( $(du -sxm /mnt/rootfs | cut -f1) * 12 / 10 + 256 ))
rm -f /data/rootfs.ext4
mkfs.ext4 -q -L rootfs -d /mnt/rootfs /data/rootfs.ext4 "${size}M"
umount /mnt/rootfs
e2fsck -fy /data/rootfs.ext4 >/dev/null
resize2fs -M /data/rootfs.ext4
```

- Remonter la racine une seconde fois avec un **montage bind** laisse de côté tout ce qui est empilé dessus
  (`/proc`, `/sys`, `/dev`, `/run`, `/tmp`, `/boot/efi`, `/data`) et ne montre que **le système de fichiers racine
  lui-même**. Les répertoires de points de montage restent, vides, prêts à servir dans le nouveau slot.
- `mkfs.ext4 -d` crée le système de fichiers et le remplit avec le contenu d'un répertoire. Propriétaires,
  permissions et attributs étendus (capacités de fichiers comprises) sont copiés en root dans la VM : ils sont
  exacts. Construire l'image sur la machine de build à partir d'une archive tar exige root là-bas, et les capacités
  se perdent facilement.
- La sortie va dans `/data`. `/tmp` (tmpfs) est trop petit, et écrire sur la racine ferait que l'image essaierait
  de se contenir elle-même.
- `resize2fs -M` la réduit au minimum. Une fois écrite dans un slot, le `resize=true` de RAUC l'agrandit de nouveau.

Le provisionneur `file` la télécharge ensuite, et la dernière étape en ligne la supprime puis lance `fstrim` pour
rendre les blocs vides : aucune trace d'un fichier image de 1 Go ne reste dans l'OVA.

## Résultats

Cet environnement n'a pas VirtualBox : le vrai modèle a été vérifié avec `packer init`, `packer fmt -check` et
`packer validate`, et le build a tourné avec le builder QEMU. Seule la `source` a changé, pour partir du disque
installé dans la partie 1 ; le bloc `build` est celui de cet article. Les parties propres à VirtualBox,
`vboxmanage` et l'export OVA, n'ont pas été exécutées.

```text
==> qemu.debian: Uploading ./files/ => /tmp/files
==> qemu.debian: Provisioning with shell script: ./scripts/10-packages.sh
==> qemu.debian: Provisioning with shell script: ./scripts/20-ab-layout.sh
==> qemu.debian: Provisioning with shell script: ./scripts/90-cleanup.sh
==> qemu.debian: Provisioning with shell script: ./scripts/95-export-rootfs.sh
==> qemu.debian: Resizing the filesystem on /data/rootfs.ext4 to 264975 (4k) blocks.
==> qemu.debian: Downloading /data/rootfs.ext4 => output/debian-ab-1.0.0/rootfs.ext4
==> qemu.debian: /data: 3.2 GiB (3388547072 bytes) trimmed on /dev/sda4
==> qemu.debian: /boot/efi: 238.6 MiB (250190848 bytes) trimmed on /dev/sda1
==> qemu.debian: /: 2 GiB (2177200128 bytes) trimmed on /dev/sda2
==> Wait completed after 5 minutes 45 seconds
```

- Le provisionnement prend environ 6 minutes. Après `resize2fs -M`, `rootfs.ext4` fait 1,0 Go (264975 blocs).
- Le téléchargement et le `fstrim` ont eu lieu sur la même connexion SSH **après** le verrouillage du mot de passe
  root par le script de nettoyage.
- Cela n'a pas marché du premier coup. Le `Protected: yes` de `grub-efi-amd64-signed`, le `/etc/default/grub`
  supprimé par la purge et le `rauc-mark-good.service` absent de Debian ont chacun arrêté le build une fois ; les
  scripts ci-dessus sont les versions corrigées.

Le disque obtenu a été démarré avec une **NVRAM vide**, comme lorsqu'on importe l'OVA dans une nouvelle VM. Le
micrologiciel trouve `\EFI\BOOT\BOOTX64.EFI` et le sélecteur choisit le slot A.

```text
ab: ORDER=A B A=1/1 B=0/0 -> slot A
[    0.000000] Command line: BOOT_IMAGE=/boot/vmlinuz-6.12.115+deb13.1-amd64 root=PARTLABEL=rootfs_A ro rauc.slot=A console=tty0 console=ttyS0,115200
```

```text
# lsblk -o NAME,SIZE,FSTYPE,PARTLABEL,MOUNTPOINTS /dev/sda
NAME    SIZE FSTYPE PARTLABEL MOUNTPOINTS
sda      10G
├─sda1  243M vfat   esp       /boot/efi
├─sda2  2.9G ext4   rootfs_A  /
├─sda3  2.9G        rootfs_B
└─sda4    4G ext4   data      /data

# find /boot/efi -type f
/boot/efi/EFI/BOOT/BOOTX64.EFI
/boot/efi/EFI/BOOT/grub.cfg
/boot/efi/EFI/BOOT/grubenv

# networkctl --no-legend list
  1 lo     loopback carrier  unmanaged
  2 enp0s2 ether    routable configured

# passwd -S root
root L 2026-11-06 0 99999 7 -1

# rauc status
Compatible:  pfclakr-debian-amd64
Booted from: rootfs.0 (A)
Activated: rootfs.0 (A)
○ [rootfs.1] (/dev/disk/by-partlabel/rootfs_B, ext4, inactive)
      boot status: bad
⏺ [rootfs.0] (/dev/disk/by-partlabel/rootfs_A, ext4, booted)
      boot status: good
```

- Les noms GPT sont là, et l'ESP ne contient plus que les trois fichiers dont le sélecteur a besoin.
- Quel que soit le nom de la carte réseau pendant le build, networkd l'a prise en DHCP. Un nouveau machine-id et de
  nouvelles clés d'hôte SSH ont été générés au premier démarrage, et root est verrouillé (`L`) : seules les clés
  permettent d'entrer.
- `rauc-mark-good.service` s'est exécuté après `boot-complete.target` et a remis le `A_TRY` de grubenv à 0. Le
  slot B est encore vide (`B_OK=0`) et apparaît comme bad. `systemctl --failed` était vide.

L'article suivant lance ce build dans GitLab CI, signe `rootfs.ext4` dans un bundle RAUC et le publie à côté de
l'OVA.
