+++
title = "Construction d'images OS 3 : builder dans GitLab CI et publier un bundle RAUC"
description = "Le build Packer en pipeline GitLab CI : un runner shell avec VirtualBox, une version tirée du tag, la signature du bundle RAUC avec des variables File protégées, le Generic Package Registry et les releases. Création des clés de signature, puis installation du bundle sur un appareil jusqu'au changement de slot."
tags = ["linux", "gitlab", "ci", "packer", "rauc", "ota"]
date = 2026-11-06T15:00:00
+++

Un seul `packer build` de la [partie 2](/fr/posts/architecture/os/image-build/image-2-packer/) produit un OVA
et un `rootfs.ext4`. Cet article passe le tout dans GitLab CI, avec ces objectifs :

- Chaque merge request vérifie que le modèle est valide.
- Chaque commit sur la branche par défaut est vraiment construit, bundle compris.
- **Pousser un tag** construit un OVA et un bundle RAUC signé avec cette version, et les publie en release.

| Job | Quand | Où |
|---|---|---|
| `validate` | MR, branche par défaut | Docker |
| `build` | branche par défaut, tags | runner shell + VirtualBox |
| `bundle` | branche par défaut, tags | Docker |
| `release` | tags seulement | Docker |

## Le runner : pourquoi un executor shell

`validate`, `bundle` et `release` sont des jobs conteneurs ordinaires. **`build` fait exception.** VirtualBox
passe par un module noyau de l'hôte (`vboxdrv`), ce qui le rend difficile à lancer dans un conteneur Docker. On
installe donc un runner avec l'**executor shell** sur une machine qui a VirtualBox et Packer, avec le tag
`virtualbox`.

Sur la machine du runner :

- Installer VirtualBox, Packer et zstd. Ajouter l'utilisateur `gitlab-runner` au groupe `vboxusers`.
- Créer `/var/cache/packer`, propriété de `gitlab-runner`. C'est le cache des ISO.
- Installer apt-cacher-ng. Il écoute sur le port 3142 par défaut ; le preseed de la partie 1 et la variable
  `apt_proxy` de la partie 2 l'utilisent.

:::note Runners shell et répertoire de travail
Un executor shell ne donne pas un conteneur neuf à chaque job. Si le `output/` d'un job précédent est encore là,
Packer s'arrête avec « output directory already exists ». Par défaut, GitLab lance `git clean` au début de chaque
job pour supprimer les fichiers non suivis ; gardez `GIT_CLEAN_FLAGS` à sa valeur par défaut (`-ffdx`) pour que
`output/` soit supprimé même s'il figure dans `.gitignore`.
:::

## Versions et conditions d'exécution

```yaml
stages: [validate, build, bundle, release]

variables:
  COMPATIBLE: pfclakr-debian-amd64
  IMAGE_VERSION: 0.0.0-$CI_COMMIT_SHORT_SHA
  OUT: output/debian-ab-$IMAGE_VERSION

workflow:
  rules:
    - if: $CI_COMMIT_TAG
      variables:
        IMAGE_VERSION: $CI_COMMIT_TAG
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

- `workflow:rules` décide si un pipeline est créé. Seuls les tags, les merge requests et la branche par défaut
  en ont un ; un push sur une branche de fonctionnalité ne déclenche rien tant qu'aucune MR n'est ouverte.
- **La version vient du tag.** Le tag `1.2.0` donne `IMAGE_VERSION=1.2.0` ; sinon c'est `0.0.0-<commit>`. Cette
  même valeur se retrouve dans le nom de la VM Packer, le `--version` de l'OVA, `/etc/os-image-release` et la
  `version` du manifeste RAUC.
- `COMPATIBLE` n'existe aussi qu'ici. Le `system.conf` de l'image et le manifeste du bundle doivent porter la
  même valeur, sinon RAUC refuse l'installation ; deux copies finissent toujours par diverger.

## validate

```yaml
validate:
  stage: validate
  image:
    name: hashicorp/packer:1.16.2
    entrypoint: [""]
  script:
    - packer init .
    - packer fmt -check -diff .
    - packer validate -var root_password=unused .
