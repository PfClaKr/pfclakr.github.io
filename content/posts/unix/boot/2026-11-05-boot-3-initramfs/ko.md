+++
title = "부팅 과정 3: initramfs — 커널이 진짜 루트를 찾기까지"
description = "커널과 실제 루트 파일시스템 사이에서 initramfs가 하는 일: cpio 형식과 커널이 그것을 푸는 방식, initramfs 없이도 되는 경우와 안 되는 경우, busybox로 직접 만든 /init과 switch_root, 우분투 initramfs-tools의 구조와 break= 디버깅, hook과 스크립트로 커스텀하기."
tags = ["linux", "initramfs", "boot", "embedded", "kernel"]
+++

[1편](/ko/posts/unix/boot/boot-1-firmware/)과 [2편](/ko/posts/unix/boot/boot-2-u-boot/)에서 부트로더는 커널과
함께 initramfs를 메모리에 올리고 그 위치를 커널에 알려 줬습니다. 커널은 하드웨어를 초기화한 뒤 루트
파일시스템을 마운트하고 `/sbin/init`을 실행해야 합니다. 그런데 루트가 있는 디스크의 드라이버가 모듈로 빌드되어
있거나, 루트가 LVM·RAID·LUKS 위에 있거나, 네트워크(NFS)에 있으면 커널 혼자서는 그 일을 할 수 없습니다.
initramfs는 이 사이를 메우는 **메모리 위의 작은 루트 파일시스템**입니다.

실험은 x86 QEMU에서 Ubuntu 6.8 커널로 했고, 디스크는 [GRUB A/B 글](/ko/posts/unix/boot/grub-ab/)에서 만든
이미지(ESP, `rootfs_A`, `rootfs_B`)입니다.

## 커널이 initramfs를 다루는 방식

initramfs 파일은 **cpio 아카이브**(newc 형식)이고, 보통 gzip이나 zstd로 압축되어 있습니다. 압축하지 않은
아카이브의 첫 6바이트는 newc의 매직 넘버입니다.

```text
$ head -c 6 initrd.img-6.8.0-147-generic
070701
```

커널은 부팅 초기에 이 아카이브를 **rootfs**라는 tmpfs(또는 ramfs) 인스턴스에 풀고, 그 안의 `/init`을 PID 1로
실행합니다. 커널 명령줄의 `rdinit=`으로 다른 프로그램을 지정할 수 있습니다. 몇 가지 세부 사항이 있습니다.

- 아카이브 여러 개를 **이어 붙여도** 됩니다. 커널은 차례로 풀면서 같은 경로의 파일은 뒤의 것으로 덮어씁니다.
  압축된 것과 압축되지 않은 것이 섞여도 됩니다.
- 커널 이미지 안에도 initramfs가 하나 들어 있습니다(`CONFIG_INITRAMFS_SOURCE`). 지정하지 않으면 `/dev`,
  `/dev/console`, `/root`만 든 최소 아카이브이고, 부트로더가 넘긴 것은 그 위에 풀립니다. 임베디드에서는 여기에
  initramfs 전체를 넣어 커널 파일 하나로 끝내기도 합니다.
- 풀린 결과에 `/init`이 **없으면** 커널은 예전 방식대로 `root=`의 장치를 직접 마운트하고 `/sbin/init`(또는
  `init=`)을 실행합니다.

예전에는 initramfs 대신 **initrd**라는 블록 장치 이미지(램디스크)를 썼습니다. 파일 이름 `initrd.img`나 부트로더
명령 `initrd`는 그 시절의 이름이 남은 것이고, 지금은 거의 모두 cpio 방식입니다.

## initramfs 없이도 되는 경우

드라이버가 커널에 들어 있다면 initramfs 없이도 부팅할 수 있습니다. Ubuntu의 x86 커널은 virtio 블록 장치와
ext4가 내장(`=y`)이라 이 디스크로 시험할 수 있습니다. initramfs 없이 `root=PARTLABEL=rootfs_A`로 부팅하면 커널이
직접 루트를 마운트합니다.

```text
[    2.455157] EXT4-fs (vda2): mounted filesystem cb87ce6f-... ro with ordered data mode.
[    2.455654] VFS: Mounted root (ext4 filesystem) readonly on device 253:2.
[    2.742791] Run /sbin/init as init process
rootfs: pid 1, running release 1.0 from /dev/root
```

