+++
title = "OS 이미지 빌드 2: Packer로 OVA와 RAUC용 rootfs 만들기"
description = "Packer virtualbox-iso 템플릿으로 Debian netinst를 설치하고, A/B 레이아웃과 RAUC를 넣고, 빌드 VM에만 해당하는 것들을 지운 뒤 OVA와 rootfs.ext4를 함께 내보내는 과정. 템플릿과 프로비저닝 스크립트를 한 줄씩."
tags = ["linux", "debian", "packer", "virtualbox", "rauc", "ota"]
draft = true
date = 2026-11-05T14:00:00
+++

[1편](/ko/posts/architecture/os/image-build/image-1-preseed/)의 preseed는 "사람 없이 설치되는 Debian"까지만
책임집니다. 이 글에서는 그 설치를 Packer로 감싸서, 설치가 끝난 VM을 제품 이미지로 바꾸고 두 가지 산출물을
내보냅니다.

- **OVA**: 새 VM이나 장비에 그대로 올리는 공장 이미지. ESP, 슬롯 A(채워짐), 슬롯 B(비어 있음), 데이터
  파티션이 들어 있습니다.
- **rootfs.ext4**: 같은 빌드의 루트 파일시스템만 담은 이미지. 3편에서 RAUC 번들로 서명해 이미 배포된
  장비에 보냅니다.

## 저장소 구조

```text
os-image/
├── debian.pkr.hcl            source(VM 정의)와 build(단계)
├── variables.pkr.hcl
├── http/
│   └── preseed.pkrtpl        1편의 preseed, templatefile()로 채움
├── files/                    VM에 올려서 설치할 파일
│   ├── esp-grub.cfg          ESP의 슬롯 선택기 (GRUB A/B 글)
│   ├── 10_ab                 슬롯 안의 update-grub 스크립트 (GRUB A/B 글)
│   ├── system.conf           RAUC 슬롯 정의
│   ├── ca.cert.pem           번들 서명을 확인할 CA 인증서
│   ├── authorized_keys       완성된 이미지에 들어갈 SSH 공개 키
│   ├── 20-wired.network
│   └── ssh-hostkeys.service
├── scripts/
│   ├── 10-packages.sh
│   ├── 20-ab-layout.sh
│   ├── 90-cleanup.sh
│   └── 95-export-rootfs.sh
├── rauc/manifest.raucm.in    3편
└── .gitlab-ci.yml            3편
```

## Packer가 하는 일

`packer build`를 실행하면 `virtualbox-iso` 빌더가 이 순서로 움직입니다.

1. ISO를 받아 체크섬을 확인하고 캐시에 둡니다(`PACKER_CACHE_DIR`).
2. VM을 만들고 디스크와 ISO를 붙입니다.
3. preseed를 내려줄 **HTTP 서버**를 띄웁니다.
4. VM을 켜고 **`boot_command`를 키보드 입력으로 칩니다.**
5. 설치가 끝나고 재부팅되면 **SSH로 접속될 때까지** 기다립니다.
6. 프로비저너(파일 업로드, 셸 스크립트)를 차례로 실행합니다.
7. `shutdown_command`로 VM을 끄고 **OVA로 내보낸 뒤** VM을 지웁니다.

## 플러그인과 변수

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

빌더는 Packer 본체와 따로 배포되는 플러그인입니다. `packer init .`이 `required_plugins`를 보고 받아서
`~/.config/packer/plugins`에 설치합니다. 버전을 `~> 1.1`로 묶어 두면 CI에서 어느 날 갑자기 2.x가 받아지는
일이 없습니다.

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

- `sensitive = true`인 변수는 Packer가 로그에 값을 출력하지 않습니다. 값은 `-var`가 아니라 환경 변수
  `PKR_VAR_root_password`로 넘깁니다. `PKR_VAR_<이름>`은 Packer가 자동으로 읽는 형식이라 CI 변수와 잘 맞습니다.
