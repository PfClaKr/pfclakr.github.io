+++
title = "OS image build 1: unattended Debian installs with preseed"
description = "How the Debian installer (d-i) asks its questions and how preseed answers them ahead of time: the boot command line, each part of preseed.cfg, a partman recipe for an A/B layout, late_command, and where to look when the install stops. Tested by installing in QEMU."
tags = ["linux", "debian", "preseed", "packer", "ota"]
draft = true
date = 2026-11-05T13:00:00
+++

The [A/B partitions](/posts/architecture/os/ota-update/ab-partitions/), [GRUB A/B](/posts/unix/boot/grub-ab/)
and [RAUC](/posts/architecture/os/ota-update/rauc/) posts covered a device that is already installed and
receives updates. This series covers the step before that: **building the image** that goes on the device.

1. **preseed** (this post): installing Debian netinst without a person at the keyboard
2. **Packer**: a template that runs the install in VirtualBox and exports an OVA plus a rootfs image for RAUC
3. **GitLab CI**: a pipeline that builds the image on every commit, signs a RAUC bundle and publishes a release

There are two outputs. The **OVA** is the factory image for new VMs or devices. The **RAUC bundle** is the
update for devices already in the field. Both come out of the same build.

## How d-i asks questions

Every screen of the Debian installer (debian-installer, d-i) is a **debconf question**. Language, keyboard,
hostname, mirror, partitioning, the root password: each one has a key of the form `owner name type value`.

```text
d-i netcfg/get_hostname string debian
d-i partman/confirm boolean true
tasksel tasksel/first multiselect ssh-server
```

**Preseed** is a file with those answers written in advance. Before d-i shows a question it looks in the
debconf database, and if an answer is already there it marks the question as seen and moves on. The first
column is the owner: `d-i` for the installer's own questions, the package name (`tasksel`,
`popularity-contest`) for questions asked by packages being installed.

Every question also has a **priority** (low, medium, high, critical). With the boot parameter
`priority=critical`, anything below critical is not asked and takes its default. If a critical question has
no answer in the preseed, the install stops on it and waits. That is the cause of almost every stuck
unattended install.

## Three ways to hand over a preseed

| Method | Used for |
|---|---|
| **initrd**: `preseed.cfg` at the root of the initrd | rebuilding the installer image yourself |
| **file**: a file on the ISO, `preseed/file=/cdrom/preseed.cfg` | custom ISOs |
| **network**: `preseed/url=http://…/preseed.cfg` | PXE, Packer |

Packer starts a small HTTP server for the duration of the build and puts its address on the boot command
line. The official netinst ISO can be used as is, without modifying it, which is why this is the most common
approach.

## The boot command line

Booting the netinst ISO with UEFI shows a GRUB menu. Instead of picking an entry, press `c` to get the GRUB
command line and boot the kernel directly. In the next post Packer types these keys for us.

```text
linux /install.amd/vmlinuz auto=true priority=critical preseed/url=http://10.0.2.2:8080/preseed.cfg hostname=debian domain=
initrd /install.amd/initrd.gz
boot
```

- `auto=true` **changes the order of the questions.** Normally d-i asks for language and keyboard first and
  configures the network afterwards. But the preseed file can only be fetched once the network is up.
  `auto=true` postpones the language and keyboard questions until after the preseed has been loaded, so those
  answers can come from the file too.
- `preseed/url`, `priority`, `hostname` and `domain` are debconf keys given directly as boot parameters.
  `hostname` is a short alias for `netcfg/get_hostname`. Every `key=value` on the kernel command line goes into
  debconf, so any key from the preseed file can be put here.
- **Why the hostname goes on the command line**: network configuration (netcfg) runs *before* the preseed file
  is fetched, and that is when it asks for the hostname. Writing it only in the file is too late, the question
  has already been asked. So it goes in both places.

## preseed.cfg

