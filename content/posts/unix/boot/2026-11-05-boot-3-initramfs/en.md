+++
title = "Boot process 3: the initramfs — how the kernel gets to the real root"
description = "What the initramfs does between the kernel and the real root filesystem: the cpio format and how the kernel unpacks it, when you can boot without one and when you can't, a hand-made busybox /init with switch_root, the structure of Ubuntu's initramfs-tools and break= debugging, and customising it with hooks and scripts."
tags = ["linux", "initramfs", "boot", "embedded", "kernel"]
+++

In [part 1](/posts/unix/boot/boot-1-firmware/) and [part 2](/posts/unix/boot/boot-2-u-boot/), the bootloader
loaded an initramfs into memory along with the kernel and told the kernel where it was. Once the kernel has
initialised the hardware, it has to mount the root filesystem and run `/sbin/init`. But if the driver for the
root disk is built as a module, or the root sits on LVM, RAID or LUKS, or on the network (NFS), the kernel can't
do that alone. The initramfs fills the gap: **a small root filesystem in memory**.

The experiments ran on x86 QEMU with Ubuntu's 6.8 kernel, using the disk image (ESP, `rootfs_A`, `rootfs_B`) built
in the [GRUB A/B post](/posts/unix/boot/grub-ab/).

## How the kernel handles an initramfs

An initramfs file is a **cpio archive** (newc format), usually compressed with gzip or zstd. An uncompressed
archive starts with newc's six-byte magic number:

```text
$ head -c 6 initrd.img-6.8.0-147-generic
070701
```

Early in boot the kernel unpacks the archive into **rootfs**, a tmpfs (or ramfs) instance, and runs its `/init`
as PID 1. `rdinit=` on the command line names a different program. A few details:

- Several archives can be **concatenated**. The kernel unpacks them in order, later files overwriting earlier
  ones at the same path. Compressed and uncompressed archives can be mixed.
- The kernel image carries an initramfs of its own (`CONFIG_INITRAMFS_SOURCE`). By default it is a minimal
  archive with just `/dev`, `/dev/console` and `/root`, and the one from the bootloader is unpacked on top of it.
  Embedded systems sometimes put their whole initramfs there and ship a single kernel file.
- If there is **no** `/init` after unpacking, the kernel falls back to the old way: it mounts the `root=` device
  itself and runs `/sbin/init` (or `init=`).

The older mechanism was **initrd**, a block-device image (a RAM disk). The file name `initrd.img` and the
bootloader command `initrd` are leftovers from that era; nearly everything uses cpio now.

## When you can boot without one

With the drivers built into the kernel, you can boot without an initramfs. Ubuntu's x86 kernel has virtio block
and ext4 built in (`=y`), so this disk works for a test. Booting with no initramfs and `root=PARTLABEL=rootfs_A`,
the kernel mounts the root itself:

```text
[    2.455157] EXT4-fs (vda2): mounted filesystem cb87ce6f-... ro with ordered data mode.
[    2.455654] VFS: Mounted root (ext4 filesystem) readonly on device 253:2.
[    2.742791] Run /sbin/init as init process
rootfs: pid 1, running release 1.0 from /dev/root
```

Naming the same partition by filesystem label (`root=LABEL=rootA`) panics:

```text
[    2.394034] Disabling rootwait; root= is invalid.
[    2.401243] VFS: Cannot open root device "LABEL=rootA" or unknown-block(0,0): error -6
[    2.401369] Please append a correct "root=" boot option; here are the available partitions:
[    2.401879]   fd01           32768 vda1 001b70e0-a665-42fc-af81-cecafcc05824
[    2.401987]   fd02           65536 vda2 a8d14487-02a0-491b-9ae9-d75f471a987c
[    2.402051]   fd03           65536 vda3 402dbd4e-e479-4cc9-bc1c-079d1ece0630
[    2.402506] Kernel panic - not syncing: VFS: Unable to mount root fs on unknown-block(0,0)
```

The difference is where the information lives. The only `root=` forms the kernel understands itself are device
names such as `/dev/vda2`, plus `PARTUUID=` and `PARTLABEL=` from the partition table. `LABEL=` and `UUID=` are
stored **inside the filesystem**, so each device has to be opened and its superblock read. That is the job of
`blkid` and udev in user space, hence the initramfs. The value next to each partition in the panic message is
its PARTUUID.