```

- Le point d'entrée de l'image officielle est `packer` ; on le vide pour que GitLab puisse lancer le script.
- `fmt -check` échoue si le formatage diffère et `-diff` montre les différences.
- `validate` échoue si une variable obligatoire est vide. Le mot de passe n'a aucun sens pour la vérification,
  n'importe quelle valeur convient. L'idée est que cette étape n'ait jamais besoin du vrai secret.

## build

```yaml
build:
  stage: build
  tags: [virtualbox]
  resource_group: virtualbox
  rules:
    - if: $CI_COMMIT_TAG || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  variables:
    PACKER_CACHE_DIR: /var/cache/packer
    PKR_VAR_image_version: $IMAGE_VERSION
    PKR_VAR_compatible: $COMPATIBLE
    PKR_VAR_apt_proxy: http://10.0.2.2:3142
  script:
    - packer init .
    - packer build -color=false -timestamp-ui .
    - zstd -T0 --rm "$OUT/rootfs.ext4"
  after_script:
    - VBoxManage unregistervm "debian-ab-$IMAGE_VERSION" --delete 2>/dev/null || true
  artifacts:
    paths:
      - $OUT/
    expire_in: 1 week
```

- **`resource_group`** : un seul job d'un même groupe de ressources s'exécute à la fois. Même avec un
  `concurrent` plus élevé sur le runner, les builds de VM ne se disputent pas CPU et mémoire.
- **`PKR_VAR_*`** : des variables que Packer lit tout seul. `PKR_VAR_root_password` n'est pas écrite ici ; elle
  va dans les **variables CI/CD** du projet, en Masked. La partie 2 l'a déclarée `sensitive = true`, elle reste
  donc aussi hors du journal de Packer.
- `PACKER_CACHE_DIR` : par défaut, c'est `packer_cache` dans le répertoire de travail, que `git clean` supprime.
  En le plaçant dans un répertoire fixe du runner, l'ISO n'est téléchargée qu'une fois.
- `-color=false` évite les codes ANSI dans le journal du job, et `-timestamp-ui` horodate chaque ligne pour
  repérer les étapes lentes.
- **zstd** : `rootfs.ext4` dépasse 1 Go. Les artefacts GitLab ont une taille maximale (réglage de l'instance) et
  sont envoyés puis téléchargés entre les jobs : plus ils sont petits, plus c'est rapide. Une image ext4 contient
  beaucoup d'espace vide et se compresse bien.
- **`after_script`** : si le job est annulé ou si le runner tombe en cours de build, Packer ne peut pas supprimer
  la VM et elle reste enregistrée dans VirtualBox. `after_script` s'exécute que le job réussisse ou non, et la
  supprime pour que le build suivant n'échoue pas sur une VM du même nom.

## Clés de signature

Les bundles RAUC sont signés avec des certificats X.509. On crée une CA et un certificat de signature émis par
elle.

```bash
# CA : va dans /etc/rauc/ca.cert.pem sur l'appareil. Clé conservée hors ligne.
openssl req -x509 -newkey rsa:4096 -nodes -days 3650 \
  -keyout ca.key.pem -out ca.cert.pem -subj "/O=pfclakr/CN=pfclakr os-image CA"

# Signature : utilisée par la CI pour signer les bundles.
openssl req -newkey rsa:4096 -nodes \
  -keyout signing.key.pem -out signing.csr -subj "/O=pfclakr/CN=pfclakr os-image signing"
openssl x509 -req -in signing.csr -CA ca.cert.pem -CAkey ca.key.pem -CAcreateserial \
  -days 1825 -out signing.cert.pem