- **ISO 주소가 두 개인 이유**: Debian은 포인트 릴리스(13.8)가 나오면 이전 ISO를 `release/`에서
  `archive/`로 옮깁니다. `iso_urls`는 앞에서부터 차례로 시도하므로 13.7.0을 고정해 둔 템플릿이 다음
  릴리스 뒤에도 깨지지 않습니다. `current/` 주소는 쓰지 않습니다. 내용이 바뀌면 체크섬이 맞지 않습니다.
- 체크섬은 `file:…/SHA256SUMS`로 원격 파일을 가리킬 수도 있지만, 값을 직접 적어 두면 저장소만 보고도
  어떤 ISO로 빌드했는지 알 수 있습니다.

## source: VM 정의

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
  # (계속)
```

- **`firmware = "efi"`**: A/B 선택기가 ESP의 `\EFI\BOOT\BOOTX64.EFI`에 있으므로 UEFI로 부팅해야 합니다.
- **디스크 10GB**: 1편의 레시피대로 ESP 256MB, 슬롯 둘 3GB씩, 나머지가 데이터입니다. 데이터 파티션은 빌드
  중에 rootfs 이미지를 잠시 만들어 두는 자리로도 씁니다(뒤에서 설명).
- **`hard_drive_discard`, `hard_drive_nonrotational`**: 디스크를 SSD로 보이게 하고 TRIM을 VirtualBox까지
  전달합니다. 마지막에 `fstrim`을 하면 지운 블록이 디스크 이미지에서도 빠져서 OVA가 작아집니다.
- **`guest_additions_mode = "disable"`**: VirtualBox 게스트 확장은 제품 이미지에 넣을 것이 아닙니다. 기본값은
  ISO를 업로드하는 것이라 꺼 둡니다.

### preseed 전달: http_content

```hcl
  http_content = {
    "/preseed.cfg" = templatefile("${path.root}/http/preseed.pkrtpl", {
      hostname      = "debian"
      apt_proxy     = var.apt_proxy
      root_password = var.root_password
    })
  }
```

`http_directory`는 디렉터리를 그대로 서비스하고, `http_content`는 **경로와 내용을 직접** 줍니다.
`templatefile()`로 preseed의 `${root_password}` 같은 자리를 채운 결과를 메모리에서 바로 서비스하므로
비밀번호가 들어간 preseed 파일이 디스크에 생기지 않습니다.

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

- `boot_wait`만큼 기다린 뒤 문자열을 **키 입력으로** 보냅니다. `<enter>`, `<esc>` 같은 특수 키와 `<wait>`(1초),
  `<wait5>`(5초)를 섞어 씁니다. 배열은 그냥 이어 붙여지므로 줄을 나눈 것은 읽기 위해서입니다.
- `c`는 netinst ISO의 GRUB 메뉴에서 명령줄로 들어가는 키입니다. 1편에서 본 세 줄을 그대로 칩니다.
- `{{ .HTTPIP }}`와 `{{ .HTTPPort }}`는 Packer가 채웁니다. VirtualBox NAT에서 게스트가 보는 호스트 주소는
  `10.0.2.2`이고, 포트는 빌드마다 8000–9000 사이에서 고릅니다.
- **`<wait>`가 너무 짧으면** GRUB이 키를 다 받기 전에 다음 줄이 들어갑니다. 빌드가 느린 러너에서는 첫
  `boot_wait`를 넉넉히 줍니다. 키 입력이 맞게 들어갔는지는 `headless = false`로 한 번 보면 바로 압니다.

### VirtualBox 7의 NAT

```hcl
  vboxmanage = [
    ["modifyvm", "{{ .Name }}", "--nat-localhostreachable1", "on"],
  ]