같은 파티션을 파일시스템 레이블로 지정하면(`root=LABEL=rootA`) 패닉이 납니다.

```text
[    2.394034] Disabling rootwait; root= is invalid.
[    2.401243] VFS: Cannot open root device "LABEL=rootA" or unknown-block(0,0): error -6
[    2.401369] Please append a correct "root=" boot option; here are the available partitions:
[    2.401879]   fd01           32768 vda1 001b70e0-a665-42fc-af81-cecafcc05824
[    2.401987]   fd02           65536 vda2 a8d14487-02a0-491b-9ae9-d75f471a987c
[    2.402051]   fd03           65536 vda3 402dbd4e-e479-4cc9-bc1c-079d1ece0630
[    2.402506] Kernel panic - not syncing: VFS: Unable to mount root fs on unknown-block(0,0)
```

차이는 정보가 어디에 있느냐입니다. 커널이 직접 이해하는 `root=`는 `/dev/vda2` 같은 장치 이름, 그리고 파티션
테이블에 적힌 `PARTUUID=`와 `PARTLABEL=`뿐입니다. `LABEL=`과 `UUID=`는 **파일시스템 안**에 적힌 값이라, 장치를
하나씩 열어 슈퍼블록을 읽어 봐야 합니다. 그 일은 사용자 공간의 `blkid`와 udev가 하고, 그래서 initramfs가 필요합니다.
패닉 메시지에 나열된 파티션 옆의 값이 PARTUUID입니다.

initramfs가 필요한 대표적인 경우를 모으면 이렇습니다.

- 루트 장치나 파일시스템의 드라이버가 모듈인 경우 (배포판 커널은 대부분 그렇습니다)
- `UUID=`, `LABEL=`로 루트를 지정하는 경우
- LVM, 소프트웨어 RAID, LUKS 암호화, NFS 위의 루트
- 마운트 전에 fsck를 하거나, [OverlayFS 글](/ko/posts/architecture/os/ota-update/overlayfs/)처럼 루트를 여러 층으로
  조립하는 경우

## 직접 만들어 보기

정적 링크된 busybox 하나면 동작하는 initramfs를 만들 수 있습니다. GRUB 글의 실험에 쓴 것이 이것입니다.

```bash
mkdir -p initramfs/{bin,dev,proc,sys,newroot}
cp /usr/bin/busybox initramfs/bin/        # busybox-static 패키지의 정적 바이너리
install -m755 init initramfs/init
(cd initramfs && find . | cpio -o -H newc | gzip -9) > initrd.img
```

`/init`은 이렇습니다.

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

1. busybox의 애플릿 링크를 만들고 `/dev`, `/proc`, `/sys`를 마운트합니다. `/dev`는 커널이 장치 노드를 만들어
   주는 devtmpfs입니다.
2. 커널 명령줄에서 `root=`를 읽습니다. 커널은 자기가 모르는 인자도 `/proc/cmdline`에 그대로 남겨 두므로,
   `rauc.slot=` 같은 값도 여기서 읽습니다.
3. 루트 장치를 찾습니다. 파티션마다 `/sys/class/block/*/uevent`에 `PARTNAME=`(GPT 파티션 이름)이 있어서 udev
   없이도 찾을 수 있습니다.
4. 루트를 `/newroot`에 마운트하고, `/dev`를 그 아래로 옮기고, **`switch_root`** 합니다.

```text
initramfs: pid 1, cmdline: BOOT_IMAGE=/boot/vmlinuz-6.8.0-147-generic root=PARTLABEL=rootfs_A ro rauc.slot=A console=ttyS0
initramfs: root PARTLABEL=rootfs_A is /dev/vda2
rootfs: pid 1, running release 1.0 from /dev/vda2
```

`switch_root`는 initramfs의 파일을 모두 지워 메모리를 돌려주고, 새 루트를 `/`로 옮기고(`mount --move` 후
`chroot`), 새 루트의 init을 `exec`합니다. `exec`이라서 PID가 그대로 1이고, 마지막 줄에서 실제 루트의
`/sbin/init`도 PID 1로 출력됩니다. 일반 시스템의 `pivot_root`를 쓰지 않는 이유는 rootfs가 특수한 마운트라서
떼어 내거나 옮길 수 없기 때문입니다. 대신 그 내용을 비우고 그 위에 새 루트를 덮어씌웁니다. `switch_root`가
PID 1에서만 동작하는 것도 이 때문입니다.

