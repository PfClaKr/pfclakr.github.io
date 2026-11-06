+++
title = "OS 이미지 빌드 3: GitLab CI로 빌드하고 RAUC 번들로 릴리스하기"
description = "Packer 빌드를 GitLab CI 파이프라인으로: VirtualBox가 있는 셸 러너, 태그에서 정해지는 버전, 보호된 File 변수로 하는 RAUC 번들 서명, Generic Package Registry와 릴리스. 서명 키를 만들고 장비에서 번들을 설치해 슬롯이 바뀌는 것까지."
tags = ["linux", "gitlab", "ci", "packer", "rauc", "ota"]
draft = true
date = 2026-11-05T15:00:00
+++

[2편](/ko/posts/architecture/os/image-build/image-2-packer/)의 `packer build` 한 번으로 OVA와 `rootfs.ext4`가
나옵니다. 이 글에서는 그것을 GitLab CI에 올립니다. 목표는 이렇습니다.

- 머지 요청마다 템플릿이 올바른지 검사합니다.
- 기본 브랜치에 들어간 커밋은 실제로 빌드하고 번들까지 만들어 봅니다.
- **태그를 달면** 그 버전으로 OVA와 서명된 RAUC 번들을 만들어 릴리스로 올립니다.

```text
validate ──► build ──► bundle ──► release
 (MR, main)   (main, 태그)          (태그만)
 docker       shell+VirtualBox      docker
```

## 러너: 왜 셸 executor인가

`validate`, `bundle`, `release`는 평범한 컨테이너 잡입니다. **`build`만 다릅니다.** VirtualBox는 호스트
커널 모듈(`vboxdrv`)로 동작해서 Docker 컨테이너 안에서는 돌리기 어렵습니다. 그래서 VirtualBox와 Packer를
설치한 머신에 **셸 executor**로 러너를 하나 두고 `virtualbox` 태그를 붙입니다.

러너 머신에서 해 둘 것:

- VirtualBox, Packer, zstd 설치. `gitlab-runner` 사용자를 `vboxusers` 그룹에 넣습니다.
- `/var/cache/packer` 디렉터리를 만들고 `gitlab-runner`에게 소유권을 줍니다. ISO 캐시입니다.
- apt-cacher-ng 설치. 기본 포트 3142에서 돕니다. 1편의 preseed와 2편의 `apt_proxy` 변수가 이것을 씁니다.

:::note 셸 러너와 작업 디렉터리
셸 executor는 잡마다 깨끗한 컨테이너를 주지 않습니다. 이전 잡의 `output/`이 남아 있으면 Packer가
"output directory already exists"로 멈춥니다. GitLab은 기본적으로 잡을 시작할 때 `git clean`으로 추적하지 않는
파일을 지우지만, `.gitignore`에 `output/`이 있어도 지워지도록 `GIT_CLEAN_FLAGS`의 기본값(`-ffdx`)을 바꾸지
않는 것이 좋습니다.
:::

## 버전과 실행 조건

```yaml
stages: [validate, build, bundle, release]

variables:
  COMPATIBLE: pfclakr-debian-amd64
  IMAGE_VERSION: 0.0.0-$CI_COMMIT_SHORT_SHA
  OUT: output/debian-ab-$IMAGE_VERSION

workflow:
  rules:
    - if: $CI_COMMIT_TAG
      variables:
        IMAGE_VERSION: $CI_COMMIT_TAG
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

- `workflow:rules`는 파이프라인을 만들지 말지를 정합니다. 태그, 머지 요청, 기본 브랜치만 파이프라인이
  생기고, 기능 브랜치에 푸시만 한 것은 MR이 열리기 전까지 돌지 않습니다.
- **버전은 태그에서 옵니다.** `1.2.0` 태그를 달면 `IMAGE_VERSION`이 `1.2.0`이 되고, 그 외에는
  `0.0.0-<커밋>`입니다. 이 값이 Packer의 VM 이름, OVA의 `--version`, `/etc/os-image-release`, RAUC
  매니페스트의 `version`까지 전부 같은 값으로 들어갑니다.
- `COMPATIBLE`도 여기 한 곳에만 둡니다. 이미지 안의 `system.conf`와 번들 매니페스트가 같은 값을 써야
  RAUC가 설치를 허락하는데, 두 곳에 따로 적으면 언젠가 어긋납니다.

## validate

```yaml
validate:
  stage: validate
  image:
    name: hashicorp/packer:1.16.2
    entrypoint: [""]
  script:
    - packer init .
    - packer fmt -check -diff .
    - packer validate -var root_password=unused .