```

`vboxmanage`는 VM을 만든 뒤 켜기 전에 실행할 `VBoxManage` 명령 목록입니다. VirtualBox 7.0부터 NAT로 나간
게스트가 `10.0.2.2`를 통해 호스트의 루프백에 닿는 동작이 **기본으로 꺼져** 있습니다. 이걸 켜지 않으면
설치 프로그램이 preseed를 받지 못하고 "preseed 파일을 가져올 수 없음" 화면에서 멈춥니다. 러너에
apt-cacher-ng를 둘 때도 같은 이유로 필요합니다.

### SSH와 내보내기

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

- `ssh_timeout`은 **설치 시간 전체**를 포함합니다. Packer는 VM을 켠 순간부터 SSH를 기다리고, 설치가 끝나
  재부팅돼야 sshd가 뜹니다. 미러가 느리면 20–30분도 걸리니 넉넉히 줍니다.
- `format = "ova"`면 마지막에 `VBoxManage export`로 OVF 디스크립터, VMDK, 매니페스트를 한 파일로 묶습니다.
  `export_opts`는 그 명령에 그대로 붙는 인자입니다. `--manifest`는 OVA 안에 파일별 SHA256 목록을 넣고,
  `--product`, `--version`은 가져오기(import) 화면에 보입니다.

## build: 단계

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

- `source`가 `/`로 끝나면 **디렉터리의 내용**을 `destination` 안에 올립니다. 슬래시가 없으면 `files`
  디렉터리 자체가 그 안에 들어갑니다.
- `/tmp`는 Debian 13부터 tmpfs라 올린 파일이 이미지에 남지 않습니다.
- 셸 프로비저너는 스크립트를 VM에 올리고 `environment_vars`를 앞에 붙여 실행합니다. 하나라도 0이 아닌
  코드로 끝나면 빌드가 멈추고, 기본 동작으로 VM을 지웁니다. 실패한 VM을 들여다보고 싶으면 로컬에서
  `packer build -on-error=abort`로 실행합니다.
- **`direction = "download"`**는 반대로 VM의 파일을 빌드 머신으로 가져옵니다. RAUC용 rootfs 이미지를 이렇게
  꺼냅니다.

## 10-packages.sh

```sh
#!/bin/sh
set -eu
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
    rauc rauc-service gdisk efibootmgr grub-efi-amd64-bin systemd-resolved
```

- Debian은 RAUC를 둘로 나눠 패키징합니다. `rauc`는 명령줄 도구이고, `rauc-service`에 D-Bus로 활성화되는
  `rauc.service`가 들어 있습니다. Debian의 `rauc` 명령은 이 서비스에 요청을 보내는 방식으로 동작하므로
  `rauc install`과 `rauc status`를 쓰려면 둘 다 있어야 합니다.
- `gdisk`는 파티션 이름을 붙이는 `sgdisk`, `grub-efi-amd64-bin`은 ESP용 GRUB을 만들 모듈입니다.
  `systemd-resolved`는 정리 단계에서 네트워크 설정을 바꿀 때 씁니다. 그 단계에서는 네트워크를 다시 쓰지 않도록
  패키지 설치를 여기서 전부 끝냅니다.

## 20-ab-layout.sh: 일반 설치를 A/B로

설치 프로그램이 만든 것은 평범한 Debian입니다. 이 스크립트가 [GRUB A/B 글](/ko/posts/unix/boot/grub-ab/)의
구조로 바꿉니다. 각 부분이 왜 그런지는 그 글에 있고, 여기서는 이미지 빌드에서 달라지는 점만 봅니다.

```sh
# 1. 파티션 이름. 아래는 전부 UUID가 아니라 이름으로 가리킨다.
disk=/dev/$(lsblk -no PKNAME "$(findmnt -no SOURCE /)")
sgdisk -c 1:esp -c 2:rootfs_A -c 3:rootfs_B -c 4:data "$disk"