The typical reasons you need an initramfs:

- the driver for the root device or filesystem is a module (true of most distribution kernels);
- the root is named by `UUID=` or `LABEL=`;
- the root is on LVM, software RAID, LUKS encryption or NFS;
- the root needs fsck before mounting, or is assembled from layers as in the
  [OverlayFS post](/posts/architecture/os/ota-update/overlayfs/).

## Building one by hand

A single statically linked busybox is enough for a working initramfs. This is the one used in the GRUB post's
experiments:

```bash
mkdir -p initramfs/{bin,dev,proc,sys,newroot}
cp /usr/bin/busybox initramfs/bin/        # the static binary from the busybox-static package
install -m755 init initramfs/init
(cd initramfs && find . | cpio -o -H newc | gzip -9) > initrd.img
```

And its `/init`:

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

1. Create busybox's applet links and mount `/dev`, `/proc` and `/sys`. `/dev` is devtmpfs, where the kernel
   creates device nodes itself.
2. Read `root=` from the kernel command line. The kernel leaves arguments it doesn't know in `/proc/cmdline`
   too, so values such as `rauc.slot=` are read here as well.
3. Find the root device. Each partition's `/sys/class/block/*/uevent` has a `PARTNAME=` (the GPT partition
   name), so no udev is needed.
4. Mount the root on `/newroot`, move `/dev` under it, and **`switch_root`**.

```text
initramfs: pid 1, cmdline: BOOT_IMAGE=/boot/vmlinuz-6.8.0-147-generic root=PARTLABEL=rootfs_A ro rauc.slot=A console=ttyS0
initramfs: root PARTLABEL=rootfs_A is /dev/vda2
rootfs: pid 1, running release 1.0 from /dev/vda2
```

`switch_root` deletes every file in the initramfs to give the memory back, moves the new root onto `/`
(`mount --move` then `chroot`), and `exec`s the new root's init. Because it is an `exec`, the PID stays 1: in
the last line the real root's `/sbin/init` reports PID 1 as well. `pivot_root`, used elsewhere, doesn't apply
because rootfs is a special mount that can't be unmounted or moved; instead it is emptied and the new root is
mounted over it. That is also why `switch_root` only works as PID 1.

## A distribution's initramfs: initramfs-tools

Distributions don't build initramfs images by hand; they use a generator. Debian and Ubuntu use
**initramfs-tools**, Fedora and RHEL **dracut**, Arch Linux **mkinitcpio**. On Ubuntu, installing a kernel
package runs `update-initramfs`, which creates `/boot/initrd.img-<version>` with `mkinitramfs` doing the work.

Here is an initramfs for the 6.8 kernel built with Ubuntu 24.04's `mkinitramfs`:

```text
$ lsinitramfs initrd.img-6.8.0-147-generic | wc -l
523
$ lsinitramfs initrd.img-6.8.0-147-generic | grep -c '\.ko'
148
$ unmkinitramfs initrd.img-6.8.0-147-generic x && du -sh x/*
12M	x/early
27M	x/main
```

The file holds two archives. The first, `early`, is an uncompressed cpio, and what it holds is kernel modules
that are already zstd-compressed (`*.ko.zst`); keeping them apart avoids compressing compressed files again. It
is followed by `main`, everything else, compressed with zstd. Intel and AMD microcode updates go into such a
leading uncompressed cpio too (`kernel/x86/microcode/…`), because the kernel picks them up from there very early,
before it decompresses anything.

`main` contains busybox, klibc utilities, `systemd-udevd`, the modules, and an `/init` written as a shell script.
At boot, that `/init` prints a message per stage:

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

The skeleton of `/init` (the essentials of its 388 lines):

```sh
export init=/sbin/init
export rootmnt=/root
. /scripts/functions
# … parse the kernel command line (root=, init=, break=, debug, …)
maybe_break top
run_scripts /scripts/init-top          # start udev, etc.
maybe_break modules
# … load the modules listed in /etc/initramfs-tools/modules
maybe_break premount
run_scripts /scripts/init-premount
maybe_break mount
. /scripts/local                       # local-top → wait for root device → local-premount → fsck → mount → local-bottom
. /scripts/nfs
maybe_break mountroot
mountroot
maybe_break bottom
run_scripts /scripts/init-bottom       # stop udev, move /dev to ${rootmnt}/dev
mount -n -o move /run ${rootmnt}/run
maybe_break init
mount -n -o move /sys ${rootmnt}/sys
mount -n -o move /proc ${rootmnt}/proc
exec run-init ${drop_caps} "${rootmnt}" "${init}" "$@" <"${rootmnt}/dev/console" >"${rootmnt}/dev/console" 2>&1
```

