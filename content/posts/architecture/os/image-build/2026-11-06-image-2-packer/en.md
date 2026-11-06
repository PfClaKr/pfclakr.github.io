+++
title = "OS image build 2: an OVA and a RAUC rootfs with Packer"
description = "A Packer virtualbox-iso template that installs Debian netinst, adds the A/B layout and RAUC, removes everything that only belongs to the build VM, and exports an OVA and a rootfs.ext4 together. The template and provisioning scripts line by line."
tags = ["linux", "debian", "packer", "virtualbox", "rauc", "ota"]
date = 2026-11-06T14:00:00
+++

The preseed from [part 1](/posts/architecture/os/image-build/image-1-preseed/) only covers "Debian installs
itself without a person". This post wraps that install in Packer, turns the finished VM into a product image, and
exports two outputs.

- **OVA**: the factory image for new VMs or devices. It holds the ESP, slot A (filled), slot B (empty) and the
  data partition.
- **rootfs.ext4**: just the root filesystem of the same build. Part 3 signs it into a RAUC bundle for devices
  already in the field.

## Repository layout

```text
os-image/
├── debian.pkr.hcl            source (the VM) and build (the steps)
├── variables.pkr.hcl
├── http/
│   └── preseed.pkrtpl        part 1's preseed, filled in by templatefile()
├── files/                    files uploaded into the VM and installed
│   ├── esp-grub.cfg          the slot selector on the ESP (GRUB A/B post)
│   ├── 10_ab                 the update-grub script inside a slot (GRUB A/B post)
│   ├── system.conf           RAUC slot definitions
│   ├── ca.cert.pem           CA certificate that verifies bundle signatures
│   ├── authorized_keys       SSH public key for the finished image
│   ├── rauc-mark-good.service
│   ├── 20-wired.network
│   └── ssh-hostkeys.service
├── scripts/
│   ├── 10-packages.sh
│   ├── 20-ab-layout.sh
│   ├── 90-cleanup.sh
│   └── 95-export-rootfs.sh
├── rauc/manifest.raucm.in    part 3
└── .gitlab-ci.yml            part 3
```

## What Packer does

`packer build` drives the `virtualbox-iso` builder through these steps:

1. Download the ISO, check its checksum and keep it in the cache (`PACKER_CACHE_DIR`).
2. Create the VM and attach the disk and the ISO.
3. Start an **HTTP server** to serve the preseed.
4. Power on the VM and **type `boot_command` as keystrokes.**
5. Once the install has finished and rebooted, **wait until SSH answers**.
6. Run the provisioners (file uploads, shell scripts) in order.
7. Power the VM off with `shutdown_command`, **export it as an OVA**, and delete the VM.

## Plugins and variables

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

Builders are plugins shipped separately from Packer itself. `packer init .` reads `required_plugins`, downloads
them and installs them under `~/.config/packer/plugins`. Pinning `~> 1.1` keeps CI from suddenly picking up a 2.x
one day.

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

- Packer never prints the value of a `sensitive = true` variable in its logs. The value is passed through the
  environment variable `PKR_VAR_root_password` rather than `-var`. Packer reads `PKR_VAR_<name>` on its own, which
  fits CI variables well.
- **Why two ISO URLs**: when a point release (13.8) comes out, Debian moves the previous ISO from `release/` to
  `archive/`. `iso_urls` is tried in order, so a template pinned to 13.7.0 keeps working after the next release.
  The `current/` URL is not used: its content changes and the checksum stops matching.
- The checksum can point at a remote `file:…/SHA256SUMS`, but writing the value down means the repository alone
  tells you which ISO a build used.

## source: the VM

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
  # (continued)
