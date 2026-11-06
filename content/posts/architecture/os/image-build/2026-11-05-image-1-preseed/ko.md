+++
title = "OS 이미지 빌드 1: preseed로 Debian 무인 설치하기"
description = "Debian 설치 프로그램(d-i)이 질문하는 방식과 preseed로 미리 답하는 방법: 부팅 명령줄, preseed.cfg의 각 부분, A/B 파티션을 만드는 partman 레시피, late_command, 그리고 설치가 멈췄을 때 찾아보는 곳. QEMU에서 실제로 설치해 확인."
tags = ["linux", "debian", "preseed", "packer", "ota"]
draft = true
date = 2026-11-05T13:00:00
+++

[A/B 파티션](/ko/posts/architecture/os/ota-update/ab-partitions/), [GRUB A/B](/ko/posts/unix/boot/grub-ab/),
[RAUC](/ko/posts/architecture/os/ota-update/rauc/) 글에서는 이미 설치된 장비가 업데이트를 받는 쪽을
다뤘습니다. 이 시리즈는 그 앞단, 장비에 들어갈 **이미지를 만드는 쪽**입니다.

1. **preseed** (이 글): Debian netinst를 사람 없이 설치하는 방법
2. **Packer**: VirtualBox에서 설치를 돌리고 OVA와 RAUC용 rootfs 이미지를 뽑는 템플릿
3. **GitLab CI**: 커밋마다 이미지를 빌드하고, RAUC 번들로 서명하고, 릴리스로 올리는 파이프라인

최종 산출물은 두 가지입니다. 새 장비나 VM에 그대로 올리는 **OVA**(공장 이미지)와, 이미 배포된 장비가
받는 **RAUC 번들**(업데이트)입니다. 둘 다 같은 빌드에서 나옵니다.

## d-i는 질문을 어떻게 하나

Debian 설치 프로그램(debian-installer, d-i)의 화면은 전부 **debconf 질문**입니다. 언어, 키보드, 호스트 이름,
미러, 파티션, root 비밀번호까지 각각 `소유자 질문이름 타입 값` 형태의 키가 있습니다.

```text
d-i netcfg/get_hostname string debian
d-i partman/confirm boolean true
tasksel tasksel/first multiselect ssh-server
```

**preseed**는 이 질문들의 답을 미리 적어 둔 파일입니다. d-i는 질문을 띄우기 전에 debconf 데이터베이스를
보고, 답이 이미 있으면 그 질문을 "본 것"으로 처리하고 넘어갑니다. 첫 칸이 소유자인데, 설치 프로그램
자체의 질문은 `d-i`, 설치되는 패키지의 질문은 그 패키지 이름(`tasksel`, `popularity-contest`)입니다.

질문마다 **우선순위**(low, medium, high, critical)도 있습니다. 부팅 인자 `priority=critical`을 주면 critical이
아닌 질문은 묻지 않고 기본값을 씁니다. preseed에 없는 질문이 critical이면 설치가 거기서 멈추고 기다립니다.
무인 설치가 멈추는 원인은 대부분 이것입니다.

## preseed를 넘기는 세 가지 방법

| 방법 | 쓰는 곳 |
|---|---|
| **initrd**: initrd 루트에 `preseed.cfg`를 넣음 | 설치 이미지를 직접 다시 만드는 경우 |
| **file**: ISO 안의 파일, `preseed/file=/cdrom/preseed.cfg` | 커스텀 ISO |
| **network**: `preseed/url=http://…/preseed.cfg` | PXE, Packer |

Packer는 빌드하는 동안 작은 HTTP 서버를 띄우고 부팅 명령줄에 그 주소를 넣습니다. ISO를 고치지 않고
공식 netinst ISO를 그대로 쓸 수 있어서 이 방식이 가장 흔합니다.

## 부팅 명령줄

UEFI로 netinst ISO를 부팅하면 GRUB 메뉴가 뜹니다. 메뉴 항목을 고르는 대신 `c`를 눌러 GRUB 명령줄로 들어가
커널을 직접 부팅합니다. 다음 글의 Packer가 이 키 입력을 대신 쳐 줍니다.

