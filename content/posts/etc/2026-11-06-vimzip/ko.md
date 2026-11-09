+++
title = "vimzip: 압축만 풀면 쓰는 vim 개발 환경"
description = "정적 링크된 tarball 하나로 어떤 x86-64 리눅스든 내 vim 환경으로 바꾼다 — root도, 패키지 매니저도, glibc 버전 걱정도 없이. 어떻게 만들었고 각 조각이 왜 거기 있는지."
tags = ["vim", "linux", "tools"]
updated = 2026-11-07
+++

제 것이 아닌 머신에서 보내는 시간이 많습니다. 갓 만든 VM, 42 클러스터의 한 자리, SSH로 들어간 고객 서버. 매번 같은 마찰이 있습니다. 시스템 vim은 낡았고, `clangd`도 없고, `apt install`은 못 하고, `pip`/`npm`은 프록시 뒤에서 지뢰밭입니다. **vimzip**은 그에 대한 제 답입니다. 홈 디렉터리에 tarball 하나 풀면 제 편집기 전체가 생깁니다.

```sh
curl -fsSL https://github.com/PfClaKr/vimzip/releases/latest/download/vimzip-linux-x86_64.tar.gz | tar -xz -C ~
~/vimzip/install.sh
vim .
```

root도, 건드린 시스템 패키지도 없습니다. `~/.local/bin/vim` 심볼릭 링크를 걸고 `EDITOR=vim`을 설정하는 게 전부입니다. 지우려면 폴더만 삭제하면 됩니다.

![vim . 은 트리와 마지막 파일, 편집기를 함께 엽니다](layout.png)