```

- **`firmware = "efi"`**: the A/B selector lives at `\EFI\BOOT\BOOTX64.EFI` on the ESP, so the VM must boot UEFI.
- **10 GB disk**: following part 1's recipe, a 256 MB ESP, two 3 GB slots, and the rest for data. The data
  partition also serves as scratch space for the rootfs image during the build (see below).
- **`hard_drive_discard`, `hard_drive_nonrotational`**: present the disk as an SSD and pass TRIM through to
  VirtualBox. A final `fstrim` then drops deleted blocks from the disk image too, which makes the OVA smaller.
- **`guest_additions_mode = "disable"`**: VirtualBox Guest Additions do not belong in a product image. The default
  uploads their ISO, so it is turned off.

### Serving the preseed: http_content

```hcl
  http_content = {
    "/preseed.cfg" = templatefile("${path.root}/http/preseed.pkrtpl", {
      hostname      = "debian"
      apt_proxy     = var.apt_proxy
      root_password = var.root_password
    })
  }
```

`http_directory` serves a directory as is; `http_content` takes **paths and contents directly**.
`templatefile()` fills placeholders like `${root_password}` in the preseed and the result is served from memory,
so no preseed file with the password in it ever lands on disk.

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

- After `boot_wait`, the strings are sent **as keystrokes**. Special keys like `<enter>` and `<esc>` mix with
  `<wait>` (1 second) and `<wait5>` (5 seconds). The array is simply concatenated; the line breaks are only for
  reading.
- `c` drops from the netinst ISO's GRUB menu to the command line, and the three lines from part 1 are typed as is.
- Packer fills in `{{ .HTTPIP }}` and `{{ .HTTPPort }}`. Under VirtualBox NAT the guest sees the host as
  `10.0.2.2`, and the port is picked per build between 8000 and 9000.
- **If a `<wait>` is too short**, the next line arrives before GRUB has taken all the keys. On a slow runner, give
  the first `boot_wait` some slack. One run with `headless = false` shows right away whether the keystrokes landed.

### NAT in VirtualBox 7

```hcl
  vboxmanage = [
    ["modifyvm", "{{ .Name }}", "--nat-localhostreachable1", "on"],
  ]
```

`vboxmanage` is a list of `VBoxManage` commands run after the VM is created and before it is powered on. Since
VirtualBox 7.0, a NAT guest reaching the host's loopback through `10.0.2.2` is **off by default**. Without this, the
installer cannot fetch the preseed and stops on the "failed to retrieve the preconfiguration file" screen. An
apt-cacher-ng on the runner needs it for the same reason.

### SSH and export

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

- `ssh_timeout` covers **the entire install**. Packer starts waiting for SSH as soon as the VM is on, and sshd only
  comes up after the install has finished and rebooted. A slow mirror can mean 20–30 minutes, so be generous.
- With `format = "ova"`, the final `VBoxManage export` packs the OVF descriptor, the VMDK and a manifest into one
  file. `export_opts` are appended to that command as is. `--manifest` adds per-file SHA256 sums inside the OVA,
  and `--product` and `--version` show up in the import dialog.

## build: the steps

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

- When `source` ends in `/`, **the directory's contents** go into `destination`. Without the slash, the `files`
  directory itself ends up inside it.
- `/tmp` is a tmpfs since Debian 13, so uploaded files do not stay in the image.
- The shell provisioner uploads each script and runs it with `environment_vars` in front. If one exits non-zero,
  the build stops and, by default, deletes the VM. To inspect a failed VM, run `packer build -on-error=abort`
  locally.
- **`direction = "download"`** goes the other way and copies a file from the VM to the build machine. That is how
  the rootfs image for RAUC comes out.

## 10-packages.sh

```sh
#!/bin/sh
set -eu
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
    rauc rauc-service gdisk efibootmgr grub-efi-amd64-bin systemd-resolved
```

- Debian splits RAUC into two packages. `rauc` is the command-line tool; `rauc-service` holds the D-Bus-activated
  `rauc.service`. Debian's `rauc` command works by sending requests to that service, so `rauc install` and
  `rauc status` need both.
- `gdisk` provides `sgdisk` for naming partitions, and `grub-efi-amd64-bin` the modules to build the ESP's GRUB.
  `systemd-resolved` is for the network change in the cleanup step. All package installs happen here so that step
  does not need the network again.

## 20-ab-layout.sh: from a normal install to A/B

What the installer produced is a plain Debian. This script turns it into the layout from the
[GRUB A/B post](/posts/unix/boot/grub-ab/). The reasons behind each part are in that post; here we only look at
what changes when building an image.

```sh
F=/tmp/files   # where the file provisioner uploaded them