`run-init` at the end is klibc's equivalent of `switch_root`. It does the same job as the hand-made initramfs,
with udev, module loading and per-stage script directories added in between. Since `/run`, `/sys`, `/proc` and
`/dev` are moved onto the real root, the real root needs those mount points; the `mounting /run on /root/run
failed` warning in the log comes from the busybox test root, which has no `/run` directory.

`root=PARTLABEL=` is handled by `resolve_device` in `scripts/functions`, together with `LABEL=`, `UUID=` and
`PARTUUID=`, using links udev creates such as `/dev/disk/by-partlabel/`.

## Debugging: break=

When the initramfs can't find the root, it usually drops to a shell with an `(initramfs)` prompt. To stop on
purpose, add `break=<stage>` to the kernel command line; the stage names are the `maybe_break` arguments in the
skeleton above (`top`, `modules`, `premount`, `mount`, `mountroot`, `bottom`, `init`). Booting with `break=premount`:

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

- The shell's PID is 139, not 1. PID 1 is still `/init`, and the shell runs as its child; `exit` lets `/init`
  carry on from where it stopped.
- `/` is `rootfs`, the filesystem the kernel unpacked the initramfs into.
- udev has already run, so the `/dev/disk/by-partlabel/` links exist. For a root that can't be found, start here
  with `blkid`, `ls /dev` and `cat /proc/partitions` to see whether the kernel sees the device at all.

Other useful arguments:

| Argument | Effect |
|---|---|
| `debug` | initramfs-tools: trace `/init` like `sh -x` into `/run/initramfs/initramfs.debug` |
| `rdinit=/bin/sh` | a shell as PID 1 instead of `/init`; no scripts run at all |
| `panic=10` | reboot 10 seconds after a kernel panic, so remote devices don't sit stuck |
| `rd.break`, `rd.shell`, `rd.debug` | the same roles in dracut (stages named like `rd.break=pre-mount`) |

## Customising: hooks and scripts

initramfs-tools has two kinds of extension points:

- **hooks** (`/etc/initramfs-tools/hooks/`) run when `mkinitramfs` **builds** the image and decide what goes in;
- **scripts** (`/etc/initramfs-tools/scripts/<stage>/`) go into the initramfs and run at that stage **during boot**.

As an example, a hook that adds `blkid`, and a script that prints slot information right after the root is mounted:

```sh
#!/bin/sh
# /etc/initramfs-tools/hooks/slotinfo -- runs at mkinitramfs time
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /usr/share/initramfs-tools/hook-functions
copy_exec /usr/sbin/blkid /sbin
```

```sh
#!/bin/sh
# /etc/initramfs-tools/scripts/local-bottom/slotinfo -- runs during boot, after the root is mounted on ${rootmnt}
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /scripts/functions

for x in $(cat /proc/cmdline); do
  case "$x" in rauc.slot=*) slot=${x#rauc.slot=} ;; esac
done
log_success_msg "slot ${slot:-?}, root ${ROOT} = $(blkid -o value -s PARTLABEL "$(resolve_device "$ROOT")")"
```

- The `PREREQ`/`prereqs` lines are fixed boilerplate; initramfs-tools uses them to order scripts.
- `copy_exec` copies a binary together with the shared libraries it links against; the image ended up with both
  `usr/sbin/blkid` and `libblkid.so.1`. Modules are added with `manual_add_modules`.
- Both files must be executable, and changes only take effect after rebuilding with `update-initramfs -u`.

Booting with `rauc.slot=A` shows the message at the `local-bottom` stage:

```text
Begin: Running /scripts/local-premount ... done.
Warning: fsck not present, so skipping root file system
done.
Begin: Running /scripts/local-bottom ... Success: slot A, root /dev/vda2 = rootfs_A
done.
Begin: Running /scripts/init-bottom ... done.
```

It also shows `ROOT` already resolved to `/dev/vda2` at `local-bottom`. The same spot is where you would turn the
root into an overlay, pick a per-slot data partition, or record a failed boot.