([소스](https://github.com/PfClaKr/vimzip))

## 정적 링크가 사실상 전부다

"그냥 내 vim을 복사해 오기"가 보통 실패하는 이유는 공유 라이브러리입니다. Ubuntu 24.04에서 빌드한 vim은 그 머신의 glibc·ncurses에 링크되어 있어서, CentOS 7에 올리면 `GLIBC_2.34 not found`로 죽습니다. 그래서 vimzip은 vim을 glibc가 아니라 **musl로 정적 빌드**합니다.

```sh
PKG_CONFIG=false ./configure CC="$CC" \
  CPPFLAGS="-I$W/nc/include -I$W/nc/include/ncursesw" \
  LDFLAGS="-static -L$W/nc/lib" \
  --with-features=huge --with-tlib=ncursesw --enable-multibyte \
  --disable-gui --without-x --disable-gpm ...
```

ncurses를 먼저, 역시 정적으로(`--without-shared`) 컴파일하고, `--with-fallbacks` 목록(`xterm-256color`, `screen`, `tmux-256color`…)을 넉넉히 박아 넣어서 `/usr/share/terminfo`가 없는 곳에서도 터미널이 동작하게 합니다. 결과물은 공유 라이브러리 의존성이 전혀 없는 `vim` 바이너리 하나입니다. Alpine이든, 오래된 Debian이든, scratch 컨테이너든 똑같이 돕니다. 동반 도구도 같은 방식입니다. `ctags`, `fzf`, `ripgrep`, `shellcheck`는 호스트가 아니라 업스트림의 정적 릴리스 바이너리를, 버전을 고정해서 가져옵니다.

## 플러그인 매니저 없이 `pack/`만

Vim 8+에는 네이티브 패키지 레이아웃이 있어서 vimzip에는 vim-plug나 Lazy가 필요 없습니다. 플러그인은 `pack/vimzip/start/` 아래에 두면 자동으로 로드됩니다. 저장소에서는 git 서브모듈이고, 릴리스는 `.git`·테스트를 걷어낸 체크아웃 복사본만 담습니다. 구성은 일부러 작게, 전부 네이티브 Vimscript입니다(Node도, Python 호스트도 없음).

- **ALE** — 린팅과 LSP 클라이언트 (호스트에 `clangd`가 있으면 쓰고, 없으면 시스템 컴파일러로 폴백)
- **gutentags** + **tagbar** — 자동 ctags와 심볼 아웃라인
- **fzf.vim**, **nerdtree** (+git), **gitgutter**, **fugitive**, **lightline** (+bufferline)

여기에 사람들이 있는 줄 잊는 vim 내장 기능들: `comment`, `editorconfig`, `matchit`, `termdebug`, `osc52`.

## 내가 진짜 신경 쓴 설정들

포터블 바이너리는 절반입니다. 나머지 절반은, 이미 길들여 둔 편집기처럼 동작한다는 점입니다.

**`vim .`은 파일이 아니라 프로젝트.** 디렉터리에서 시작하면 폴더별 세션(열린 분할, 마지막으로 만진 파일)을 복원하고 트리를 엽니다. `Space p`(또는 `:Project 경로`)로 vim 안에서 다른 프로젝트로 바로 넘어갈 수 있습니다. 지금 프로젝트의 탭을 저장하고 터미널을 닫은 뒤, 다른 프로젝트의 탭을 복원합니다. 트리는 VSCode 탐색기처럼 지금 보고 있는 파일을 따라가고, `:q`는 현재 탭만 닫고 마지막 탭에서만 vim을 종료합니다. 세션을 비롯한 모든 것 — 실행 취소 기록, swap, tags — 은 `~/.cache/vimzip`에 있고, 프로젝트 안에 흩어지지 않습니다.

**자동 저장, 내가 잊으니까.** 편집 모드를 나가거나, 포커스를 잃거나, 버퍼를 바꾸면 조용히 `update`가 돌고, 일반 모드에서 연달아 고친 것은 마지막 수정 1초 뒤에 한 번에 저장합니다. 영속 실행 취소(`undofile`)와 합치면 고전 vim보다 현대 편집기에 가깝게 느껴집니다.

**SSH 너머의 클립보드.** 복사(`y`)는 OSC 52 이스케이프로 시스템 클립보드에 같이 보냅니다. 그래서 원격 머신에서 `y`를 해도 터미널이 지원하면 *로컬* 클립보드에 들어갑니다. X 포워딩도, `xclip`도 없이요.

**정의로 이동, 상황에 맞게 물러서기.** `clangd`가 있으면 진짜 LSP 점프이고, 없으면 프로젝트 tags로 폴백하고, 그것마저 없으면 *현재 파일이 include한 시스템 헤더*의 tags를 즉석에서 만들어 냅니다(`cc -M`을 훑는데, `compile_commands.json`이 있으면 빌드와 같은 include 옵션을 씀). 그래서 `malloc` 위에서 `gd`를 눌러도 libc 헤더로 들어갑니다.

**프로젝트 전체에서 자동완성.** `Ctrl+n`은 아직 열지 않은 파일까지 포함해 저장소 소스 파일의 단어도 제안합니다. ripgrep이 백그라운드에서 단어를 정렬된 파일 하나로 모으고, 자동완성할 때마다 그 파일을 grep하기 때문에 아주 큰 프로젝트에서도 메모리가 늘지 않습니다.

오른쪽에 AI 에이전트 패널(`Space a`)도 있어서 `claude`/`codex`/`gemini`/`aider`/`opencode` 중 먼저 찾은 것을 띄웁니다. [단축키 전체는 스크린샷과 함께 별도 글](/ko/posts/etc/vimzip-keymap/)로 정리했습니다.

## 작고, 큰 프로젝트에서도 빠르게

| | vimzip | VSCode + C/C++ 확장 |
|---|---|---|
| 다운로드 | 21 MB | 357 MB (215 + 142) |
| 설치 후 크기 | 74 MB | 1,033 MB (632 + 401) |
| SSH 서버에 올라가는 것 | 74 MB 전부 | VS Code Server 283 MB + 확장 |
| 서버 요구 사항 | x86-64 리눅스면 어디나 | glibc 2.28 이상 |

VSCode 쪽은 같은 소스로 빌드한 VSCodium 1.143과 cpptools 1.35의 크기입니다. 릴리스에서는 여기서 쓰지 않는 vim 튜터와 UTF-8이 아닌 맞춤법 파일도 빼고, `gzip -9`로 압축합니다.

트리, 에러 검사, git 표시를 모두 켠 채로 C 파일을 열면 약 70 ms, 메모리 약 30 MB입니다. 더 어려운 시험은 파일이 약 19만 개인 LLVM이었고, 처음 버전은 버티지 못했습니다. 고친 방법은 전부 일을 덜 하는 쪽이었습니다.

- **소스 파일만 인덱싱.** tags는 소스 언어로 거른 `rg --files` 목록으로 만들고, 파일이 3만 개를 넘으면 테스트 디렉터리는 뺍니다. LLVM의 tags가 474 MB에서 140 MB로 줄었습니다.
- **tags를 이진 탐색.** `tagcase=match`면 vim이 정렬된 tags 파일을 처음부터 훑지 않고 이진 탐색합니다. LLVM에서 `gd`가 540 ms에서 1 ms가 됐습니다.
- **항목마다 키 하나로 트리 정렬.** NERDTree는 Vimscript 비교 함수로 폴더를 정렬해서 비교 호출이 `N log N`번 일어났고, 파일 5,500개짜리 폴더를 여는 데 14초가 걸렸습니다. vimzip은 항목마다 키를 한 번 계산해 vim 내장 정렬을 쓰고, 트리 출력도 문자열 하나를 계속 늘리는 대신 리스트에 모읍니다.
- **시작을 막지 않기.** gitgutter의 첫 git 호출은 첫 화면이 그려진 뒤에 돌고, 프로젝트 단어 목록은 전부 백그라운드에서 만듭니다.

이렇게 해서 LLVM에서 `vim .`은 약 0.14초에 뜨고 메모리는 약 31 MB입니다.

## 손 안 대고 최신 유지하기

도구 버전은 `versions.env` 하나에 고정되어 있습니다. 매주 도는 GitHub Action이 업스트림 최신 태그를 확인하고, 뭔가 바뀌었으면 tarball 전체를 다시 빌드하고, 스모크 테스트(vim을 헤드리스로 띄워 플러그인 로드 확인, 도움말 태그 생성)를 돌린 뒤 PR을 엽니다. PR을 머지하면 새 릴리스가 올라갑니다. 그래서 알아서 최신으로 유지되고, 모든 릴리스는 실제로 부팅에 성공한 것입니다.

## 내가 이걸 좋아하는 이유

철학은 일부러 밋밋합니다. **파일 하나, 압축 풀기 외엔 설치 단계 없음, 폴더 둘 바깥에 상태 없음, 지우면 흔적 없음.** 모든 걸 갖춘 Neovim이 되려는 게 아닙니다. 남의 리눅스 머신을 내 머신처럼 느끼게 하는 가장 작은 물건이고, 그 머신이 2014년 것이든 어제 것이든 똑같이 동작합니다.
