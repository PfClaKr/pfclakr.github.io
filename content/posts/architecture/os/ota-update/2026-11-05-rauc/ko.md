+++
title = "RAUC: 서명된 번들이 부팅된 슬롯이 되기까지"
description = "RAUC가 임베디드 리눅스에서 A/B 업데이트를 구현하는 방식: system.conf에 슬롯 정의하기, 번들 만들고 서명하기, rauc install이 실제로 하는 일, U-Boot 쪽 설정, mark-good 단계."
tags = ["linux", "embedded", "ota", "rauc", "yocto"]
+++

[A/B 파티션 글](/ko/posts/architecture/os/ota-update/ab-partitions/)에서는 구조를 설명했습니다.
슬롯 두 개, 부트로더 플래그, 부팅 횟수 카운트, 새 슬롯을 good으로 표시하는 헬스 체크입니다. 그래도
누군가는 업데이트를 검증하고, 올바른 파티션에 쓰고, 알맞은 순간에 부트로더 상태를 바꿔야 합니다.
임베디드 리눅스에서 이 일에 자주 쓰이는 것이 RAUC(Robust Auto-Update Controller)입니다.
Pengutronix에서 시작한 오픈소스 업데이트 클라이언트입니다.

## RAUC가 하는 일과 하지 않는 일

RAUC는 장비 위에서 D-Bus API(`de.pengutronix.rauc.Installer`)를 가진 서비스와 명령줄 도구로
동작합니다. 맡은 일은 좁습니다.

- 업데이트 번들이 신뢰하는 키로 서명되었고 이 장비용인지 확인합니다.
- 번들 안의 이미지를 비활성 슬롯에 씁니다.
- 부트로더와 통신해서 슬롯을 전환하고, 나중에 슬롯을 good이나 bad로 표시합니다.

언제 업데이트할지 정하거나, 일정에 맞춰 다운로드하거나, 플릿을 관리하지는 않습니다. 그건 위에 있는
계층의 몫입니다. 장비에 `rauc-hawkbit-updater`를 둔 Eclipse hawkBit 같은 서버일 수도 있고, D-Bus
API를 호출하는 직접 만든 에이전트일 수도 있습니다.

## 장비 설명하기: `/etc/rauc/system.conf`

RAUC는 루트 파일시스템에 있는 파일 하나로 파티션 구성을 알게 됩니다.

```ini
[system]
compatible=acme-robot-v2
bootloader=uboot
# RAUC가 상태를 저장하는 곳 (예전 버전: statusfile=)
data-directory=/data/rauc
# 예전 "plain" 형식은 받지 않고 verity/crypt 번들만 허용
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

- `compatible`은 제품을 식별합니다. 매니페스트의 `compatible`이 다른 번들은 거부되므로, 다른 보드용
  번들을 실수로 설치할 수 없습니다.
- 슬롯 이름은 `slot.<클래스>.<인덱스>` 형식입니다. 같은 클래스의 슬롯은 서로 바꿔 쓸 수 있는 복사본입니다.
  `rootfs.0`과 `rootfs.1`이 A와 B 루트 파일시스템입니다.
- `bootname`은 부트로더가 그 슬롯을 부르는 이름(`A`, `B`)입니다. RAUC의 관점과 부트로더의
  `BOOT_ORDER`를 이어 주는 연결 고리입니다.
- 다른 슬롯은 `parent=`로 루트 슬롯을 따라가게 할 수 있습니다. 예를 들어 항상 rootfs와 함께
  업데이트되어야 하는 애플리케이션 파티션입니다.

## 번들: 배포하는 것

번들(`.raucb`)은 매니페스트와 이미지들을 담은 SquashFS 이미지에 CMS 서명(X.509 인증서)을 붙인
것입니다. 매니페스트는 안에 무엇이 들어 있는지 설명합니다.

```ini
[update]
compatible=acme-robot-v2
version=2026.11.0

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

번들을 만들고 확인하는 방법:

```bash
mkdir bundle && cp rootfs.ext4 manifest.raucm bundle/
rauc bundle --cert=release.cert.pem --key=release.key.pem bundle/ update.raucb
rauc info --keyring=ca.cert.pem update.raucb
```

써야 할 형식은 `verity`입니다. 서명은 매니페스트를 덮고, 매니페스트에는 번들 전체에 대한 dm-verity
해시 트리의 루트 해시가 들어 있습니다. 그래서 RAUC는 한 번 검증한 뒤 디스크에서 바뀌었을 수도 있는
번들을 믿는 대신, 블록을 읽을 때마다 검사할 수 있습니다. 스트리밍이 가능한 것도 이 덕분입니다.
`rauc install https://…`는 번들을 장비에 먼저 저장하지 않고 HTTP로 읽으면서 설치합니다(커널의 NBD
지원이 필요합니다).

Yocto에서는 `meta-rauc` 레이어가 `bundle` 클래스를 제공해서, 번들도 레시피 하나일 뿐입니다.

```bitbake
inherit bundle

RAUC_BUNDLE_COMPATIBLE = "acme-robot-v2"
RAUC_BUNDLE_FORMAT = "verity"
RAUC_BUNDLE_SLOTS = "rootfs"
RAUC_SLOT_rootfs = "acme-image"
RAUC_KEY_FILE = "${THISDIR}/files/release.key.pem"
RAUC_CERT_FILE = "${THISDIR}/files/release.cert.pem"
```

