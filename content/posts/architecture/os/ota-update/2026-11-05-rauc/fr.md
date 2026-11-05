+++
title = "RAUC : d'un bundle signé à un slot démarré"
description = "Comment RAUC implémente les mises à jour A/B en Linux embarqué : décrire les slots dans system.conf, construire et signer les bundles, ce que fait vraiment rauc install, le côté U-Boot, et l'étape mark-good."
tags = ["linux", "embedded", "ota", "rauc", "yocto"]
+++

L'[article sur les partitions A/B](/fr/posts/architecture/os/ota-update/ab-partitions/) décrivait le
schéma : deux slots, un drapeau dans le bootloader, un comptage des démarrages, et une vérification de
santé qui marque le nouveau slot comme bon. Il faut encore que quelqu'un vérifie la mise à jour,
l'écrive dans la bonne partition et bascule l'état du bootloader au bon moment. En Linux embarqué, un
choix courant pour ce travail est **RAUC** (Robust Auto-Update Controller), un client de mise à jour
open source lancé chez Pengutronix.

## Ce que fait RAUC, et ce qu'il ne fait pas

RAUC tourne sur l'appareil, comme service avec une API D-Bus (`de.pengutronix.rauc.Installer`) et un
outil en ligne de commande. Son rôle est étroit :

- vérifier qu'un **bundle** de mise à jour est signé par une clé de confiance et destiné à cet appareil ;
- écrire les images qu'il contient dans les **slots inactifs** ;
- dialoguer avec le **bootloader** pour basculer de slot, puis marquer un slot comme bon ou mauvais.

Il ne décide pas *quand* mettre à jour, ne télécharge pas selon un calendrier et ne gère pas de flotte.
C'est le rôle d'une couche au-dessus : un serveur comme Eclipse hawkBit avec `rauc-hawkbit-updater`
sur l'appareil, ou votre propre agent qui appelle l'API D-Bus.

## Décrire l'appareil : `/etc/rauc/system.conf`

RAUC apprend le partitionnement grâce à un seul fichier dans le système de fichiers racine :

```ini
[system]
compatible=acme-robot-v2
bootloader=uboot
# où RAUC garde son état (anciennes versions : statusfile=)
data-directory=/data/rauc
# accepter uniquement les bundles verity/crypt, pas l'ancien format « plain »
bundle-formats=-plain

[keyring]
path=/etc/rauc/ca.cert.pem

[slot.rootfs.0]
device=/dev/mmcblk0p2
type=ext4
bootname=A

[slot.rootfs.1]
device=/dev/mmcblk0p3
type=ext4
bootname=B
```

- **`compatible`** identifie le produit. Un bundle dont le manifeste a un autre `compatible` est
  refusé : impossible d'installer par erreur le bundle d'une autre carte.
- Les **slots** s'appellent `slot.<classe>.<index>`. Les slots d'une même classe sont des copies
  interchangeables : `rootfs.0` et `rootfs.1` sont les systèmes de fichiers racine A et B.
- **`bootname`** est le nom que le bootloader donne à ce slot (`A`, `B`). C'est le lien entre la vue de
  RAUC et le `BOOT_ORDER` du bootloader.
- D'autres slots peuvent suivre un slot racine avec `parent=`, par exemple une partition applicative
  qui doit toujours être mise à jour avec son rootfs.

## Les bundles : ce qu'on livre

Un bundle (`.raucb`) est une image SquashFS qui contient un **manifeste** et les images, avec une
**signature CMS** (certificats X.509) attachée. Le manifeste décrit le contenu :

```ini
[update]
compatible=acme-robot-v2
version=2026.11.0

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

Construire et inspecter un bundle :

```bash
mkdir bundle && cp rootfs.ext4 manifest.raucm bundle/
rauc bundle --cert=release.cert.pem --key=release.key.pem bundle/ update.raucb
rauc info --keyring=ca.cert.pem update.raucb
```

Le format **`verity`** est celui à utiliser. La signature couvre le manifeste, qui contient lui-même le
hash racine d'un arbre de hachage dm-verity calculé sur le bundle. RAUC peut alors vérifier chaque bloc
au moment où il le lit, au lieu de faire confiance à un bundle vérifié une fois et qui pourrait changer
ensuite sur le disque. C'est aussi ce qui permet le **streaming** : `rauc install https://…` lit le
bundle en HTTP et l'installe sans le stocker d'abord sur l'appareil (cela repose sur le support NBD du noyau).

Sous Yocto, la couche `meta-rauc` fournit une classe `bundle` : un bundle n'est qu'une recette de plus.

```bitbake
inherit bundle

RAUC_BUNDLE_COMPATIBLE = "acme-robot-v2"
RAUC_BUNDLE_FORMAT = "verity"
RAUC_BUNDLE_SLOTS = "rootfs"
RAUC_SLOT_rootfs = "acme-image"
RAUC_KEY_FILE = "${THISDIR}/files/release.key.pem"
RAUC_CERT_FILE = "${THISDIR}/files/release.cert.pem"
```

## Ce que fait vraiment `rauc install`

```bash
rauc install /data/update.raucb
```