cat > /etc/fstab <<'EOT'
/dev/root       /          ext4  errors=remount-ro  0 1
PARTLABEL=esp   /boot/efi  vfat  umask=0077         0 1
PARTLABEL=data  /data      ext4  defaults           0 2
EOT
```

**fstab을 다시 쓰는 이유**가 이 시리즈에서 가장 중요한 부분입니다. 설치 프로그램은 fstab에 파일시스템
UUID를 적습니다. 그런데 UUID는 `mkfs`를 할 때마다 새로 생기므로 **빌드마다 다릅니다.** 1.0 OVA로 만든 장비에
1.1 빌드의 rootfs를 RAUC로 쓰면, 1.1의 fstab에는 1.1 빌드 VM의 ESP UUID가 적혀 있어서 `/boot/efi`가
마운트되지 않습니다. 그러면 RAUC가 grubenv를 고칠 수 없어 mark-good도 실패합니다.

- ESP와 데이터는 **GPT 파티션 이름**으로 마운트합니다. 파티션 테이블은 공장에서 한 번 쓰고 업데이트가
  건드리지 않으므로 어느 빌드의 rootfs에서 봐도 같습니다.
- 루트는 `/dev/root`라고만 적습니다. 실제 루트는 커널 명령줄의 `root=PARTLABEL=rootfs_$SLOT`이 정하고,
  fstab의 이 줄은 마운트 옵션(`errors=remount-ro`)과 rw 재마운트에만 쓰입니다. Yocto 이미지의 기본 fstab도
  같은 방식입니다.

```sh
# 2. 배포판의 EFI 부팅 경로를 걷어낸다. 슬롯에는 grub-mkconfig/update-grub만 남긴다.
for n in $(efibootmgr | sed -n 's/^Boot\([0-9A-F]\{4\}\)\*\{0,1\} debian.*/\1/p'); do
    efibootmgr -q -b "$n" -B
done
apt-mark manual grub-common grub2-common grub-efi-amd64-bin
apt-get purge -y --allow-remove-essential grub-efi-amd64 grub-efi-amd64-signed shim-signed
apt-get autoremove -y --purge
rm -rf /boot/efi/EFI/debian /boot/efi/EFI/BOOT
```

- 설치 프로그램은 NVRAM에 `debian` 부트 엔트리를 만들고 그것을 맨 앞에 둡니다. 남겨 두면 펌웨어가 우리
  선택기를 건너뛰고 `\EFI\debian\shimx64.efi`로 부팅합니다. 엔트리와 디렉터리를 지워서 펌웨어가 기본 경로
  `\EFI\BOOT\BOOTX64.EFI`로 오게 합니다.
- `grub-efi-amd64`, `-signed`, `shim-signed`는 설치·업그레이드 때 `grub-install`을 실행합니다. 슬롯 안에 있으면
  업데이트로 받은 rootfs가 ESP를 덮어쓸 수 있어서 지웁니다. 모듈만 있는 `grub-efi-amd64-bin`은 남깁니다.
  `grub-mkimage`가 그 모듈로 ESP용 GRUB을 만듭니다.
- 먼저 `apt-mark manual`을 해 두지 않으면 `autoremove`가 의존성으로만 설치된 `grub-common`까지 지웁니다.
- **`--allow-remove-essential`이 필요합니다.** Debian은 `grub-efi-amd64-signed`를 `Protected: yes`로 표시해 두어서,
  apt가 이것을 필수 패키지로 취급하고 `-y`만으로는 지우지 않습니다. 실수로 지워 부팅이 안 되는 일을 막는
  장치인데, 여기서는 바로 그 부트로더를 우리 것으로 바꾸려는 것이라 명시적으로 허락합니다.

  ```text
  WARNING: The following essential packages will be removed.
  This should NOT be done unless you know exactly what you are doing!
    grub-efi-amd64-signed
  E: Essential packages were removed and -y was used without --allow-remove-essential.
  ```

```sh
# 3. 슬롯 쪽: update-grub이 root=PARTLABEL=rootfs_$SLOT으로 부팅하는 엔트리 하나를 만든다.
for s in 00_header 10_linux 20_linux_xen 25_bli 30_os-prober 30_uefi-firmware; do
    [ -e /etc/grub.d/$s ] && chmod -x /etc/grub.d/$s