```

- 공식 이미지는 엔트리포인트가 `packer`라서 GitLab이 스크립트를 실행할 수 있게 비워 줍니다.
- `fmt -check`는 포맷이 다르면 실패하고 `-diff`로 무엇이 다른지 보여 줍니다.
- `validate`는 필수 변수가 비어 있으면 실패합니다. 비밀번호는 검사에 아무 의미가 없으니 아무 값이나
  줍니다. 이 단계에 진짜 비밀값을 줄 필요가 없게 하는 것이 요점입니다.

## build

```yaml
build:
  stage: build
  tags: [virtualbox]
  resource_group: virtualbox
  rules:
    - if: $CI_COMMIT_TAG || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  variables:
    PACKER_CACHE_DIR: /var/cache/packer
    PKR_VAR_image_version: $IMAGE_VERSION
    PKR_VAR_compatible: $COMPATIBLE
    PKR_VAR_apt_proxy: http://10.0.2.2:3142
  script:
    - packer init .
    - packer build -color=false -timestamp-ui .
    - zstd -T0 --rm "$OUT/rootfs.ext4"
  after_script:
    - VBoxManage unregistervm "debian-ab-$IMAGE_VERSION" --delete 2>/dev/null || true
  artifacts:
    paths:
      - $OUT/
    expire_in: 1 week
```

- **`resource_group`**: 같은 이름의 리소스 그룹을 가진 잡은 동시에 하나만 돕니다. 러너의 `concurrent`를
  늘려도 VM 빌드끼리 CPU와 메모리를 다투지 않습니다.
- **`PKR_VAR_*`**: Packer가 자동으로 읽는 변수입니다. `PKR_VAR_root_password`는 여기 적지 않고 프로젝트의
  **CI/CD 변수**에 마스킹(Masked)해서 넣습니다. 2편에서 `sensitive = true`로 선언했으니 Packer 로그에도
  나오지 않습니다.
- `PACKER_CACHE_DIR`: 기본값은 작업 디렉터리 안의 `packer_cache`라 `git clean`에 지워집니다. 러너의 고정
  디렉터리로 옮기면 ISO를 한 번만 받습니다.
- `-color=false`는 잡 로그에 ANSI 코드가 섞이지 않게, `-timestamp-ui`는 줄마다 시각을 붙여서 어느 단계가
  오래 걸리는지 보이게 합니다.
- **zstd 압축**: `rootfs.ext4`는 1GB가 넘습니다. GitLab 아티팩트에는 최대 크기 제한이 있고(인스턴스
  설정), 아티팩트는 다음 잡에 전달될 때 업로드·다운로드되므로 작을수록 빠릅니다. ext4 이미지는 빈 공간이 많아
  잘 줄어듭니다.
- **`after_script`**: 잡이 취소되거나 러너가 중간에 끊기면 Packer가 VM을 지우지 못하고 VirtualBox에 남습니다.
  다음 빌드가 같은 이름의 VM을 만들다 실패하지 않도록, 성공·실패와 관계없이 도는 `after_script`에서
  지웁니다.

## 서명 키

RAUC 번들은 X.509 인증서로 서명합니다. CA 하나와 그 CA가 발급한 서명용 인증서를 만듭니다.

```bash
# CA: 장비의 /etc/rauc/ca.cert.pem에 들어간다. 키는 오프라인 보관.
openssl req -x509 -newkey rsa:4096 -nodes -days 3650 \
  -keyout ca.key.pem -out ca.cert.pem -subj "/O=pfclakr/CN=pfclakr os-image CA"

# 서명용: CI가 번들을 서명할 때 쓴다.
openssl req -newkey rsa:4096 -nodes \
  -keyout signing.key.pem -out signing.csr -subj "/O=pfclakr/CN=pfclakr os-image signing"
openssl x509 -req -in signing.csr -CA ca.cert.pem -CAkey ca.key.pem -CAcreateserial \
  -days 1825 -out signing.cert.pem
