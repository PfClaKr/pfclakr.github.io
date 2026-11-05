+++
title = "부팅 과정 2: U-Boot — 환경 변수, bootcmd, FIT, 그리고 A/B 스크립트"
description = "임베디드 리눅스의 부트로더 U-Boot가 하는 일을 QEMU arm64에서 실제로 돌려 보며 정리합니다: 환경 변수와 리눅스 쪽 fw_setenv, standard boot와 extlinux.conf, 손으로 하는 booti, 해시로 검증되는 FIT 이미지, 부트 스크립트로 만드는 A/B 슬롯 선택."
tags = ["linux", "u-boot", "boot", "embedded", "arm64"]
+++

[1편](/ko/posts/unix/boot/boot-1-firmware/)에서 ARM SoC는 부트 ROM이 작은 1단계 로더(U-Boot SPL 등)를
SRAM에 올리고, 그 로더가 DRAM을 초기화한 뒤 다음 단계를 올린다고 했습니다. 그 다음 단계로 가장 흔히 쓰이는
것이 U-Boot입니다. 이 글은 U-Boot가 커널을 찾아 올리고 점프하기까지를 다룹니다.

실험 환경은 QEMU의 `virt` 머신(Cortex-A57), Ubuntu 24.04의 `u-boot-qemu` 패키지에 든 U-Boot 2025.10,
Ubuntu의 arm64 커널 6.8입니다. initramfs는 busybox로 만든 최소 구성으로, 부트로더에게서 무엇을 넘겨받았는지
출력하고 전원을 끕니다.

## 시작 화면 읽기

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

- `DRAM`: 쓸 수 있는 메모리 크기. 실제 보드에서는 SPL이 초기화한 결과를 이어받습니다.
- `devicetree: board`: U-Boot 자신도 장치 트리로 하드웨어를 파악합니다. QEMU에서는 QEMU가 만들어 준 DTB를 씁니다.
- `Loading Environment from Flash... bad CRC`: 플래시에 저장된 환경 변수가 없거나 깨져서 U-Boot에 컴파일되어
  들어 있는 기본값을 쓴다는 뜻입니다. 처음 부팅하는 보드에서 흔히 보는 경고입니다.
- `Hit any key to stop autoboot`: `bootdelay`초 안에 키를 누르면 `=>` 프롬프트로 들어가고, 아니면 `bootcmd`
  변수의 내용을 실행합니다.

## 환경 변수

U-Boot의 동작은 대부분 환경 변수로 정합니다. 프롬프트에서 몇 개를 보면 이렇습니다.

```text
=> printenv bootcmd boot_targets kernel_addr_r ramdisk_addr_r bootdelay
bootcmd=bootflow scan -lb
boot_targets=qfw usb scsi virtio nvme dhcp
kernel_addr_r=0x40400000
ramdisk_addr_r=0x44000000
bootdelay=2
```

| 변수 | 의미 |
|---|---|
| `bootcmd` | 자동 부팅 때 실행할 명령 |
| `bootargs` | 커널 명령줄. 부팅할 때 장치 트리의 `/chosen/bootargs`에 들어갑니다 |
| `bootdelay` | 자동 부팅 전 대기 시간(초). `-1`이면 자동 부팅 안 함 |
| `kernel_addr_r`, `ramdisk_addr_r`, `fdt_addr_r` | 커널, initramfs, DTB를 올릴 메모리 주소의 관례적인 이름 |
| `boot_targets` | standard boot가 부팅 장치를 찾는 순서 |

`setenv`로 바꾸고 `saveenv`로 저장합니다. 저장 위치는 빌드 설정에 따라 SPI 플래시, eMMC의 특정 오프셋,
FAT 파티션의 `uboot.env` 파일, UBI 볼륨 등이고, `env default -a`로 기본값으로 돌아갑니다.

### 저장 형식과 리눅스 쪽 도구

저장된 환경은 단순한 형식입니다. 앞 4바이트가 CRC32이고, 그 뒤에 `이름=값` 문자열이 NUL로 구분되어 이어지며,
남은 공간은 채움 바이트로 정해진 크기를 채웁니다.

