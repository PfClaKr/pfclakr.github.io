+++
title = "Boot process 2: U-Boot — environment, bootcmd, FIT and an A/B script"
description = "What U-Boot, the embedded Linux bootloader, actually does, run on QEMU arm64: the environment and fw_setenv on the Linux side, standard boot and extlinux.conf, booting by hand with booti, FIT images verified by hashes, and A/B slot selection in a boot script."
tags = ["linux", "u-boot", "boot", "embedded", "arm64"]
date = 2026-11-05T11:00:00
+++

[Part 1](/posts/unix/boot/boot-1-firmware/) ended with an ARM SoC's boot ROM loading a small first-stage
loader (U-Boot SPL or similar) into SRAM, and that loader initialising DRAM and loading the next stage. That
next stage is most often U-Boot. This post covers what U-Boot does until it has found the kernel, loaded it
and jumped to it.

The test setup is QEMU's `virt` machine (Cortex-A57), U-Boot 2025.10 from Ubuntu 24.04's `u-boot-qemu`
package, and Ubuntu's arm64 6.8 kernel. The initramfs is a minimal busybox one that prints what it received
from the bootloader and powers off.

## Reading the start-up banner

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

- `DRAM`: usable memory. On real boards this is what SPL initialised.
- `devicetree: board`: U-Boot itself learns the hardware from a device tree. On QEMU it uses the DTB QEMU
  generates.
- `Loading Environment from Flash... bad CRC`: no saved environment (or a corrupt one), so U-Boot falls back
  to the defaults compiled into it. A common warning on a board's first boot.
- `Hit any key to stop autoboot`: press a key within `bootdelay` seconds to get the `=>` prompt; otherwise
  U-Boot runs the contents of the `bootcmd` variable.

## The environment

Most of U-Boot's behaviour is set through environment variables. A few, from the prompt:

```text
=> printenv bootcmd boot_targets kernel_addr_r ramdisk_addr_r bootdelay
bootcmd=bootflow scan -lb
boot_targets=qfw usb scsi virtio nvme dhcp
kernel_addr_r=0x40400000
ramdisk_addr_r=0x44000000
bootdelay=2
```

| Variable | Meaning |
|---|---|
| `bootcmd` | command run on autoboot |
| `bootargs` | the kernel command line; it goes into the device tree's `/chosen/bootargs` at boot |
| `bootdelay` | seconds to wait before autoboot; `-1` disables autoboot |
| `kernel_addr_r`, `ramdisk_addr_r`, `fdt_addr_r` | conventional names for where to load the kernel, initramfs and DTB |
| `boot_targets` | the order in which standard boot looks at boot devices |

`setenv` changes a variable and `saveenv` stores the environment. Where it goes depends on the build: SPI
flash, an offset on eMMC, a `uboot.env` file on a FAT partition, a UBI volume… `env default -a` goes back to
the defaults.

### Storage format and Linux-side tools

The stored environment has a simple format: a 4-byte CRC32, then `name=value` strings separated by NUL bytes,
then padding up to a fixed size:

```text
000000   +   K 305   S   a   r   c   h   =   a   r   m  \0   b   a   u
000010   d   r   a   t   e   =   1   1   5   2   0   0  \0   b   o   a
```

This QEMU build uses the first 256 KiB (`0x40000`) of the second flash bank. The last line of `printenv`,
`Environment size: 514/262140 bytes`, shows 256 KiB minus the 4-byte CRC. Some configurations use a
**redundant environment**: two copies written alternately, with an extra flag byte, so one intact copy
survives a power cut in the middle of a write.