1. **Vérifier** la signature du bundle avec le trousseau (la chaîne doit mener à une CA de confiance).
2. **Contrôler** que le `compatible` du manifeste correspond à `system.conf`.
3. **Trouver le slot démarré**, normalement grâce à `rauc.slot=A` sur la ligne de commande du noyau
   (posé par le bootloader), ou en comparant le périphérique `root=`. La cible est l'autre slot de chaque classe.
4. **Marquer le slot cible comme mauvais** dans le bootloader *avant* d'écrire, pour qu'un slot à moitié
   écrit ne puisse jamais démarrer, même si le courant coupe au milieu.
5. **Écrire** chaque image dans son slot (pour une image `ext4` sur un slot `ext4`, une copie par blocs),
   en exécutant les hooks d'installation éventuels.
6. **Marquer le slot cible comme actif** : le placer en tête de l'ordre de démarrage avec un nouveau
   nombre de tentatives.

RAUC ne redémarre pas tout seul ; c'est votre agent de mise à jour qui décide quand. `rauc status`
montre le résultat :

```bash
rauc status --detailed
# Compatible:  acme-robot-v2
# Booted from: rootfs.0 (A)
# Activated:   rootfs.1 (B)
# ...
```

## Côté bootloader : U-Boot

Avec `bootloader=uboot`, RAUC lit et écrit les variables d'environnement d'U-Boot depuis Linux avec
`fw_printenv`/`fw_setenv` (libubootenv, configuré par `/etc/fw_env.config`). Il utilise exactement les
variables de l'article A/B : `BOOT_ORDER` et `BOOT_<bootname>_LEFT`. Le script de démarrage côté U-Boot
implémente le comptage :

```bash
test -n "${BOOT_ORDER}" || setenv BOOT_ORDER "A B"
test -n "${BOOT_A_LEFT}" || setenv BOOT_A_LEFT 3
test -n "${BOOT_B_LEFT}" || setenv BOOT_B_LEFT 3

setenv bootargs
for BOOT_SLOT in "${BOOT_ORDER}"; do
  if test "x${bootargs}" != "x"; then
    # un slot a déjà été choisi
  elif test "x${BOOT_SLOT}" = "xA"; then
    if test ${BOOT_A_LEFT} -gt 0; then
      setexpr BOOT_A_LEFT ${BOOT_A_LEFT} - 1
      setenv bootargs "root=/dev/mmcblk0p2 rauc.slot=A"
    fi
  elif test "x${BOOT_SLOT}" = "xB"; then
    if test ${BOOT_B_LEFT} -gt 0; then
      setexpr BOOT_B_LEFT ${BOOT_B_LEFT} - 1
      setenv bootargs "root=/dev/mmcblk0p3 rauc.slot=B"
    fi
  fi
done

if test -n "${bootargs}"; then
  saveenv
else
  echo "No bootable slot left"; reset
fi
```

L'argument `rauc.slot=` boucle la boucle : c'est ainsi que RAUC sait, une fois Linux démarré, depuis
quel slot il tourne. Les autres backends suivent la même idée avec un stockage différent : barebox a
`bootchooser`, GRUB utilise `grubenv`, EFI des entrées de démarrage, et `custom` appelle votre propre script.

## Marquer le slot comme bon

Après une mise à jour, le nouveau slot n'a que quelques tentatives. Quelque chose dans l'espace
utilisateur doit confirmer que le système fonctionne, puis appeler :

```bash
rauc status mark-good      # remettre les tentatives du slot démarré
rauc status mark-bad       # l'abandonner tout de suite (le prochain démarrage revient en arrière)
rauc status mark-active other
```

Une unité systemd lancée après les services importants est une façon simple de le faire :

```ini
[Unit]
Description=Mark the booted slot good once the robot is healthy
After=robot-core.service network-online.target
Requires=robot-core.service

[Service]
Type=oneshot
ExecStartPre=/usr/bin/robot-healthcheck --timeout 300
ExecStart=/usr/bin/rauc status mark-good

[Install]
WantedBy=multi-user.target
```

Si `robot-healthcheck` échoue, `mark-good` ne s'exécute jamais ; avec un watchdog matériel pour les cas
de blocage, le bootloader finit par revenir tout seul au slot précédent.

## Ce qui compte une fois qu'il y a une flotte

- **Les clés.** Séparez les certificats de développement et de production, avec la clé de production
  hors ligne ou dans un HSM. Le trousseau peut contenir plusieurs CA : c'est ainsi qu'on fait tourner
  une clé sans laisser d'appareils en plan.
- **Testez les chemins d'échec, pas seulement le cas nominal.** Coupez le courant pendant une
  installation, livrez un bundle de test qui échoue volontairement à sa vérification de santé, et
  vérifiez que l'appareil revient sur l'ancien slot. Idéalement en CI, sur du vrai matériel.
- **Données et retour arrière.** RAUC remplace des slots ; il ne fait rien pour la partition de données.
  Les migrations doivent survivre à une nouvelle exécution de l'ancienne version.
- **Bande passante.** Avec les bundles `verity` et le streaming, les mises à jour *adaptatives*
  (`adaptive=block-hash-index` dans le manifeste) permettent à RAUC de réutiliser les blocs déjà présents
  sur l'appareil et de ne télécharger que le reste, ce qui compte beaucoup sur une liaison cellulaire.
- **Les hooks sont du code qui tourne en root pendant les mises à jour.** Gardez-les petits, idempotents,
  et couverts par les mêmes tests que le reste de la mise à jour.