```

- `ca.cert.pem` (public) va dans le `files/` du dépôt et est intégré à l'image.
- `signing.cert.pem` et `signing.key.pem` vont dans les variables CI/CD du projet, **de type File**, avec
  **Protected** activé. Avec File, la variable contient dans le job le chemin d'un fichier temporaire qui porte
  le contenu, à passer tel quel à `--key=`. Protected ne donne la variable qu'aux pipelines des branches et tags
  protégés : cela empêche de modifier `.gitlab-ci.yml` sur une branche de MR pour afficher la clé. Il faut aussi
  protéger les tags (Protected tags).
- **La clé de la CA ne va pas dans la CI.** Si la clé de signature fuit, la CA révoque ce certificat et en émet
  un nouveau. Si la clé de la CA fuit, il faut changer le trousseau de tous les appareils. Un fichier de trousseau
  RAUC peut contenir plusieurs certificats : pour changer de CA, on distribue d'abord la nouvelle CA dans une mise
  à jour, puis on bascule.

## bundle

```yaml
bundle:
  stage: bundle
  image: debian:trixie
  needs: [build]
  rules:
    - if: $CI_COMMIT_TAG || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  before_script:
    - apt-get update -q
    - apt-get install -y -q --no-install-recommends rauc squashfs-tools zstd
  script:
    - mkdir bundle
    - zstd -d "$OUT/rootfs.ext4.zst" -o bundle/rootfs.ext4
    - sed -e "s/@COMPATIBLE@/$COMPATIBLE/" -e "s/@VERSION@/$IMAGE_VERSION/"
        rauc/manifest.raucm.in > bundle/manifest.raucm
    - rauc bundle --cert="$RAUC_CERT" --key="$RAUC_KEY" bundle "$OUT/debian-ab-$IMAGE_VERSION.raucb"
    - rauc info --keyring=files/ca.cert.pem "$OUT/debian-ab-$IMAGE_VERSION.raucb"
  artifacts:
    paths:
      - $OUT/*.raucb
    expire_in: 1 week
```

Le modèle de manifeste :

```ini
[update]
compatible=@COMPATIBLE@
version=@VERSION@

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

- `needs: [build]` récupère les artefacts du job build.
- `rauc bundle` met le répertoire dans un squashfs et le signe, d'où le besoin de `squashfs-tools`
  (`mksquashfs`).
- `format=verity` ajoute un arbre de hachage dm-verity au bundle. Un appareil peut alors installer en ne lisant
  en HTTP que les parties nécessaires (streaming), sans télécharger tout le bundle, et chaque bloc reste vérifié.
  C'est le pendant de `bundle-formats=-plain` dans le `system.conf` de la partie 2.
- Le `rauc info` final **revérifie la signature du bundle** avec la CA du dépôt. Si la variable CI contient un
  certificat émis par une autre CA, l'échec arrive ici et non sur l'appareil.

## release

```yaml
release:
  stage: release
  image: alpine:3.22
  needs: [build, bundle]
  rules:
    - if: $CI_COMMIT_TAG
  variables:
    PKG: $CI_API_V4_URL/projects/$CI_PROJECT_ID/packages/generic/debian-ab/$CI_COMMIT_TAG
  script:
    - apk add --no-cache curl jq
    - cd "$OUT"
    - sha256sum ./*.ova ./*.raucb > SHA256SUMS
    - |
      for f in *.ova *.raucb SHA256SUMS; do
        curl --fail-with-body -H "JOB-TOKEN: $CI_JOB_TOKEN" --upload-file "$f" "$PKG/$f"
      done
    - |
      jq -n --arg tag "$CI_COMMIT_TAG" --arg pkg "$PKG" '{
        tag_name: $tag, name: $tag,
        assets: { links: [
          { name: "OVA",         url: "\($pkg)/debian-ab-\($tag).ova",   link_type: "image" },
          { name: "RAUC bundle", url: "\($pkg)/debian-ab-\($tag).raucb", link_type: "package" },
          { name: "SHA256SUMS",  url: "\($pkg)/SHA256SUMS",              link_type: "other" }
        ] } }' |
      curl --fail-with-body -H "JOB-TOKEN: $CI_JOB_TOKEN" -H "Content-Type: application/json" \
        --data @- "$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"
```

- **Les artefacts ne servent pas au stockage.** Ils disparaissent après `expire_in` et leur taille est limitée.
  Les fichiers de release vont dans le **Generic Package Registry** : un PUT sur `paquet/version/fichier`,
  authentifié par le jeton du job (`CI_JOB_TOKEN`).
- Ensuite, l'API Releases crée une release sur le tag, avec les fichiers du registre en liens. La page Releases
  du projet affiche l'OVA, le bundle et les sommes de contrôle ensemble.
- `--fail-with-body` : en cas d'erreur HTTP, curl sort avec un code non nul et affiche le message du serveur. Un
  `curl` simple sort avec 0 sur un 404 ou un 413, et le job a l'air de réussir.

## Sur l'appareil

On importe l'OVA dans VirtualBox et on la démarre : elle boote sur le slot A. En installant le bundle d'une
nouvelle release, RAUC écrit le slot B et met à jour grubenv.

```bash
rauc install debian-ab-1.1.0.raucb
reboot
# se reconnecter
rauc status
cat /etc/os-image-release
```

Le disque de la partie 2 a été démarré dans QEMU. Le bundle 1.1.0 vient du `rootfs.ext4` du même build, où
seul `/etc/os-image-release` est passé à 1.1.0, avec les mêmes commandes que le job `bundle` ci-dessus et une CA de
test. L'image ext4 de 1,0 Go est devenue un bundle de 446 Mo.

```text
# rauc install /data/debian-ab-1.1.0.raucb
installing
 10% Checking bundle
 20% Verifying signature done.
 30% Checking manifest contents done.
 40% Determining target install group done.
 46% Checking slot rootfs.1 (B) done.
 99% Copying image to rootfs.1 done.
100% Installing done.
Installing `/data/debian-ab-1.1.0.raucb` succeeded

# grub-editenv /boot/efi/EFI/BOOT/grubenv list
ORDER=B A
A_OK=1
A_TRY=0
B_OK=1
B_TRY=0
```

RAUC a écrit l'image dans le slot B, agrandi le système de fichiers comme le demande `resize=true` dans
`system.conf`, puis placé B en tête dans grubenv avec `B_OK=1`. L'installation a pris 41 secondes. Au redémarrage, le
sélecteur choisit B.

```text
ab: ORDER=B A A=1/0 B=1/1 -> slot B

# cat /etc/os-image-release
IMAGE_VERSION=1.1.0
# df -h /
/dev/sda3       2.8G  726M  2.0G  27% /
# rauc status
Booted from: rootfs.1 (B)
Activated: rootfs.1 (B)
⏺ [rootfs.1] (/dev/disk/by-partlabel/rootfs_B, ext4, booted)
      boot status: good
○ [rootfs.0] (/dev/disk/by-partlabel/rootfs_A, ext4, inactive)
      boot status: good
```

- L'image de 1,0 Go occupe maintenant toute la partition de 2,8 Go, grâce à `resize=true`.
- Le sélecteur a enregistré `B_TRY=1` avant de démarrer, et `rauc-mark-good.service` l'a remis à 0.

Le **retour arrière** a aussi été vérifié. Sur le slot B, `rauc-mark-good.service` a été désactivé pour simuler un
contrôle de santé raté, puis deux redémarrages.

```text
ab: ORDER=B A A=1/0 B=1/1 -> slot B     # redémarrage 1 : B, pas de mark-good → B_TRY=1 reste
ab: ORDER=B A A=1/1 B=1/1 -> slot A     # redémarrage 2 : B déjà essayé → A

# cat /etc/os-image-release
IMAGE_VERSION=1.0.0
# rauc status
Booted from: rootfs.0 (A)
Activated: rootfs.0 (A)
○ [rootfs.1] (/dev/disk/by-partlabel/rootfs_B, ext4, inactive)
      boot status: bad
⏺ [rootfs.0] (/dev/disk/by-partlabel/rootfs_A, ext4, booted)
      boot status: good
```

Un slot qui n'exécute jamais mark-good garde `B_TRY=1`, et le sélecteur le saute au démarrage suivant. L'appareil est
revenu sur le slot A avec la 1.0.0, et RAUC signale B comme bad. Personne n'a eu à intervenir.

## Bilan

Le flux construit au fil des trois parties :

1. **preseed** installe netinst sans intervention, avec la disposition de partitions A/B.
2. **Packer** lance cette installation dans une VM, ajoute le démarrage A/B et RAUC, retire les valeurs propres
   à la VM de build, et exporte ensemble un OVA et une image rootfs.
3. **GitLab CI** lance le build à partir d'un seul tag, fait du rootfs un bundle RAUC signé et le publie à côté
   de l'OVA.

L'usine flashe l'OVA, et les appareils sur le terrain reçoivent le bundle issu du même build. Les deux livrables
sortent d'un seul build : « image d'usine 1.1 » et « mise à jour 1.1 » ne peuvent pas avoir un contenu différent.