## 배포판의 initramfs: initramfs-tools

배포판은 initramfs를 직접 만들지 않고 생성기를 씁니다. 데비안·우분투는 **initramfs-tools**, 페도라·RHEL 계열은
**dracut**, 아치 리눅스는 **mkinitcpio**입니다. 우분투에서는 커널 패키지를 설치할 때 `update-initramfs`가
`/boot/initrd.img-버전`을 만들고, 그 안에서 `mkinitramfs`가 실제 작업을 합니다.

Ubuntu 24.04의 `mkinitramfs`로 6.8 커널용 initramfs를 만들어 보았습니다.

```text
$ lsinitramfs initrd.img-6.8.0-147-generic | wc -l
523
$ lsinitramfs initrd.img-6.8.0-147-generic | grep -c '\.ko'
148
$ unmkinitramfs initrd.img-6.8.0-147-generic x && du -sh x/*
12M	x/early
27M	x/main
```

파일 하나에 아카이브가 두 개 있습니다. 앞쪽 `early`는 압축하지 않은 cpio인데, 들어 있는 것은 이미 zstd로
압축된 커널 모듈(`*.ko.zst`)입니다. 압축된 파일을 다시 압축하는 낭비를 피하려고 따로 둔 것이고, 그 뒤에 나머지
전부를 zstd로 압축한 `main`이 이어집니다. 인텔·AMD 마이크로코드 업데이트도 이렇게 맨 앞의 압축하지 않은 cpio에
넣습니다(`kernel/x86/microcode/…`). 커널이 압축을 풀기도 전, 부팅 아주 초기에 그 위치에서 읽어 가기 때문입니다.

`main`에는 busybox, klibc 유틸리티, `systemd-udevd`, 모듈, 그리고 셸 스크립트로 된 `/init`이 들어 있습니다.
부팅하면 그 `/init`이 단계마다 메시지를 출력합니다.

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

`/init`의 뼈대는 이렇습니다(388줄 중 핵심만).

```sh
export init=/sbin/init
export rootmnt=/root
. /scripts/functions
# … 커널 명령줄 파싱 (root=, init=, break=, debug, …)
maybe_break top
run_scripts /scripts/init-top          # udev 시작 등
maybe_break modules
# … /etc/initramfs-tools/modules 의 모듈 로드
maybe_break premount
run_scripts /scripts/init-premount
maybe_break mount
. /scripts/local                       # local-top → 루트 장치 대기 → local-premount → fsck → 마운트 → local-bottom
. /scripts/nfs
maybe_break mountroot
mountroot
maybe_break bottom
run_scripts /scripts/init-bottom       # udev 정지, /dev 를 ${rootmnt}/dev 로 이동
mount -n -o move /run ${rootmnt}/run
maybe_break init
mount -n -o move /sys ${rootmnt}/sys
mount -n -o move /proc ${rootmnt}/proc
exec run-init ${drop_caps} "${rootmnt}" "${init}" "$@" <"${rootmnt}/dev/console" >"${rootmnt}/dev/console" 2>&1
```

마지막의 `run-init`은 klibc에 든 `switch_root`와 같은 도구입니다. 직접 만든 initramfs와 하는 일이 같고, 그
사이에 udev, 모듈 로딩, 단계별 스크립트 디렉터리가 더해진 구조입니다. `/run`, `/sys`, `/proc`, `/dev`를 실제 루트로
옮기기 때문에 실제 루트에는 그 마운트 포인트가 있어야 합니다. 위 로그의 `mounting /run on /root/run failed`는
busybox로 만든 실험용 루트에 `/run` 디렉터리가 없어서 나온 경고입니다.

`root=PARTLABEL=`은 `scripts/functions`의 `resolve_device`가 `LABEL=`, `UUID=`, `PARTUUID=`와 함께 처리합니다.
udev가 만든 `/dev/disk/by-partlabel/` 같은 링크를 쓰는 방식입니다.

## 디버깅: break=

initramfs에서 루트를 찾지 못하면 보통 `(initramfs)` 프롬프트의 셸로 떨어집니다. 일부러 멈추려면 커널 명령줄에
`break=단계`를 줍니다. 단계 이름은 위 뼈대의 `maybe_break` 인자들(`top`, `modules`, `premount`, `mount`, `mountroot`,
`bottom`, `init`)입니다. `break=premount`로 부팅한 결과입니다.

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