```text
000000   +   K 305   S   a   r   c   h   =   a   r   m  \0   b   a   u
000010   d   r   a   t   e   =   1   1   5   2   0   0  \0   b   o   a
```

이 QEMU 빌드는 플래시 두 번째 뱅크의 처음 256 KiB(`0x40000`)를 씁니다. `printenv`의 마지막 줄
`Environment size: 514/262140 bytes`에서 262140이 256 KiB에서 CRC 4바이트를 뺀 값입니다. 설정에 따라 두 벌을
번갈아 쓰는 **중복 환경**(redundant environment)도 있습니다. 앞에 플래그 바이트가 하나 더 붙고, 쓰는 도중
전원이 나가도 한 벌은 온전히 남습니다.

리눅스에서는 `libubootenv`(또는 U-Boot 소스의 `tools/env`)의 `fw_printenv`, `fw_setenv`로 같은 영역을
읽고 씁니다. 위치는 `/etc/fw_env.config`에 적습니다.

```text
# 장치/파일          오프셋   환경 크기  섹터 크기
/dev/mtd1            0x0      0x40000    0x20000
```

중복 환경이면 두 번째 줄에 두 번째 사본의 위치를 적습니다. 공장에서 굽는 초기 환경은 `mkenvimage`로 텍스트
파일에서 바로 만듭니다.

```bash
mkenvimage -s 0x40000 -o env.img env.txt       # env.txt: printenv 형식의 key=value 줄들
fw_setenv -c fw_env.config BOOT_ORDER "A B"
fw_setenv -c fw_env.config BOOT_A_LEFT 3
```

이렇게 리눅스 쪽에서 쓴 값을 다음 부팅에서 U-Boot가 그대로 읽습니다.

```text
Loading Environment from Flash... OK
=> printenv BOOT_ORDER BOOT_A_LEFT
BOOT_ORDER=A B
BOOT_A_LEFT=3
```

A/B 업데이트에서 리눅스의 업데이트 에이전트(RAUC 등)가 부트로더 상태를 바꾸는 방법이 바로 이것입니다. 참고로
QEMU의 플래시 에뮬레이션에서는 이 U-Boot의 `saveenv`가 `Flash buffer write timeout`으로 실패해서, 이 글의
실험에서는 환경을 항상 리눅스 쪽 도구로 만들었습니다.

## bootcmd와 standard boot

예전 U-Boot의 `bootcmd`는 보드마다 다른 긴 스크립트였습니다. 지금은 **standard boot**(bootstd)가 기본이라
`bootcmd=bootflow scan -lb` 한 줄입니다. `boot_targets` 순서대로 부팅 장치(bootdev)를 돌면서, 각 장치에서
부팅 방법(bootmeth)이 알아볼 수 있는 것을 찾고(`-l`은 목록 출력, `-b`는 찾는 즉시 부팅) 처음 성공한 것으로
부팅합니다.

| bootmeth | 찾는 것 |
|---|---|
| `extlinux` | 파티션의 `/extlinux/extlinux.conf` 또는 `/boot/extlinux/extlinux.conf` |
| `script` | `boot.scr` (U-Boot 스크립트 이미지) |
| `efi_mgr`, `efi` | UEFI 부트 항목, 또는 ESP의 `\EFI\BOOT\BOOTAA64.EFI` |
| `pxe` | 네트워크의 PXE 설정 |

`/boot/extlinux/extlinux.conf`가 든 디스크를 붙이고 자동 부팅에 맡긴 결과입니다.

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

UEFI 부트 매니저를 먼저 시도했다가 실패하고, virtio 디스크의 두 번째 파티션에서 `extlinux.conf`를 찾아
부팅했습니다. 그 파일은 이렇습니다.

```text
default linux
timeout 1
label linux
    kernel /boot/Image
    initrd /boot/initrd.img
    append console=ttyAMA0 root=/dev/vda2 rauc.slot=A
```