done
install -m 0755 $F/10_ab /etc/grub.d/10_ab
sed -i 's/^GRUB_CMDLINE_LINUX=.*/GRUB_CMDLINE_LINUX="console=tty0 console=ttyS0,115200"/' /etc/default/grub
update-grub

# 4. ESP 쪽: 직접 만든 GRUB, 선택기, grubenv.
install -d /boot/efi/EFI/BOOT
grub-mkimage -O x86_64-efi -p /EFI/BOOT -o /boot/efi/EFI/BOOT/BOOTX64.EFI \
    part_gpt fat ext2 normal configfile loadenv test echo linux gzio sleep halt efi_gop
install -m 0644 $F/esp-grub.cfg /boot/efi/EFI/BOOT/grub.cfg
grub-editenv /boot/efi/EFI/BOOT/grubenv create
grub-editenv /boot/efi/EFI/BOOT/grubenv set ORDER="A B" A_OK=1 A_TRY=0 B_OK=0 B_TRY=0
```

GRUB A/B 글과 같은 내용이고, 모듈에 `efi_gop`만 더했습니다. 이 모듈이 없으면 GRUB이 커널에 프레임버퍼
정보를 넘기지 못해, VirtualBox 화면에 커널 메시지가 나오지 않고 한참 검은 화면으로 있습니다.

```sh
# 5. RAUC: 슬롯, 신뢰할 CA, 부팅 성공 후 mark-good.
install -d /etc/rauc /data/rauc
sed "s/@COMPATIBLE@/$COMPATIBLE/" $F/system.conf > /etc/rauc/system.conf
install -m 0644 $F/ca.cert.pem /etc/rauc/ca.cert.pem
install -m 0644 $F/rauc-mark-good.service /etc/systemd/system/rauc-mark-good.service
systemctl enable rauc-mark-good.service

printf 'IMAGE_VERSION=%s\n' "$IMAGE_VERSION" > /etc/os-image-release
```

`system.conf`입니다.

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

- `bootloader=grub`이면 RAUC가 GRUB A/B 글의 `ORDER`, `X_OK`, `X_TRY` 변수를 `grubenv`에서 직접 고칩니다.
- `bootname`은 커널 명령줄의 `rauc.slot=A`와 맞춰 보는 값입니다. RAUC는 이것으로 지금 어느 슬롯에서
  부팅했는지 알고, 업데이트를 반대편 슬롯에 씁니다.
- `data-directory`를 데이터 파티션에 둡니다. 슬롯 상태와 설치 기록이 루트에 있으면 슬롯을 바꿀 때마다
  사라집니다.
- **`resize=true`**: 번들의 ext4 이미지는 뒤에서 최소 크기로 줄여서 만듭니다. RAUC가 슬롯에 쓴 뒤
  `resize2fs`로 파티션 크기(3GB)까지 늘립니다. 이게 없으면 새 슬롯의 루트가 거의 꽉 찬 상태로 부팅합니다.
- RAUC 소스에는 `rauc-mark-good.service`가 있지만 Debian 패키지에는 들어 있지 않아서 직접 둡니다.

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

  `boot-complete.target`은 systemd가 "부팅이 성공했다"를 표현하는 지점입니다. 헬스 체크 유닛을
  `Before=boot-complete.target`, `RequiredBy=boot-complete.target`으로 걸어 두면, 체크가 실패했을 때
  이 target에 도달하지 못하고 mark-good도 실행되지 않습니다. 그러면 grubenv의 `X_TRY=1`이 그대로 남아
  다음 부팅에서 이전 슬롯으로 돌아갑니다. [RAUC 글](/ko/posts/architecture/os/ota-update/rauc/)의 헬스
  체크가 들어갈 자리입니다.
- `/etc/os-image-release`는 지금 부팅한 슬롯이 어느 빌드인지 확인하는 용도입니다.

## 90-cleanup.sh: 빌드 VM에만 해당하는 것 지우기

설치가 끝난 VM에는 **그 VM에서만 맞는 값**이 여럿 들어 있습니다. OVA로 만든 장비 수백 대가, 그리고 나중에
RAUC로 받는 rootfs가 이 값을 그대로 물려받습니다.

| 남아 있는 것 | 문제 | 처리 |
|---|---|---|
| ifupdown 설정의 NIC 이름 (`enp0s3`) | 다른 하이퍼바이저나 실제 장비에서 이름이 달라 네트워크가 안 올라옴 | systemd-networkd로 `en*`, `eth*` 전부 DHCP |
| apt 프록시 (`/etc/apt/apt.conf`) | 장비가 CI 러너의 apt-cacher-ng를 찾음 | 줄 삭제 |
| `/etc/machine-id` | 모든 장비가 같은 ID. DHCP 클라이언트 ID, journald 등이 겹침 | 비워 두면 첫 부팅에 생성 |
| SSH 호스트 키 | 모든 장비가 같은 키. 하나가 새면 전부 위장 가능 | 삭제, 첫 부팅에 `ssh-keygen -A` |
| root 비밀번호, `PermitRootLogin yes` | 빌드용 비밀번호로 누구나 접속 | 잠그고, 공개 키만 허용 |
| 설치 로그, 셸 기록 | 빌드 환경 정보 노출, 이미지 크기 | 비움 |

```sh
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