```text
linux /install.amd/vmlinuz auto=true priority=critical preseed/url=http://10.0.2.2:8080/preseed.cfg hostname=debian domain=
initrd /install.amd/initrd.gz
boot
```

- `auto=true`는 **질문 순서를 바꿉니다.** 원래 d-i는 언어와 키보드를 먼저 묻고 네트워크를 설정합니다.
  그런데 preseed 파일은 네트워크가 올라와야 받을 수 있습니다. `auto=true`를 주면 언어·키보드 질문을
  preseed를 받은 뒤로 미뤄서, 그 답도 preseed 파일에서 가져올 수 있게 됩니다.
- `preseed/url`, `priority`, `hostname`, `domain`은 debconf 키를 부팅 인자로 바로 준 것입니다. `hostname`은
  `netcfg/get_hostname`의 짧은 이름입니다. 커널 명령줄의 `키=값`은 전부 debconf에 들어가므로 preseed 파일의
  어떤 키든 여기에 쓸 수 있습니다.
- **호스트 이름을 명령줄에 주는 이유**: 네트워크 설정(netcfg)은 preseed 파일을 받기 *전에* 돌고, 그때 호스트
  이름을 묻습니다. 파일에만 적어 두면 이미 지나간 질문이라 소용이 없습니다. 그래서 파일에도 적고 명령줄에도
  줍니다.

## preseed.cfg

파일 전체는 70줄 정도입니다. 다음 글에서 Packer의 `templatefile()`로 `${…}` 자리를 채워서 넘깁니다.

### 로캘, 네트워크, 미러

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

- `choose_interface select auto`는 링크가 올라온 첫 인터페이스를 고릅니다. VirtualBox의 NIC 이름을 적어 둘
  필요가 없습니다.
- `netcfg/hostname`은 DHCP가 호스트 이름을 내려줘도 이 값을 쓰게 강제합니다. `get_hostname`은 질문의
  기본값일 뿐이라 DHCP 값에 밀립니다.
- `load_firmware false`: 펌웨어 파일을 찾지 못했을 때 이동식 매체에서 찾을지 묻는 질문을 막습니다.
- **미러는 고정하고 프록시만 바꿉니다.** CI에서는 러너에 apt-cacher-ng를 두고 `apt_proxy`에
  `http://10.0.2.2:3142`를 넣어 빌드마다 같은 패키지를 다시 받지 않게 합니다. 로컬에서는 빈 문자열입니다.
  미러 주소 자체를 캐시로 바꾸면 그 주소가 설치된 시스템의 `sources.list`에 남는데, 프록시는
  `/etc/apt/apt.conf`의 한 줄이라 정리 단계에서 지우기 쉽습니다.
- `cdrom/set-first false`, `disable-cdrom-entries true`: 설치가 끝난 시스템의 APT 소스에 ISO가 남지 않게 합니다.

### 계정과 시계

```text
d-i passwd/root-login boolean true
d-i passwd/make-user boolean false
d-i passwd/root-password password ${root_password}
d-i passwd/root-password-again password ${root_password}

d-i clock-setup/utc boolean true
d-i time/zone string Etc/UTC
d-i clock-setup/ntp boolean true
```

일반 사용자를 만들지 않고 root만 둡니다. 이 비밀번호는 **빌드하는 동안 Packer가 SSH로 들어오는 데만** 씁니다.
빌드 마지막 정리 단계에서 잠그고, 완성된 이미지에는 SSH 키로만 들어갈 수 있게 합니다.

:::note
`passwd/root-password-crypted`에 해시를 넣는 방법도 있습니다. 여기서는 preseed가 빌드 중에만 로컬 HTTP로
전달되고 비밀번호도 빌드가 끝나면 잠기기 때문에 평문을 씁니다. 값은 CI의 마스킹된 변수에서 옵니다.
:::

### 파티션: A/B 레이아웃

여기가 일반 설치와 가장 다른 부분입니다. [GRUB A/B 글](/ko/posts/unix/boot/grub-ab/)의 레이아웃을 그대로
만듭니다. ESP, 슬롯 A, 슬롯 B, 업데이트해도 유지되는 데이터 파티션입니다.

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