syslinux에서 가져온 형식이라 x86의 syslinux 설정과 거의 같고, 배포판이 커널을 설치할 때 이 파일을 갱신하게
만들기도 쉽습니다. 장치 트리를 파일로 지정하려면 `fdt /boot/dtbs/보드.dtb` 줄을 추가합니다.

## 손으로 부팅하기

standard boot가 하는 일을 프롬프트에서 직접 해 보면 각 단계가 보입니다.

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

- `load 인터페이스 장치:파티션 주소 파일`로 파일을 메모리에 올립니다. 끝나면 읽은 크기가 `filesize` 변수에
  들어갑니다.
- `booti`는 arm64 `Image` 형식용입니다. 32비트 ARM의 `zImage`는 `bootz`, 레거시 `uImage`와 FIT은 `bootm`을 씁니다.
- 세 인자는 커널 주소, initramfs `주소:크기`, DTB 주소입니다. `fdtcontroladdr`는 U-Boot 자신이 쓰는 DTB이고,
  QEMU에서는 QEMU가 만든 것입니다. 실제 보드라면 보통 커널과 함께 빌드된 DTB 파일을 `fdt_addr_r`에 올려서 넘깁니다.

마지막 세 줄은 커널이 실행한 initramfs의 `/init`이 출력한 것입니다. U-Boot는 부팅 직전에 DTB를 복사하면서
`/chosen` 노드에 `bootargs`와 initramfs 위치를 써 넣고, 커널은 그 값을 읽습니다. U-Boot가 출력한
`Loading Ramdisk to 7d440000, end 7d5358c1`과 커널 쪽 장치 트리의 `linux,initrd-start`·`-end`가 정확히 같습니다.
부팅 전 원래 DTB의 `/chosen`에는 이 값들이 없습니다.

```text
=> fdt addr ${fdtcontroladdr}
=> fdt print /chosen
chosen {
	stdout-path = "/pl011@9000000";
	rng-seed = <...>;
	kaslr-seed = <...>;
};
```

## FIT 이미지

커널, initramfs, DTB를 따로따로 두면 짝이 맞지 않는 파일끼리 섞일 수 있고, 무결성을 확인할 방법도 없습니다.
**FIT**(Flattened Image Tree)은 이들을 장치 트리 형식의 파일 하나로 묶고 각각의 해시(선택적으로 서명)를 함께
담습니다. 원본은 `.its` 파일로 씁니다.

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

실제 보드라면 `images`에 `fdt` 노드를 넣고 configuration에 `fdt = "fdt";`를 추가해서 DTB도 함께 묶습니다.
QEMU는 DTB를 실행할 때 만들기 때문에 여기서는 커널과 initramfs만 넣었습니다. 보드 여러 종을 이미지 하나로
지원하려면 configuration을 보드마다 하나씩 두고 `bootm ${addr}#conf-보드` 형태로 고릅니다.

U-Boot는 `iminfo`로 내용과 해시를 확인하고, `bootm`이 부팅하면서 다시 검증합니다. 커널은 gzip으로 압축되어
있어서 `bootm`이 `load` 주소에 풀어 놓습니다.

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

메모리에 올린 FIT의 커널 데이터 한 바이트를 `mw.b`로 바꾸고 다시 부팅하면 이렇게 거부합니다.

```text
=> mw.b 0x50100000 0x00 1
=> bootm 0x50000000 0x50000000 ${fdtcontroladdr}
   Verifying Hash Integrity ... sha256 error!
Bad hash value for 'hash' hash node in 'kernel' image node
Bad Data Hash
ERROR -2: can't get kernel image!
```

해시는 우발적인 손상만 막습니다. 공격자는 해시까지 다시 계산하면 되기 때문입니다. 위조를 막으려면 FIT에
**서명**을 넣습니다. `mkimage -k 키디렉터리 -K u-boot.dtb -r`로 서명하면서 공개키를 U-Boot의 DTB에 "필수 키"로
심고, U-Boot는 그 키로 서명을 확인하지 못한 configuration은 부팅하지 않습니다. 공개키가 든 U-Boot 자체는
1편의 부트 ROM 서명 검사가 보호합니다. 이렇게 부트 ROM에서 커널까지 이어지는 신뢰 사슬이 만들어집니다.