@@CLEANUP@@

- `machine-id`는 지우지 않고 **빈 파일로** 둡니다. systemd는 빈 `/etc/machine-id`를 보면 첫 부팅으로
  판단하고 새 ID를 만들어 그 파일에 씁니다.
- SSH 호스트 키를 만드는 유닛은 키가 없을 때만 돕니다(`ConditionPathExists=!/etc/ssh/ssh_host_ed25519_key`).
- **스크립트가 root 비밀번호를 잠그고 sshd 설정을 지워도 빌드는 계속됩니다.** Packer는 처음 맺은 SSH 연결
  하나로 나머지 단계를 진행하고, sshd 설정 변경은 다음 접속부터 적용되기 때문입니다.

## 95-export-rootfs.sh: RAUC용 이미지 꺼내기

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

- **bind 마운트**로 루트를 한 번 더 마운트하면 그 위에 겹쳐 있던 `/proc`, `/sys`, `/dev`, `/run`, `/tmp`,
  `/boot/efi`, `/data`가 빠지고 **루트 파일시스템 자체**만 보입니다. 마운트 지점 디렉터리는 빈 채로 남아서
  새 슬롯에서도 그대로 마운트됩니다.
- `mkfs.ext4 -d`는 디렉터리 내용을 채워 넣으면서 파일시스템을 만듭니다. 소유자, 권한, 확장 속성(파일
  capability 포함)도 VM 안에서 root로 복사하므로 정확합니다. 빌드 머신에서 tar를 풀어 만들면 root 권한이
  필요하고 capability가 빠지기 쉽습니다.
- 출력은 `/data`에 씁니다. `/tmp`(tmpfs)에는 공간이 부족하고, 루트에 쓰면 만드는 이미지가 자기 자신을
  포함하려 합니다.
- `resize2fs -M`으로 최소 크기까지 줄입니다. 슬롯에 쓴 뒤에는 RAUC의 `resize=true`가 다시 늘립니다.

그다음 `file` 프로비저너가 이 파일을 내려받고, 마지막 인라인 단계가 지운 뒤 `fstrim`으로 빈 블록을
돌려줍니다. 그래야 OVA에 1GB 넘는 이미지 파일의 흔적이 남지 않습니다.

## 결과

@@RESULTS@@

다음 글에서는 이 빌드를 GitLab CI에서 돌리고, `rootfs.ext4`를 RAUC 번들로 서명해 OVA와 함께 릴리스로
올립니다.
