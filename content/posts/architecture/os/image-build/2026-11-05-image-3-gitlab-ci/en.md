+++
title = "OS image build 3: building in GitLab CI and releasing a RAUC bundle"
description = "The Packer build as a GitLab CI pipeline: a shell runner with VirtualBox, a version taken from the tag, RAUC bundle signing with protected File variables, the Generic Package Registry and releases. Creating the signing keys, and installing the bundle on a device until the slot switches."
tags = ["linux", "gitlab", "ci", "packer", "rauc", "ota"]
draft = true
date = 2026-11-05T15:00:00
+++

One `packer build` from [part 2](/posts/architecture/os/image-build/image-2-packer/) produces an OVA and a
`rootfs.ext4`. This post moves that into GitLab CI, with these goals:

- Every merge request checks that the template is valid.
- Every commit on the default branch is really built, bundle included.
- **Pushing a tag** builds an OVA and a signed RAUC bundle with that version and publishes them as a release.

```text
validate ──► build ──► bundle ──► release
 (MR, main)   (main, tags)          (tags only)
 docker       shell+VirtualBox      docker
```

## The runner: why a shell executor

`validate`, `bundle` and `release` are ordinary container jobs. **`build` is the odd one out.** VirtualBox
works through a host kernel module (`vboxdrv`), which makes it hard to run inside a Docker container. So a
runner with the **shell executor** goes on a machine that has VirtualBox and Packer installed, with the tag
`virtualbox`.

On the runner machine:

- Install VirtualBox, Packer and zstd. Add the `gitlab-runner` user to the `vboxusers` group.
- Create `/var/cache/packer` owned by `gitlab-runner`. This is the ISO cache.
- Install apt-cacher-ng. It listens on port 3142 by default, and part 1's preseed and part 2's `apt_proxy`
  variable use it.

:::note Shell runners and the working directory
A shell executor does not give each job a fresh container. If a previous job's `output/` is still there,
Packer stops with "output directory already exists". By default GitLab runs `git clean` at the start of a job
to remove untracked files; keep `GIT_CLEAN_FLAGS` at its default (`-ffdx`) so `output/` is removed even
though it is in `.gitignore`.
:::

## Versions and when jobs run

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

- `workflow:rules` decides whether a pipeline is created at all. Only tags, merge requests and the default
  branch get one; a push to a feature branch does nothing until an MR is opened.
- **The version comes from the tag.** Tagging `1.2.0` makes `IMAGE_VERSION` `1.2.0`; otherwise it is
  `0.0.0-<commit>`. That one value ends up in the Packer VM name, the OVA's `--version`,
  `/etc/os-image-release` and the `version` in the RAUC manifest.
- `COMPATIBLE` also lives only here. The image's `system.conf` and the bundle manifest must carry the same
  value or RAUC refuses the install, and two copies drift apart sooner or later.

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

- The official image's entrypoint is `packer`, so it is cleared to let GitLab run the script.
- `fmt -check` fails on formatting differences and `-diff` shows them.
- `validate` fails if a required variable is empty. The password means nothing to the check, so any value will
  do. The point is that this stage never needs the real secret.

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

- **`resource_group`**: only one job with a given resource group runs at a time. Even with a higher runner
  `concurrent`, VM builds do not fight over CPU and memory.
- **`PKR_VAR_*`**: variables Packer reads on its own. `PKR_VAR_root_password` is not written here; it goes into
  the project's **CI/CD variables** as Masked. Part 2 declared it `sensitive = true`, so it stays out of Packer's
  log too.
- `PACKER_CACHE_DIR`: the default is `packer_cache` inside the working directory, which `git clean` removes.
  Moving it to a fixed directory on the runner means the ISO is downloaded once.
- `-color=false` keeps ANSI codes out of the job log, and `-timestamp-ui` prefixes every line with the time so
  slow steps are visible.
- **zstd**: `rootfs.ext4` is over 1 GB. GitLab artifacts have a maximum size (an instance setting) and are
  uploaded and downloaded between jobs, so smaller is faster. An ext4 image has plenty of empty space and
  compresses well.