- 셸의 PID가 1이 아니라 139입니다. PID 1은 여전히 `/init`이고, 셸은 그 자식으로 떠 있습니다. `exit`하면 `/init`이
  멈췄던 곳부터 부팅을 이어 갑니다.
- `/`는 `rootfs`, 즉 커널이 initramfs를 푼 그 파일시스템입니다.
- udev가 이미 돌아서 `/dev/disk/by-partlabel/` 링크가 있습니다. 루트를 못 찾는 문제라면 여기서 `blkid`,
  `ls /dev`, `cat /proc/partitions`로 커널이 장치를 보고 있는지부터 확인합니다.

그 밖에 쓸모 있는 인자입니다.

| 인자 | 효과 |
|---|---|
| `debug` | initramfs-tools: `/init`을 `sh -x`처럼 추적해 `/run/initramfs/initramfs.debug`에 기록 |
| `rdinit=/bin/sh` | `/init` 대신 셸을 PID 1로. 스크립트를 전혀 거치지 않음 |
| `panic=10` | 커널 패닉 10초 뒤 재부팅. 원격 장비에서 패닉으로 멈춰 있지 않게 |
| `rd.break`, `rd.shell`, `rd.debug` | dracut에서 같은 역할 (`rd.break=pre-mount` 처럼 단계 지정) |

## 커스텀: hook과 스크립트

initramfs-tools에는 커스텀 지점이 두 종류 있습니다.

- **hook**(`/etc/initramfs-tools/hooks/`): `mkinitramfs`가 이미지를 **만들 때** 실행됩니다. 무엇을 넣을지 정합니다.
- **스크립트**(`/etc/initramfs-tools/scripts/<단계>/`): initramfs 안에 들어가서 **부팅할 때** 해당 단계에서 실행됩니다.

예로 `blkid`를 넣는 hook과, 루트를 마운트한 직후 슬롯 정보를 출력하는 스크립트를 만들어 보았습니다.

```sh
#!/bin/sh
# /etc/initramfs-tools/hooks/slotinfo -- mkinitramfs 때 실행
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /usr/share/initramfs-tools/hook-functions
copy_exec /usr/sbin/blkid /sbin
```

```sh
#!/bin/sh
# /etc/initramfs-tools/scripts/local-bottom/slotinfo -- 부팅 중, 루트를 ${rootmnt}에 마운트한 뒤 실행
PREREQ=""
prereqs() { echo "$PREREQ"; }
case "$1" in prereqs) prereqs; exit 0 ;; esac
. /scripts/functions

for x in $(cat /proc/cmdline); do
  case "$x" in rauc.slot=*) slot=${x#rauc.slot=} ;; esac
done
log_success_msg "slot ${slot:-?}, root ${ROOT} = $(blkid -o value -s PARTLABEL "$(resolve_device "$ROOT")")"
```

- 앞부분의 `PREREQ`와 `prereqs`는 정해진 틀입니다. initramfs-tools가 스크립트 사이의 실행 순서를 정할 때 씁니다.
- `copy_exec`는 바이너리와 그것이 링크하는 공유 라이브러리를 함께 복사합니다. 결과 이미지에 `usr/sbin/blkid`와
  `libblkid.so.1`이 같이 들어갔습니다. 모듈은 `manual_add_modules`로 넣습니다.
- 두 파일 모두 실행 권한이 있어야 하고, 바꾼 뒤에는 `update-initramfs -u`로 다시 만들어야 반영됩니다.

`rauc.slot=A`를 붙여 부팅하면 `local-bottom` 단계에 메시지가 나옵니다.

```text
Begin: Running /scripts/local-premount ... done.
Warning: fsck not present, so skipping root file system
done.
Begin: Running /scripts/local-bottom ... Success: slot A, root /dev/vda2 = rootfs_A
done.
Begin: Running /scripts/init-bottom ... done.
```

`local-bottom`에서 이미 `ROOT`가 `/dev/vda2`로 풀려 있는 것도 보입니다. 같은 자리에서 루트를 오버레이로 바꾸거나,
슬롯별 데이터 파티션을 고르거나, 부팅 실패를 기록하는 일을 할 수 있습니다.