On Linux, `fw_printenv` and `fw_setenv` from `libubootenv` (or U-Boot's own `tools/env`) read and write the
same area. Its location goes in `/etc/fw_env.config`:

```text
# device/file        offset   env size   sector size
/dev/mtd1            0x0      0x40000    0x20000
```

A redundant environment adds a second line with the second copy's location. The initial environment burnt at
the factory can be built straight from a text file with `mkenvimage`:

```bash
mkenvimage -s 0x40000 -o env.img env.txt       # env.txt: key=value lines, printenv style
fw_setenv -c fw_env.config BOOT_ORDER "A B"
fw_setenv -c fw_env.config BOOT_A_LEFT 3
```

U-Boot reads values written this way from Linux on the next boot:

```text
Loading Environment from Flash... OK
=> printenv BOOT_ORDER BOOT_A_LEFT
BOOT_ORDER=A B
BOOT_A_LEFT=3
```

This is how an update agent on Linux (RAUC and others) changes the bootloader's state in A/B updates.

:::note
On QEMU's flash emulation, this U-Boot's `saveenv` fails with `Flash buffer write timeout`, so in these
experiments the environment was always written with the Linux-side tools.
:::

## bootcmd and standard boot

Older U-Boot `bootcmd`s were long, board-specific scripts. Today **standard boot** (bootstd) is the default
and `bootcmd=bootflow scan -lb` is one line. It walks the boot devices (bootdevs) in `boot_targets` order,
looks on each for something a boot method (bootmeth) recognises (`-l` lists, `-b` boots as soon as one is
found), and boots the first that works.

| bootmeth | Looks for |
|---|---|
| `extlinux` | `/extlinux/extlinux.conf` or `/boot/extlinux/extlinux.conf` on a partition |
| `script` | `boot.scr` (a U-Boot script image) |
| `efi_mgr`, `efi` | UEFI boot entries, or `\EFI\BOOT\BOOTAA64.EFI` on an ESP |
| `pxe` | PXE configuration from the network |

With a disk holding `/boot/extlinux/extlinux.conf` attached, autoboot gives:

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

It tried the UEFI boot manager first, failed, then found `extlinux.conf` on the virtio disk's second
partition and booted from it. The file:

```text
default linux
timeout 1
label linux
    kernel /boot/Image
    initrd /boot/initrd.img
    append console=ttyAMA0 root=/dev/vda2 rauc.slot=A
```

The format comes from syslinux, so it is nearly identical to a syslinux config on x86, and it's easy to have a
distribution update it when installing kernels. To name a device tree file, add an `fdt /boot/dtbs/board.dtb` line.

## Booting by hand

Doing standard boot's job at the prompt shows each step:

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

- `load interface device:partition address file` loads a file into memory and leaves its size in `filesize`.
- `booti` is for the arm64 `Image` format. 32-bit ARM `zImage`s use `bootz`; legacy `uImage`s and FIT use `bootm`.
- The three arguments are the kernel address, the initramfs `address:size` and the DTB address.
  `fdtcontroladdr` is the DTB U-Boot itself uses, generated by QEMU here. On a real board you would usually
  load the DTB built with the kernel to `fdt_addr_r` and pass that.

The last three lines come from the initramfs `/init` the kernel ran. Right before booting, U-Boot copies the
DTB and writes `bootargs` and the initramfs location into its `/chosen` node, and the kernel reads them there.
U-Boot's `Loading Ramdisk to 7d440000, end 7d5358c1` matches `linux,initrd-start`/`-end` in the kernel's device
tree exactly. Before booting, the original DTB's `/chosen` has none of these:

```text
=> fdt addr ${fdtcontroladdr}
=> fdt print /chosen
chosen {
	stdout-path = "/pl011@9000000";
	rng-seed = <...>;
	kaslr-seed = <...>;
};
```

## FIT images

With the kernel, initramfs and DTB as separate files, mismatched files can end up together, and there is no
way to check integrity. **FIT** (Flattened Image Tree) bundles them into a single file in device tree format,
with a hash (and optionally a signature) for each. The source is an `.its` file:

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

On a real board you would add an `fdt` node under `images` and `fdt = "fdt";` to the configuration to bundle
the DTB too; QEMU generates its DTB at run time, so only the kernel and initramfs are in here. To support
several boards with one image, give each its own configuration and pick one with `bootm ${addr}#conf-board`.

U-Boot shows the contents and checks the hashes with `iminfo`, and `bootm` verifies them again as it boots.
The kernel is gzip-compressed, so `bootm` unpacks it to its `load` address:

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

Changing one byte of the kernel data in the loaded FIT with `mw.b` and booting again gets a refusal:

```text
=> mw.b 0x50100000 0x00 1
=> bootm 0x50000000 0x50000000 ${fdtcontroladdr}
   Verifying Hash Integrity ... sha256 error!
Bad hash value for 'hash' hash node in 'kernel' image node
Bad Data Hash
ERROR -2: can't get kernel image!
```

:::warning Hashes vs signatures
Hashes only catch accidental corruption; an attacker just recomputes them. Against forgery, the FIT gets a
**signature**: `mkimage -k keydir -K u-boot.dtb -r` signs it and embeds the public key into U-Boot's DTB as a
*required* key, and U-Boot refuses to boot any configuration it can't verify with that key. U-Boot itself,
public key included, is protected by the boot ROM's signature check from part 1. That makes a chain of trust
from the boot ROM to the kernel.
:::

## Picking an A/B slot in a boot script

Logic beyond a one-line `bootcmd` goes in a U-Boot script. The language is U-Boot's hush shell (sh-like but
much smaller), and the text file is wrapped by `mkimage` into `boot.scr`. Standard boot's `script` bootmeth
finds that file on a partition and runs it.