# 1. Name the partitions. Everything below refers to them by name, never by UUID.
disk=/dev/$(lsblk -no PKNAME "$(findmnt -no SOURCE /)")
sgdisk -c 1:esp -c 2:rootfs_A -c 3:rootfs_B -c 4:data "$disk"

cat > /etc/fstab <<'EOT'
/dev/root       /          ext4  errors=remount-ro  0 1
PARTLABEL=esp   /boot/efi  vfat  umask=0077         0 1
PARTLABEL=data  /data      ext4  defaults           0 2
EOT
```

**Why fstab is rewritten** is the most important point of this series. The installer writes filesystem UUIDs into
fstab. A UUID is generated on every `mkfs`, so it **differs from build to build.** Put a 1.1 rootfs onto a device
made from the 1.0 OVA with RAUC, and the 1.1 fstab names the ESP UUID of the 1.1 build VM: `/boot/efi` does not
mount, RAUC cannot touch grubenv, and mark-good fails.

- The ESP and data are mounted by **GPT partition name**. The partition table is written once at the factory and
  never touched by updates, so it looks the same from any build's rootfs.
- Root is just `/dev/root`. The real root comes from `root=PARTLABEL=rootfs_$SLOT` on the kernel command line;
  this fstab line only supplies mount options (`errors=remount-ro`) and the rw remount. Yocto's default fstab does
  the same.

```sh
# 2. Remove the distribution's EFI boot path. The slot keeps only grub-mkconfig/update-grub.
for n in $(efibootmgr | sed -n 's/^Boot\([0-9A-F]\{4\}\)\*\{0,1\} debian.*/\1/p'); do
    efibootmgr -q -b "$n" -B
done
apt-mark manual grub-common grub2-common grub-efi-amd64-bin
apt-get purge -y --allow-remove-essential grub-efi-amd64 grub-efi-amd64-signed shim-signed
apt-get autoremove -y --purge
rm -rf /boot/efi/EFI/debian /boot/efi/EFI/BOOT
```

- The installer creates a `debian` boot entry in NVRAM and puts it first. Left alone, the firmware skips our
  selector and boots `\EFI\debian\shimx64.efi`. Removing the entry and the directory makes the firmware fall back
  to the default path `\EFI\BOOT\BOOTX64.EFI`.
- `grub-efi-amd64`, `-signed` and `shim-signed` run `grub-install` on install and upgrade. Inside a slot, a rootfs
  received as an update could overwrite the ESP, so they go. `grub-efi-amd64-bin`, which only holds the modules,
  stays: `grub-mkimage` builds the ESP's GRUB from it.
- The directories are removed **after** the purge. While purging, the maintainer scripts of `shim-signed` and
  `grub-efi-amd64` run `grub-install` again ("Installing for x86_64-efi platform" in the log) and write to the ESP
  once more.
- Without the `apt-mark manual` first, `autoremove` would also remove `grub-common`, which was only installed as a
  dependency.
- **`--allow-remove-essential` is needed.** Debian marks `grub-efi-amd64-signed` as `Protected: yes`, so apt treats
  it as essential and `-y` alone will not remove it. It guards against removing the boot loader by accident;
  replacing that boot loader is exactly the point here, so permission is given explicitly.

  ```text
  WARNING: The following essential packages will be removed.
  This should NOT be done unless you know exactly what you are doing!
    grub-efi-amd64-signed
  E: Essential packages were removed and -y was used without --allow-remove-essential.
  ```

```sh
# 3. Slot side: update-grub writes one entry that boots root=PARTLABEL=rootfs_$SLOT.
for s in 00_header 10_linux 20_linux_xen 25_bli 30_os-prober 30_uefi-firmware; do
    [ -e /etc/grub.d/$s ] && chmod -x /etc/grub.d/$s
