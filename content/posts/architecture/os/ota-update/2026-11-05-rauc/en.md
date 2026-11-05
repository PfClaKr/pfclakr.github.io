+++
title = "RAUC: from a signed bundle to a booted slot"
description = "How RAUC implements A/B updates on embedded Linux: describing slots in system.conf, building and signing bundles, what rauc install actually does, the U-Boot side, and the mark-good step."
tags = ["linux", "embedded", "ota", "rauc", "yocto"]
+++

The [A/B post](/posts/architecture/os/ota-update/ab-partitions/) described the scheme: two slots,
a bootloader flag, boot counting, and a health check that marks the new slot good. Somebody still has
to verify the update, write it to the right partition, and flip the bootloader state at the right
moments. On embedded Linux, a common choice for that job is **RAUC** (Robust Auto-Update Controller),
an open-source update client started at Pengutronix.

## What RAUC does, and what it doesn't

RAUC runs on the device, as a service with a D-Bus API (`de.pengutronix.rauc.Installer`) and a
command-line tool. Its job is narrow:

- check that an update **bundle** is signed by a trusted key and made for this device;
- write the images it contains into the **inactive slots**;
- talk to the **bootloader** to switch slots, and later to mark a slot good or bad.

It does not decide *when* to update, download on a schedule, or manage a fleet. That is the job of
something above it: a server such as Eclipse hawkBit with `rauc-hawkbit-updater` on the device, or
your own agent that calls the D-Bus API.

## Describing the device: `/etc/rauc/system.conf`

RAUC learns the partition layout from one file in the root filesystem:

```ini
[system]
compatible=acme-robot-v2
bootloader=uboot
# where RAUC keeps its status (older versions: statusfile=)
data-directory=/data/rauc
# only accept verity/crypt bundles, not the older "plain" format
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

- **`compatible`** identifies the product. A bundle whose manifest has a different `compatible` is
  rejected, so a bundle for another board can never be installed by mistake.
- **Slots** are named `slot.<class>.<index>`. Slots of the same class are interchangeable copies:
  `rootfs.0` and `rootfs.1` are the A and B root filesystems.
- **`bootname`** is the name the bootloader uses for that slot (`A`, `B`). It links RAUC's view to the
  bootloader's `BOOT_ORDER`.
- Other slots can follow a root slot with `parent=`, for example an application partition that must
  always be updated together with its rootfs.

## Bundles: what you ship

A bundle (`.raucb`) is a SquashFS image that contains a **manifest** and the images, with a
**CMS signature** (X.509 certificates) attached. The manifest describes what is inside:

```ini
[update]
compatible=acme-robot-v2
version=2026.11.0

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

Building and inspecting one:

```bash
mkdir bundle && cp rootfs.ext4 manifest.raucm bundle/
rauc bundle --cert=release.cert.pem --key=release.key.pem bundle/ update.raucb
rauc info --keyring=ca.cert.pem update.raucb
```

The **`verity`** format is the one to use. The signature covers the manifest, which in turn contains
the root hash of a dm-verity hash tree over the bundle. RAUC can then check every block as it reads
it, instead of trusting a bundle that was verified once and could change afterwards on disk. It is
also what makes **streaming** possible: `rauc install https://…` reads the bundle over HTTP and
installs it without first storing it on the device (this relies on NBD support in the kernel).

On Yocto, the `meta-rauc` layer provides a `bundle` class, so a bundle is just another recipe:

```bitbake
inherit bundle

RAUC_BUNDLE_COMPATIBLE = "acme-robot-v2"
RAUC_BUNDLE_FORMAT = "verity"
RAUC_BUNDLE_SLOTS = "rootfs"
RAUC_SLOT_rootfs = "acme-image"
RAUC_KEY_FILE = "${THISDIR}/files/release.key.pem"
RAUC_CERT_FILE = "${THISDIR}/files/release.cert.pem"
```

## What `rauc install` actually does

```bash
rauc install /data/update.raucb
```

1. **Verify** the bundle signature against the keyring (the chain must lead to a trusted CA).
2. **Check** that the manifest's `compatible` matches `system.conf`.
3. **Find the booted slot**, normally from `rauc.slot=A` on the kernel command line (set by the
   bootloader), or by matching the `root=` device. The target is the other slot of each class.
4. **Mark the target slot bad** in the bootloader *before* writing, so a half-written slot can never
   be booted, even if power fails in the middle.
5. **Write** each image to its slot (for an `ext4` image on an `ext4` slot, a block copy), running any
   install hooks you defined.
6. **Mark the target slot active**: put it first in the boot order with a fresh number of attempts.

RAUC does not reboot by itself; your update agent decides when. `rauc status` shows the result:

```bash
rauc status --detailed
# Compatible:  acme-robot-v2
# Booted from: rootfs.0 (A)
# Activated:   rootfs.1 (B)
# ...
```

## The bootloader side: U-Boot

For `bootloader=uboot`, RAUC reads and writes U-Boot environment variables from Linux with
`fw_printenv`/`fw_setenv` (libubootenv, configured by `/etc/fw_env.config`). It uses exactly the
variables from the A/B post: `BOOT_ORDER` and `BOOT_<bootname>_LEFT`. The boot script on the U-Boot
side implements the counting:

```bash
test -n "${BOOT_ORDER}" || setenv BOOT_ORDER "A B"
test -n "${BOOT_A_LEFT}" || setenv BOOT_A_LEFT 3
test -n "${BOOT_B_LEFT}" || setenv BOOT_B_LEFT 3

setenv bootargs
for BOOT_SLOT in "${BOOT_ORDER}"; do
  if test "x${bootargs}" != "x"; then
    # a slot was already chosen
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

The `rauc.slot=` argument closes the loop: it is how RAUC knows, once Linux is up, which slot it is
running from. Other backends follow the same idea with different storage: barebox has `bootchooser`,
GRUB uses `grubenv`, EFI uses boot entries, and `custom` lets you call your own script.

## Marking the slot good

After an update, the new slot only has a few attempts. Something in userspace has to confirm that the
system works, and call:

```bash
rauc status mark-good      # reset the attempts of the booted slot
rauc status mark-bad       # give up on it now (next boot falls back)
rauc status mark-active other
```

A systemd unit that runs after the services that matter is a simple way to do it:

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

If `robot-healthcheck` fails, `mark-good` never runs; combined with a hardware watchdog for the hang
cases, the bootloader eventually falls back to the previous slot on its own.

## Things that matter once there is a fleet

- **Keys.** Keep separate development and release certificates, with the release key offline or in an
  HSM. The keyring can hold several CAs, which is how you rotate a key without stranding devices.
- **Test the failure paths, not only the happy path.** Pull the power during an install, ship a test
  bundle that deliberately fails its health check, and check that the device comes back on the old
  slot. Do it in CI on real hardware if possible.
- **Data and rollback.** RAUC replaces slots; it does nothing about the data partition. Migrations
  must survive running the old version again.
- **Bandwidth.** With `verity` bundles and streaming, *adaptive* updates (`adaptive=block-hash-index`
  in the manifest) let RAUC reuse blocks that are already on the device and fetch only the rest, which
  matters a lot on cellular links.
- **Hooks are code that runs as root during updates.** Keep them small, idempotent, and covered by
  the same tests as the rest of the update.

RAUC keeps the part that must never go wrong (verify, write the other slot, flip the bootloader
state) small and well tested, and leaves policy to you. That split is why it fits so well under an
A/B design: the risky decisions stay in one place, and the device can always find its way back to a
system that worked.