## 부트 스크립트로 A/B 슬롯 고르기

`bootcmd`보다 복잡한 로직은 U-Boot 스크립트로 씁니다. 문법은 U-Boot의 hush 셸(sh와 비슷하지만 훨씬 작음)이고,
텍스트 파일을 `mkimage`로 감싸서 `boot.scr`을 만듭니다. standard boot의 `script` bootmeth가 파티션에서 이 파일을
찾아 실행합니다.

```bash
mkimage -A arm64 -O linux -T script -C none -n "A/B boot" -d boot.cmd boot.scr
```

[A/B 파티션 글](/ko/posts/architecture/os/ota-update/ab-partitions/)과 [RAUC 글](/ko/posts/architecture/os/ota-update/rauc/)의
방식대로, 슬롯 순서 `BOOT_ORDER`와 슬롯별 남은 시도 횟수 `BOOT_A_LEFT`, `BOOT_B_LEFT`를 환경 변수로 둡니다.

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

[GRUB 글](/ko/posts/unix/boot/grub-ab/)의 GRUB과 달리 U-Boot에는 `setexpr`가 있어서 시도 횟수를 카운트다운으로
셀 수 있습니다. 디스크는 1번 파티션(FAT)에 `boot.scr`, 2·3번에 `rootfs_A`, `rootfs_B`를 두었고, 부팅할 때마다
환경을 리눅스 쪽에서 새로 만들어 두 상태를 시험했습니다.

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

두 번째는 업데이트한 B가 세 번 모두 실패한 뒤의 상태입니다. `BOOT_ORDER`가 B를 먼저 가리켜도 남은 횟수가
0이라 A로 돌아갑니다. 리눅스의 할 일은 GRUB 때와 같습니다.

| 동작 | 명령 |
|---|---|
| B에 새 이미지를 쓴 뒤 활성화 | `fw_setenv BOOT_ORDER "B A"` 와 `fw_setenv BOOT_B_LEFT 3` |
| 부팅한 슬롯이 정상임을 확인 | `fw_setenv BOOT_B_LEFT 3` (시도 횟수를 다시 채움) |

hush 문법에서 알아 둘 점이 하나 있습니다. RAUC 문서와 RAUC 글의 스크립트는 `for BOOT_SLOT in "${BOOT_ORDER}"`처럼
변수를 따옴표로 감쌉니다. POSIX sh라면 `"A B"` 한 덩어리로 한 번만 돌겠지만, 이 U-Boot(2025.10)의 hush에서는
따옴표가 있어도 단어마다 나뉘어 돌았습니다. 위 두 시나리오를 그 스크립트로 돌려도 결과가 같았습니다. 반대로
`then` 다음에 주석만 있는 빈 분기도 이 hush에서는 문제없이 동작했습니다.

### U-Boot에 내장된 bootcount

환경 변수로 직접 세는 대신 U-Boot의 bootcount 기능(`CONFIG_BOOTCOUNT_LIMIT`)을 쓸 수도 있습니다. U-Boot가
부팅할 때마다 `bootcount`를 올리고, 그 값이 `bootlimit`을 넘으면 `bootcmd` 대신 `altbootcmd`를 실행합니다.
카운터는 RTC 레지스터나 SoC의 재부팅 후에도 유지되는 레지스터처럼 쓰기 비용이 없는 곳에 두는 것이 보통이고,
환경 변수에 저장하는 설정(`CONFIG_BOOTCOUNT_ENV`)에서는 플래시 마모를 줄이려고 `upgrade_available=1`일 때만
셉니다. 리눅스는 정상 부팅을 확인하면 `bootcount`와 `upgrade_available`을 0으로 되돌립니다. SWUpdate 같은
업데이트 도구가 이 방식을 씁니다. 이 글의 QEMU용 빌드에는 이 기능이 켜져 있지 않습니다(`bootcount` 명령이 없음).