done
install -m 0755 $F/10_ab /etc/grub.d/10_ab
echo 'GRUB_CMDLINE_LINUX="console=tty0 console=ttyS0,115200"' > /etc/default/grub
update-grub

# 4. ESP side: our own GRUB, the selector and grubenv.
install -d /boot/efi/EFI/BOOT
grub-mkimage -O x86_64-efi -p /EFI/BOOT -o /boot/efi/EFI/BOOT/BOOTX64.EFI \
    part_gpt fat ext2 normal configfile loadenv test echo linux gzio sleep halt
install -m 0644 $F/esp-grub.cfg /boot/efi/EFI/BOOT/grub.cfg
grub-editenv /boot/efi/EFI/BOOT/grubenv create
grub-editenv /boot/efi/EFI/BOOT/grubenv set ORDER="A B" A_OK=1 A_TRY=0 B_OK=0 B_TRY=0
```

This matches the GRUB A/B post. The one difference in an image build is that **`/etc/default/grub` is written from
scratch**. That file belonged to the `grub-efi-amd64` package, so the purge above removed it. The first version
edited one line with `sed` and the build stopped on "can't read /etc/default/grub". `10_ab` only reads
`GRUB_CMDLINE_LINUX`, so the file is just that line.

```sh
# 5. RAUC: slots, trusted CA, mark-good after a successful boot.
install -d /etc/rauc /data/rauc
sed "s/@COMPATIBLE@/$COMPATIBLE/" $F/system.conf > /etc/rauc/system.conf
install -m 0644 $F/ca.cert.pem /etc/rauc/ca.cert.pem
install -m 0644 $F/rauc-mark-good.service /etc/systemd/system/rauc-mark-good.service
systemctl enable rauc-mark-good.service