`expert_recipe`는 `이름 ::` 뒤에 파티션을 `.`으로 구분해 나열합니다. 각 파티션의 첫 줄은
`최소 우선순위 최대 파일시스템`이고 단위는 MB입니다.

- **최소·우선순위·최대**: partman은 모든 파티션에 최소 크기를 준 뒤, 남은 공간을 우선순위에 비례해 최대
  크기까지 나눕니다. 슬롯 A와 B는 셋 다 3072라서 정확히 3GB이고, 데이터 파티션은 최대가 `-1`(제한 없음)이라
  나머지를 다 가져갑니다.
- `$iflabel{ gpt } $reusemethod{ } method{ efi }`: GPT 디스크일 때 ESP를 만들고, 이미 ESP가 있으면 다시 씁니다.
  `$`로 시작하는 것은 조건이나 플래그이고 `이름{ 값 }`은 그 파티션에 대한 설정입니다.
- **슬롯 B는 `method{ keep }`** 입니다. 파티션만 만들고 포맷도 마운트도 하지 않습니다. 공장 이미지에서 B는
  비어 있고, 첫 업데이트 때 RAUC가 채웁니다. 파일시스템이 없는 파티션이 있으면 d-i가 "파티션 #3에 파일시스템이
  지정되지 않았습니다. 메뉴로 돌아갈까요?"를 묻습니다. `method_only false`가 "돌아가지 않는다"는 답입니다.
- 스왑은 만들지 않습니다. 그러면 d-i가 "스왑 없이 계속할까요"를 묻기 때문에 `no_swap false`로 답합니다.
- 확인 질문이 네 개입니다. 새 파티션 테이블 쓰기(`confirm_write_new_label`), 파티션 화면에서 "완료"
  선택(`choose_partition`), 디스크에 쓰기(`confirm`), 포맷할 파티션이 없을 때의 확인(`confirm_nooverwrite`).
  하나라도 빠지면 그 화면에서 멈춥니다.

:::warning 파티션 이름과 순서
partman 레시피에는 GPT 파티션 이름(`rootfs_A` 같은)을 붙이는 항목이 없습니다. 레시피는 파티션을 적은
순서대로 1, 2, 3, 4번에 만들기 때문에, 이름은 다음 글의 프로비저닝 단계에서 번호를 보고 `sgdisk -c`로 붙입니다. ESP 선택기가
`(hd0,gpt2)`, `(hd0,gpt3)`을 쓰므로 순서를 바꾸면 안 됩니다.
:::

### 패키지와 부트로더

```text
tasksel tasksel/first multiselect ssh-server
d-i pkgsel/include string ca-certificates
d-i pkgsel/upgrade select full-upgrade
popularity-contest popularity-contest/participate boolean false

d-i grub-installer/only_debian boolean true
d-i grub-installer/bootdev string default
d-i grub-installer/force-efi-extra-removable boolean true
```

- tasksel에서는 `ssh-server`만 고릅니다. `standard`까지 넣으면 이미지가 수백 MB 커집니다. 필요한 패키지는
  Packer 단계에서 따로 설치해서, 설치 프로그램에는 "부팅되고 SSH가 되는 최소 시스템"만 맡깁니다.
- `pkgsel/upgrade full-upgrade`: netinst ISO가 나온 뒤의 보안 업데이트까지 설치 중에 적용합니다.
- `force-efi-extra-removable`: GRUB을 `\EFI\debian\`뿐 아니라 이동식 매체 경로 `\EFI\BOOT\BOOTX64.EFI`에도
  설치합니다. 펌웨어 NVRAM의 부트 엔트리가 없어져도 부팅됩니다. OVA로 내보내면 VM의 NVRAM이 그대로
  따라간다는 보장이 없어서 중요합니다. 이 GRUB은 다음 글에서 A/B 선택기로 바뀌지만, 설치 직후 첫 부팅은
  이것으로 합니다.

### late_command

```text
d-i preseed/late_command string \
    in-target sh -c 'echo "PermitRootLogin yes" > /etc/ssh/sshd_config.d/90-packer.conf'