- **`after_script`**: if the job is cancelled or the runner drops mid-build, Packer cannot delete the VM and it
  stays registered in VirtualBox. `after_script` runs whether the job succeeded or not, and removes it so the
  next build does not fail on a VM with the same name.

## Signing keys

RAUC bundles are signed with X.509 certificates. Create a CA and a signing certificate issued by it.

```bash
# CA: goes into /etc/rauc/ca.cert.pem on the device. Keep the key offline.
openssl req -x509 -newkey rsa:4096 -nodes -days 3650 \
  -keyout ca.key.pem -out ca.cert.pem -subj "/O=pfclakr/CN=pfclakr os-image CA"

# Signing: used by CI to sign bundles.
openssl req -newkey rsa:4096 -nodes \
  -keyout signing.key.pem -out signing.csr -subj "/O=pfclakr/CN=pfclakr os-image signing"
openssl x509 -req -in signing.csr -CA ca.cert.pem -CAkey ca.key.pem -CAcreateserial \
  -days 1825 -out signing.cert.pem
```

- `ca.cert.pem` (public) goes into the repository's `files/` and is baked into the image.
- `signing.cert.pem` and `signing.key.pem` go into the project's CI/CD variables, **of type File**, with
  **Protected** on. With File, the variable inside the job is the path of a temporary file holding the content,
  which can be passed straight to `--key=`. Protected only exposes the variable to pipelines on protected
  branches and tags, which stops someone from editing `.gitlab-ci.yml` on an MR branch to print the key.
  Protect the tags as well (Protected tags).
- **The CA key does not go into CI.** If the signing key leaks, the CA revokes that certificate and issues a new
  one. If the CA key leaks, every device's keyring has to change. A RAUC keyring file can hold several
  certificates, so a CA rotation ships the new CA in an update first and switches afterwards.

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

The manifest template:

```ini
[update]
compatible=@COMPATIBLE@
version=@VERSION@

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

- `needs: [build]` pulls in the build job's artifacts.
- `rauc bundle` packs the directory into a squashfs and signs it, so it needs `squashfs-tools` (`mksquashfs`).
- `format=verity` adds a dm-verity hash tree to the bundle. A device can then install while reading only the
  parts it needs over HTTP (streaming) instead of downloading the whole bundle, and every block is still
  verified. It pairs with `bundle-formats=-plain` in part 2's `system.conf`.
- The final `rauc info` **checks the new bundle's signature again** against the repository's CA. If the CI
  variable holds a certificate issued by some other CA, this fails here instead of on the device.

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

- **Artifacts are not storage.** They disappear after `expire_in` and have a small size limit. Release files go
  to the **Generic Package Registry**: a PUT to `package/version/filename`, authenticated with the job token
  (`CI_JOB_TOKEN`).
- Then the Releases API creates a release on the tag with the registry files attached as links. The project's
  Releases page shows the OVA, the bundle and the checksums together.
- `--fail-with-body`: on an HTTP error curl exits non-zero and prints the server's error message. Plain `curl`
  exits 0 on a 404 or 413 and the job looks successful.

## On the device

Import the OVA into VirtualBox and start it, and it boots slot A. Installing a new release's bundle makes RAUC
write slot B and update grubenv.

```bash
rauc install debian-ab-1.1.0.raucb
reboot
# log in again
rauc status
cat /etc/os-image-release
```

@@DEVICE@@

## Summary

The flow built over the three parts:

1. **preseed** installs netinst unattended, with the A/B partition layout.
2. **Packer** runs that install in a VM, adds A/B booting and RAUC, removes values that only fit the build VM,
   and exports an OVA and a rootfs image together.
3. **GitLab CI** runs the build from a single tag, turns the rootfs into a signed RAUC bundle and releases it
   next to the OVA.

The factory flashes the OVA, and devices in the field receive the bundle from the same build. Both outputs come
from one build, so "factory image 1.1" and "update 1.1" cannot have different contents.