printf 'IMAGE_VERSION=%s\n' "$IMAGE_VERSION" > /etc/os-image-release
```

The `system.conf`:

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

- With `bootloader=grub`, RAUC edits the GRUB A/B post's `ORDER`, `X_OK` and `X_TRY` variables in `grubenv`
  itself.
- `bootname` is matched against `rauc.slot=A` on the kernel command line. That is how RAUC knows which slot it
  booted from, and it writes updates to the other one.
- `data-directory` lives on the data partition. Slot status and install history kept on the root would vanish on
  every slot switch.
- **`resize=true`**: the bundle's ext4 image is shrunk to its minimum size (below). After writing it to a slot,
  RAUC grows it with `resize2fs` to the partition size (3 GB). Without it, the new slot boots with an almost full
  root.
- RAUC's source has a `rauc-mark-good.service`, but Debian's packages do not ship it, so it is provided here.

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

  `boot-complete.target` is where systemd expresses "the boot succeeded". Hook health check units in with
  `Before=boot-complete.target` and `RequiredBy=boot-complete.target`, and a failing check means the target is
  never reached and mark-good never runs. grubenv keeps `X_TRY=1` and the next boot goes back to the previous slot.
  That is where the health checks from the [RAUC post](/posts/architecture/os/ota-update/rauc/) belong.
- `/etc/os-image-release` shows which build the booted slot holds.

## 90-cleanup.sh: removing what belongs to the build VM

A freshly installed VM contains several **values that are only right for that VM**. Hundreds of devices made from
the OVA, and every rootfs shipped later through RAUC, inherit them.

| Left behind | Problem | Fix |
|---|---|---|
| NIC name in the ifupdown config (`enp0s3`) | Different name on another hypervisor or real hardware, no network | systemd-networkd, DHCP on any `en*`, `eth*` |
| apt proxy (`/etc/apt/apt.conf`) | Devices look for the CI runner's apt-cacher-ng | Delete the line |
| `/etc/machine-id` | Every device has the same ID; DHCP client IDs, journald and others collide | Leave it empty, generated on first boot |
| SSH host keys | Every device has the same keys; one leak lets anyone impersonate all of them | Delete, `ssh-keygen -A` on first boot |
| root password, `PermitRootLogin yes` | Anyone can log in with the build password | Lock it, public keys only |
| install logs, shell history | Leaks build environment details, takes space | Empty them |

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

- `machine-id` is left as an **empty file** rather than deleted. When systemd finds an empty `/etc/machine-id`, it
  treats the boot as a first boot and writes a new ID into it.
- The unit that generates SSH host keys only runs when they are missing
  (`ConditionPathExists=!/etc/ssh/ssh_host_ed25519_key`).
- **The build carries on even though the script locks the root password and removes the sshd setting.** Packer
  runs the remaining steps over the SSH connection it already has, and the sshd change only applies to new
  connections.

:::warning They are regenerated on updates too
The rootfs image also carries an empty `machine-id` and an `/etc/ssh` without host keys. So when RAUC updates the
other slot, **the machine-id and SSH host keys are regenerated** on that slot's first boot. In part 3's test, slot
A's machine-id was `f5c02aef…` and slot B's after the update was `fc311b22…`. If a device identity is tied to the
machine-id, or SSH clients pin the host key, both change on every update. Keeping them means storing them on
`/data`, which updates do not touch, and bringing them in at boot. The image in this series does not go that far.
:::

## 95-export-rootfs.sh: getting the image out for RAUC

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

- Mounting the root a second time with a **bind mount** leaves out everything stacked on top of it (`/proc`,
  `/sys`, `/dev`, `/run`, `/tmp`, `/boot/efi`, `/data`) and shows only **the root filesystem itself**. The mount
  point directories stay, empty, so they are there to mount on in the new slot.
- `mkfs.ext4 -d` creates the filesystem and fills it with a directory's contents. Owners, permissions and extended
  attributes (file capabilities included) are copied as root inside the VM, so they come out right. Building it on
  the build machine from a tarball needs root there, and capabilities get lost easily.
- The output goes to `/data`. `/tmp` (tmpfs) is too small, and writing to the root would make the image try to
  contain itself.
- `resize2fs -M` shrinks it to the minimum. Once written to a slot, RAUC's `resize=true` grows it again.

The `file` provisioner then downloads it, and the last inline step deletes it and runs `fstrim` to hand the empty
blocks back, so no trace of a 1 GB image file is left in the OVA.

## Results

There is no VirtualBox in this environment, so the real template was checked with `packer init`,
`packer fmt -check` and `packer validate`, and the build ran with the QEMU builder. Only the `source` changed, to
start from the disk installed in part 1; the `build` block is the one from this post. The VirtualBox-only parts,
`vboxmanage` and the OVA export, were not exercised.

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

- Provisioning takes about 6 minutes. After `resize2fs -M`, `rootfs.ext4` is 1.0 GB (264975 blocks).
- The download and `fstrim` ran over the same SSH connection **after** the cleanup script had locked the root
  password.
- It did not pass on the first try. `Protected: yes` on `grub-efi-amd64-signed`, the `/etc/default/grub` removed by
  the purge, and the `rauc-mark-good.service` missing from Debian each stopped the build once; the scripts above
  are the fixed versions.

The resulting disk was booted with an **empty NVRAM**, the same situation as importing the OVA into a new VM. The
firmware finds `\EFI\BOOT\BOOTX64.EFI` and the selector picks slot A.

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

- The GPT names are there, and the ESP holds only the three files the selector needs.
- Whatever the NIC was called during the build, networkd picked it up with DHCP. A new machine-id and new SSH host
  keys were generated on first boot, and root is locked (`L`), so only keys get in.
- `rauc-mark-good.service` ran after `boot-complete.target` and set grubenv's `A_TRY` back to 0. Slot B is still
  empty (`B_OK=0`) and shows as bad. `systemctl --failed` was empty.

The next post runs this build in GitLab CI, signs `rootfs.ext4` into a RAUC bundle and releases it alongside the
OVA.