```

- `ca.cert.pem`(공개)은 저장소의 `files/`에 넣고 이미지에 구워집니다.
- `signing.cert.pem`과 `signing.key.pem`은 프로젝트의 CI/CD 변수로 넣습니다. **타입을 File로**, **Protected**를
  켭니다. File 타입이면 잡 안에서 변수 값이 내용이 아니라 내용을 담은 임시 파일의 경로가 되어 `--key=`에
  그대로 줄 수 있습니다. Protected는 보호된 브랜치와 태그의 파이프라인에만 변수를 줍니다. MR 브랜치에서
  `.gitlab-ci.yml`을 고쳐 키를 출력하는 것을 막는 장치입니다. 태그도 보호(Protected tags)해 둬야 합니다.
- **CA 키는 CI에 넣지 않습니다.** 서명 키가 새면 CA로 그 인증서를 폐기하고 새 서명 인증서를 발급하면 되지만,
  CA 키가 새면 모든 장비의 키링을 바꿔야 합니다. RAUC 키링 파일에는 인증서를 여러 개 넣을 수 있어서, CA를
  교체할 때는 새 CA를 먼저 업데이트로 배포해 두는 방식으로 넘어갑니다.

## bundle

```yaml
bundle:
  stage: bundle
  image: debian:trixie
  needs: [build]
  rules:
    - if: $CI_COMMIT_TAG || $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  before_script:
    - apt-get update -q
    - apt-get install -y -q --no-install-recommends rauc squashfs-tools zstd
  script:
    - mkdir bundle
    - zstd -d "$OUT/rootfs.ext4.zst" -o bundle/rootfs.ext4
    - sed -e "s/@COMPATIBLE@/$COMPATIBLE/" -e "s/@VERSION@/$IMAGE_VERSION/"
        rauc/manifest.raucm.in > bundle/manifest.raucm
    - rauc bundle --cert="$RAUC_CERT" --key="$RAUC_KEY" bundle "$OUT/debian-ab-$IMAGE_VERSION.raucb"
    - rauc info --keyring=files/ca.cert.pem "$OUT/debian-ab-$IMAGE_VERSION.raucb"
  artifacts:
    paths:
      - $OUT/*.raucb
    expire_in: 1 week
```

매니페스트 템플릿입니다.

```ini
[update]
compatible=@COMPATIBLE@
version=@VERSION@

[bundle]
format=verity

[image.rootfs]
filename=rootfs.ext4
```

- `needs: [build]`는 build 잡의 아티팩트를 받아 옵니다.
- `rauc bundle`은 디렉터리를 squashfs로 묶어 서명하므로 `squashfs-tools`(`mksquashfs`)가 필요합니다.
- `format=verity`: 번들에 dm-verity 해시 트리를 넣습니다. 장비가 번들을 통째로 내려받지 않고 HTTP로 필요한
  부분만 읽으면서(스트리밍) 설치해도 블록마다 검증됩니다. 2편의 `system.conf`에 `bundle-formats=-plain`을 둔
  것과 짝입니다.
- 마지막 `rauc info`는 저장소의 CA로 **방금 만든 번들의 서명을 다시 확인**합니다. CI 변수에 다른 CA로 발급한
  인증서를 넣는 실수가 있으면 장비가 아니라 여기서 실패합니다.

## release

```yaml
release:
  stage: release
  image: alpine:3.22
  needs: [build, bundle]
  rules:
    - if: $CI_COMMIT_TAG
  variables:
    PKG: $CI_API_V4_URL/projects/$CI_PROJECT_ID/packages/generic/debian-ab/$CI_COMMIT_TAG
  script:
    - apk add --no-cache curl jq
    - cd "$OUT"
    - sha256sum ./*.ova ./*.raucb > SHA256SUMS
    - |
      for f in *.ova *.raucb SHA256SUMS; do
        curl --fail-with-body -H "JOB-TOKEN: $CI_JOB_TOKEN" --upload-file "$f" "$PKG/$f"
      done
    - |
      jq -n --arg tag "$CI_COMMIT_TAG" --arg pkg "$PKG" '{
        tag_name: $tag, name: $tag,
        assets: { links: [
          { name: "OVA",         url: "\($pkg)/debian-ab-\($tag).ova",   link_type: "image" },
          { name: "RAUC bundle", url: "\($pkg)/debian-ab-\($tag).raucb", link_type: "package" },
          { name: "SHA256SUMS",  url: "\($pkg)/SHA256SUMS",              link_type: "other" }
        ] } }' |
      curl --fail-with-body -H "JOB-TOKEN: $CI_JOB_TOKEN" -H "Content-Type: application/json" \
        --data @- "$CI_API_V4_URL/projects/$CI_PROJECT_ID/releases"
```

- **아티팩트는 보관용이 아닙니다.** `expire_in`이 지나면 사라지고, 크기 제한도 작습니다. 릴리스 파일은
  **Generic Package Registry**에 올립니다. `패키지이름/버전/파일이름` 경로로 PUT하면 되고, 잡 토큰
  (`CI_JOB_TOKEN`)으로 인증합니다.
- 그다음 Releases API로 태그에 릴리스를 만들고, 레지스트리의 파일을 링크로 붙입니다. 프로젝트의
  Releases 페이지에 OVA, 번들, 체크섬이 함께 보입니다.
- `--fail-with-body`: HTTP 오류면 curl이 실패 코드로 끝나고 서버의 오류 메시지를 출력합니다. 그냥
  `curl`은 404나 413에도 0으로 끝나서 잡이 성공한 것처럼 보입니다.

## 장비에서

OVA를 VirtualBox로 가져와 켜면 슬롯 A로 부팅합니다. 새 릴리스의 번들을 받아 설치하면 RAUC가 슬롯 B에 쓰고
grubenv를 바꿉니다.

```bash
rauc install debian-ab-1.1.0.raucb
reboot
# 다시 접속해서
rauc status
cat /etc/os-image-release
```

@@DEVICE@@

## 정리

세 편을 거쳐 만든 흐름입니다.

1. **preseed**가 netinst를 사람 없이 A/B 파티션 구조로 설치합니다.
2. **Packer**가 그 설치를 VM에서 돌리고, A/B 부팅과 RAUC를 넣고, 빌드 VM에만 맞는 값을 지운 뒤 OVA와
   rootfs 이미지를 함께 꺼냅니다.
3. **GitLab CI**가 태그 하나로 그 빌드를 돌리고, rootfs를 서명된 RAUC 번들로 만들어 OVA와 함께 릴리스합니다.

공장에서는 OVA를 굽고, 현장의 장비는 같은 빌드에서 나온 번들을 받습니다. 두 산출물이 한 빌드에서 나오니
"공장 이미지 1.1"과 "업데이트 1.1"의 내용이 다를 일이 없습니다.