```bash
mkimage -A arm64 -O linux -T script -C none -n "A/B boot" -d boot.cmd boot.scr
```

As in the [A/B partitions post](/posts/architecture/os/ota-update/ab-partitions/) and the
[RAUC post](/posts/architecture/os/ota-update/rauc/), the environment holds the slot order `BOOT_ORDER` and the
remaining attempts per slot, `BOOT_A_LEFT` and `BOOT_B_LEFT`:

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

Unlike GRUB in the [GRUB post](/posts/unix/boot/grub-ab/), U-Boot has `setexpr`, so attempts can be counted
down. The disk has `boot.scr` on partition 1 (FAT) and `rootfs_A`, `rootfs_B` on partitions 2 and 3; before
each boot the environment was rebuilt from the Linux side to try two states:

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

The second is the state after an updated B failed all three attempts: `BOOT_ORDER` still points at B first, but
with no attempts left the script falls back to A. Linux's job is the same as with GRUB:

| Action | Command |
|---|---|
| Activate B after writing a new image to it | `fw_setenv BOOT_ORDER "B A"` and `fw_setenv BOOT_B_LEFT 3` |
| Confirm the booted slot is healthy | `fw_setenv BOOT_B_LEFT 3` (refill its attempts) |

:::note Quotes in hush
RAUC's documentation, and the script in the RAUC post, quote the variable:
`for BOOT_SLOT in "${BOOT_ORDER}"`. POSIX sh would loop once over the single word `"A B"`, but this U-Boot's
(2025.10) hush still split it into words: running both scenarios above with that script gave the same results.
An empty `then` branch holding only a comment worked fine in this hush too.
:::

### U-Boot's built-in bootcount

Instead of counting in the environment yourself, U-Boot's bootcount feature (`CONFIG_BOOTCOUNT_LIMIT`) can do
it. U-Boot increments `bootcount` on every boot and, once it exceeds `bootlimit`, runs `altbootcmd` instead of
`bootcmd`. The counter usually lives somewhere free to write, such as an RTC register or a SoC register that
survives a reset; when it is kept in the environment (`CONFIG_BOOTCOUNT_ENV`), it is only counted while
`upgrade_available=1`, to spare the flash. Once Linux confirms a good boot it resets `bootcount` and
`upgrade_available` to 0. Update tools such as SWUpdate use this scheme. The QEMU build used here doesn't have
it enabled (there is no `bootcount` command).