The whole file is about 70 lines. In the next post Packer fills in the `${…}` placeholders with
`templatefile()` before serving it.

### Locale, network, mirror

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

- `choose_interface select auto` picks the first interface with a link. No need to hard-code the VirtualBox
  NIC name.
- `netcfg/hostname` forces this name even when DHCP hands out a hostname. `get_hostname` is only the
  question's default and loses to the DHCP value.
- `load_firmware false` suppresses the question about loading missing firmware from removable media.
- **The mirror stays fixed; only the proxy changes.** In CI the runner has an apt-cacher-ng and `apt_proxy` is
  set to `http://10.0.2.2:3142`, so each build does not download the same packages again. Locally it is an
  empty string. Pointing the mirror itself at the cache would leave that address in the installed system's
  `sources.list`. The proxy is a single line in `/etc/apt/apt.conf`, easy to remove in the cleanup step.
- `cdrom/set-first false` and `disable-cdrom-entries true` keep the ISO out of the installed system's APT
  sources.

### Accounts and clock

```text
d-i passwd/root-login boolean true
d-i passwd/make-user boolean false
d-i passwd/root-password password ${root_password}
d-i passwd/root-password-again password ${root_password}

d-i clock-setup/utc boolean true
d-i time/zone string Etc/UTC
d-i clock-setup/ntp boolean true
```

No regular user, just root. The password is **only used by Packer to log in over SSH during the build**.
The final cleanup step locks it, and the finished image only accepts SSH keys.

:::note
`passwd/root-password-crypted` takes a hash instead. Plain text is used here because the preseed is only
served over local HTTP during the build and the password is locked once the build ends. The value comes from a
masked CI variable.
:::

### Partitioning: the A/B layout

This is where the install differs most from a normal one. It builds the layout from the
[GRUB A/B post](/posts/unix/boot/grub-ab/): an ESP, slot A, slot B, and a data partition that survives updates.

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

An `expert_recipe` is `name ::` followed by partitions separated by `.`. The first line of each partition is
`minimum priority maximum filesystem`, in MB.

- **Minimum, priority, maximum**: partman gives every partition its minimum, then shares the remaining space in
  proportion to priority, up to each maximum. Slots A and B have 3072 for all three, so they are exactly 3 GB.
  The data partition has a maximum of `-1` (no limit) and takes the rest.
- `$iflabel{ gpt } $reusemethod{ } method{ efi }`: on a GPT disk, create an ESP, or reuse one that already
  exists. Words starting with `$` are conditions or flags; `name{ value }` sets something on that partition.
- **Slot B is `method{ keep }`**: the partition is created but neither formatted nor mounted. B is empty in the
  factory image and RAUC fills it on the first update. A partition without a filesystem makes d-i ask "No file
  system is specified for partition #3. Go back to the menu?". `method_only false` is the "don't go back"
  answer.
- No swap. That makes d-i ask whether to continue without swap, answered with `no_swap false`.
- There are four confirmations: writing a new partition table (`confirm_write_new_label`), selecting "Finish"
  on the partitioning screen (`choose_partition`), writing to disk (`confirm`), and the confirmation when no
  partition is being formatted (`confirm_nooverwrite`). Missing any of them stops the install on that screen.

:::warning Partition names and order
partman recipes have no entry for GPT partition names (like `rootfs_A`). Recipes create partitions 1, 2, 3, 4
in the order they are written, so the next post's provisioning step names them by number with `sgdisk -c`.
The ESP selector uses `(hd0,gpt2)` and `(hd0,gpt3)`, so the order must not change.
:::

### Packages and boot loader

```text
tasksel tasksel/first multiselect ssh-server
d-i pkgsel/include string ca-certificates
d-i pkgsel/upgrade select full-upgrade
popularity-contest popularity-contest/participate boolean false

d-i grub-installer/only_debian boolean true
d-i grub-installer/bootdev string default
d-i grub-installer/force-efi-extra-removable boolean true
```