d-i finish-install/reboot_in_progress note
```

`late_command`는 설치가 끝나고 재부팅하기 직전에 설치 프로그램 환경에서 실행됩니다. 설치된 시스템은
`/target`에 마운트되어 있고, `in-target`을 앞에 붙이면 그 안에서 chroot로 실행합니다.

Debian의 sshd는 기본값이 `PermitRootLogin prohibit-password`라서 root는 비밀번호로 SSH에 들어올 수 없습니다.
Packer가 들어올 수 있게 이 설정만 바꾸고, 이 파일도 정리 단계에서 지웁니다. 나머지 설정은 전부 Packer
쪽에서 합니다. `late_command`는 한 줄짜리 셸이라 길어지면 읽기도 디버깅하기도 어렵기 때문입니다.

`reboot_in_progress note`는 마지막 "설치 완료" 화면을 건너뛰고 바로 재부팅하게 합니다.

## 키 이름 찾기

preseed 키는 외워서 쓰는 것이 아닙니다. 찾는 곳은 세 군데입니다.

- **예제 파일**: Debian 설치 안내서 부록에 릴리스마다 `example-preseed.txt`가 있습니다. 거의 모든 항목이
  주석과 함께 들어 있습니다.
- **수동으로 한 번 설치한 시스템**: `debconf-utils` 패키지의 `debconf-get-selections --installer`가 설치하면서
  받은 답을 전부 preseed 형식으로 출력합니다. 화면에서 고른 값을 그대로 옮길 수 있습니다.
- **`/var/log/installer/cdebconf/questions.dat`**: 설치된 시스템에 남는 질문 데이터베이스입니다.

## 설치가 멈췄을 때

무인 설치가 멈춘 화면에는 질문 하나가 떠 있습니다. 그 질문의 키를 preseed에 추가하면 됩니다.

- 설치 중에 **Alt+F4**로 가면 실시간 로그가, **Alt+F2**로 가면 셸이 있습니다. 셸에서
  `grep -A3 질문이름 /var/lib/cdebconf/questions.dat`으로 키를 확인할 수 있습니다.
- 설치가 끝난 뒤에는 `/var/log/installer/syslog`에 전체 로그가 남습니다.
- 자주 놓치는 것: 위의 partman 확인 질문 네 개, 스왑이 없다는 경고, 네트워크보다 먼저 묻는 질문(언어,
  키보드, 호스트 이름)입니다. 마지막 것은 `auto=true`와 명령줄 인자로 해결합니다.

이 글을 쓰면서 실제로 한 번 멈췄습니다. 처음 버전에는 `method_only`가 없어서, 30분쯤 지나 파티션 단계에서
이 화면으로 멈춰 있었습니다.

```text
[!!] Partition disks
No file system is specified for partition #3 of SCSI1 (0,0,0) (sda).
If you do not go back to the partitioning menu and assign a file system to this partition,
it won't be used at all.
Go back to the menu?                                    <Go Back>   <Yes>   <No>
```

질문 문구는 각 udeb 패키지의 debconf `templates` 파일에 있습니다. 설치 프로그램이 내려받은
`partman-*` udeb에서 이 문장을 찾으면 키가 나옵니다.

```bash
ar x partman-basicmethods_80_all.udeb control.tar.xz
tar xJf control.tar.xz ./templates
grep -B3 "won't be used at all" templates
# Template: partman-basicmethods/method_only
# Type: boolean
# Description: Go back to the menu?
#  No file system is specified for partition #${PARTITION} of ${DEVICE}.
```

`Type: boolean`이고 질문이 "메뉴로 돌아갈까요?"이니 답은 `false`입니다.

:::info Ubuntu는 다르다
Ubuntu도 Debian 계열이지만 Server는 20.04부터, 데스크톱은 23.04부터 설치 프로그램이 subiquity로 바뀌었고
무인 설치는 **autoinstall**(cloud-init 형식의 YAML)로 합니다. 최신 Ubuntu에서는 preseed가 동작하지 않습니다.
:::

## QEMU에서 돌려 본 결과

@@RESULTS@@

다음 글에서는 이 preseed를 Packer 템플릿에 넣고, 설치된 시스템을 A/B 레이아웃으로 바꾼 뒤 OVA와 rootfs
이미지로 내보냅니다.