## `rauc install`이 실제로 하는 일

```bash
rauc install /data/update.raucb
```

1. 키링으로 번들 서명을 검증합니다(체인이 신뢰하는 CA로 이어져야 합니다).
2. 매니페스트의 `compatible`이 `system.conf`와 같은지 확인합니다.
3. 지금 부팅된 슬롯을 찾습니다. 보통은 부트로더가 커널 명령줄에 넣어 준 `rauc.slot=A`로, 아니면
   `root=` 장치를 비교해서 찾습니다. 대상은 각 클래스의 다른 쪽 슬롯입니다.
4. 쓰기 전에 부트로더에서 대상 슬롯을 bad로 표시합니다. 쓰는 도중 전원이 나가더라도 반쯤 쓰인
   슬롯으로는 절대 부팅되지 않게 하기 위해서입니다.
5. 각 이미지를 해당 슬롯에 씁니다(`ext4` 슬롯에 `ext4` 이미지라면 블록 복사). 정의해 둔 설치 훅이
   있으면 실행합니다.
6. 대상 슬롯을 active로 표시합니다. 부팅 순서 맨 앞에 두고 시도 횟수를 새로 채웁니다.

RAUC는 스스로 재부팅하지 않습니다. 언제 재부팅할지는 업데이트 에이전트가 정합니다. 결과는
`rauc status`로 볼 수 있습니다.

```bash
rauc status --detailed
# Compatible:  acme-robot-v2
# Booted from: rootfs.0 (A)
# Activated:   rootfs.1 (B)
# ...
```

## 부트로더 쪽: U-Boot

`bootloader=uboot`이면 RAUC는 리눅스에서 `fw_printenv`/`fw_setenv`(libubootenv, 설정은
`/etc/fw_env.config`)로 U-Boot 환경 변수를 읽고 씁니다. 쓰는 변수는 A/B 글의 것과 정확히 같습니다.
`BOOT_ORDER`와 `BOOT_<bootname>_LEFT`입니다. 카운트는 U-Boot 쪽 부트 스크립트가 구현합니다.

```bash
test -n "${BOOT_ORDER}" || setenv BOOT_ORDER "A B"
test -n "${BOOT_A_LEFT}" || setenv BOOT_A_LEFT 3
test -n "${BOOT_B_LEFT}" || setenv BOOT_B_LEFT 3

setenv bootargs
for BOOT_SLOT in "${BOOT_ORDER}"; do
  if test "x${bootargs}" != "x"; then
    # 이미 슬롯을 골랐음
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

`rauc.slot=` 인자가 고리를 완성합니다. 리눅스가 올라온 뒤 RAUC가 지금 어느 슬롯에서 돌고 있는지
아는 방법이 이것입니다. 다른 백엔드도 저장 위치만 다를 뿐 같은 아이디어를 따릅니다. barebox에는
`bootchooser`가 있고, GRUB은 `grubenv`, EFI는 부트 엔트리를 쓰며, `custom`은 직접 만든 스크립트를
호출합니다.

## 슬롯을 good으로 표시하기

업데이트 직후 새 슬롯에는 시도 횟수가 몇 번밖에 없습니다. 사용자 공간의 무언가가 시스템이 정상임을
확인하고 다음을 호출해야 합니다.

```bash
rauc status mark-good      # 지금 부팅된 슬롯의 시도 횟수를 다시 채움
rauc status mark-bad       # 지금 바로 포기 (다음 부팅에서 롤백)
rauc status mark-active other
```

중요한 서비스들 다음에 실행되는 systemd 유닛으로 간단히 할 수 있습니다.

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

`robot-healthcheck`가 실패하면 `mark-good`은 실행되지 않습니다. 멈춤에 대비한 하드웨어 워치독까지
있으면, 부트로더가 결국 알아서 이전 슬롯으로 돌아갑니다.

## 플릿 규모가 되면 중요해지는 것들

- 키 관리: 개발용과 릴리스용 인증서를 분리하고, 릴리스 키는 오프라인이나 HSM에 둡니다. 키링에는
  CA를 여러 개 넣을 수 있어서, 장비를 고립시키지 않고 키를 교체할 수 있습니다.
- 정상 경로만이 아니라 실패 경로를 테스트하세요. 설치 도중 전원을 뽑아 보고, 일부러 헬스 체크에
  실패하는 테스트 번들을 배포해서 장비가 옛 슬롯으로 돌아오는지 확인합니다. 가능하면 실제
  하드웨어에서 CI로 돌리세요.
- 데이터와 롤백: RAUC는 슬롯을 바꿀 뿐 데이터 파티션은 건드리지 않습니다. 마이그레이션은 옛 버전이
  다시 실행되어도 견딜 수 있어야 합니다.
- 대역폭: `verity` 번들과 스트리밍을 쓰면 adaptive 업데이트(매니페스트의
  `adaptive=block-hash-index`)로 장비에 이미 있는 블록은 재사용하고 나머지만 받아 올 수 있습니다.
  셀룰러 회선에서는 차이가 큽니다.
- 훅은 업데이트 중에 root로 실행되는 코드입니다. 작고, 여러 번 실행해도 안전하게 만들고, 업데이트의
  나머지 부분과 같은 테스트로 검증하세요.