- Only `ssh-server` in tasksel. Adding `standard` makes the image hundreds of MB bigger. Everything else is
  installed in the Packer step, so the installer is only responsible for "a minimal system that boots and
  accepts SSH".
- `pkgsel/upgrade full-upgrade` applies security updates released after the netinst ISO during the install.
- `force-efi-extra-removable` installs GRUB at the removable-media path `\EFI\BOOT\BOOTX64.EFI` as well as
  `\EFI\debian\`. The system still boots if the firmware's NVRAM boot entry disappears. That matters because
  nothing guarantees the VM's NVRAM travels with an OVA export. The next post replaces this GRUB with the A/B
  selector, but the first boot after the install uses it.

### late_command

```text
d-i preseed/late_command string \
    in-target sh -c 'echo "PermitRootLogin yes" > /etc/ssh/sshd_config.d/90-packer.conf'

d-i finish-install/reboot_in_progress note
```

`late_command` runs in the installer environment at the end of the install, just before the reboot. The
installed system is mounted at `/target`, and `in-target` runs the command inside it in a chroot.

Debian's sshd defaults to `PermitRootLogin prohibit-password`, so root cannot log in over SSH with a
password. Only that setting changes here so Packer can get in, and the file is removed again in the cleanup
step. Everything else happens on the Packer side: `late_command` is a single shell line, which becomes hard
to read and debug as it grows.

`reboot_in_progress note` skips the final "Installation complete" screen and reboots right away.

## Finding key names

Preseed keys are not something to memorize. There are three places to look.

- **The example file**: the appendix of the Debian installation guide has an `example-preseed.txt` for each
  release. Almost every option is there with comments.
- **A system installed by hand once**: `debconf-get-selections --installer` from the `debconf-utils` package
  prints every answer given during the install in preseed format. Choices made on screen can be copied over
  directly.
- **`/var/log/installer/cdebconf/questions.dat`**: the question database left on the installed system.

## When the install stops

A stuck unattended install shows a single question on screen. Add that question's key to the preseed.

- During the install, **Alt+F4** shows the live log and **Alt+F2** opens a shell. From the shell,
  `grep -A3 name /var/lib/cdebconf/questions.dat` shows the key.
- After the install, the full log is in `/var/log/installer/syslog`.
- Commonly missed: the four partman confirmations above, the no-swap warning, and questions asked before the
  network is up (language, keyboard, hostname). The last group is handled by `auto=true` and command-line
  parameters.

This happened while writing this post. The first version had no `method_only`, and about 30 minutes in,
the install was sitting on this screen at the partitioning step.

```text
[!!] Partition disks
No file system is specified for partition #3 of SCSI1 (0,0,0) (sda).
If you do not go back to the partitioning menu and assign a file system to this partition,
it won't be used at all.
Go back to the menu?                                    <Go Back>   <Yes>   <No>
```

Question texts live in each udeb's debconf `templates` file. Searching the `partman-*` udebs the installer
downloaded for that sentence gives the key.

```bash
ar x partman-basicmethods_80_all.udeb control.tar.xz
tar xJf control.tar.xz ./templates
grep -B3 "won't be used at all" templates
# Template: partman-basicmethods/method_only
# Type: boolean
# Description: Go back to the menu?
#  No file system is specified for partition #${PARTITION} of ${DEVICE}.
```

It is a `boolean` and the question is "Go back to the menu?", so the answer is `false`.

:::info Ubuntu is different
Ubuntu is Debian-based too, but its installer switched to subiquity (Server from 20.04, desktop from 23.04),
and unattended installs use **autoinstall** (a YAML file in cloud-init format). Preseed does not work on
current Ubuntu.
:::

## Running it in QEMU

@@RESULTS@@

The next post puts this preseed into a Packer template, turns the installed system into the A/B layout, and
exports an OVA and a rootfs image.
