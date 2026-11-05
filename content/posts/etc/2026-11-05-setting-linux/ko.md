+++
title = "setting_linux: 새 리눅스에서 하는 일, vimrc, bash 프롬프트"
description = "새 리눅스 머신을 받으면 하는 설정 목록과, 그때 가져가는 setting_linux 저장소를 정리합니다: 한 번에 적용하는 install.sh, vimrc의 각 설정과 그 이유, oh-my-zsh 없이 동작하는 bash 프롬프트의 구조, zsh를 쓸 때의 테마까지. 모두 실제로 돌려 보며 확인했습니다."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

새 리눅스 머신을 받으면 가장 먼저 옮기는 파일들을 [setting_linux](https://github.com/PfClaKr/setting_linux)
저장소에 모아 두었습니다. 기본은 vimrc와 bash 프롬프트이고, 설치 스크립트 하나로 적용합니다. bash만 있으면 되므로
서버나 컨테이너에서도 같은 환경을 쓸 수 있고, oh-my-zsh를 쓰는 머신을 위한 같은 모양의 zsh 테마도 들어 있습니다.

```text
setting_linux/
├── install.sh          vimrc + 프롬프트를 한 번에 적용
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    bash 프롬프트
├── extras.bash         히스토리 설정, alias
├── ychun.zsh-theme     oh-my-zsh용 테마 (--zsh로 설치)
└── .zshrc
```

이 글은 새 머신에서 하는 일의 목록, 설치 스크립트, 그리고 저장소의 파일들(vimrc, bash 프롬프트, `extras.bash`,
zsh 테마)을 차례로 설명합니다. 예제는 Ubuntu 24.04, Vim 9.1, bash 5.2, zsh 5.9로 확인했습니다.

## 새 리눅스 머신에서 하는 일

데비안·우분투 기준입니다. 데스크톱이든 서버든 대부분 같고, 서버에서만 하는 것은 따로 표시했습니다.

**1. 패키지를 최신으로 올리고 기본 도구를 설치합니다.**

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y git vim curl wget build-essential htop tmux tree unzip ripgrep fd-find jq
```

데비안 계열에서 `fd-find`의 실행 파일 이름은 `fd`가 아니라 `fdfind`입니다.

**2. 호스트 이름, 시간대, 로캘을 정합니다.**

```bash
sudo hostnamectl set-hostname robot-dev        # /etc/hosts의 127.0.1.1 줄도 같이 고칩니다
sudo timedatectl set-timezone Europe/Paris
sudo locale-gen en_US.UTF-8 ko_KR.UTF-8
sudo update-locale LANG=en_US.UTF-8
```

메시지는 영어로 두되 `ko_KR.UTF-8`도 만들어 두면 한글 파일 이름이나 한국어 프로그램을 쓸 때 문제가 없습니다.

**3. 기본 에디터를 vim으로 바꿉니다.** `visudo`, `crontab -e`, `git commit` 등이 이 설정을 따릅니다.

```bash
sudo update-alternatives --config editor       # 시스템 전체 (vim.basic 선택)
```

사용자 단위로는 `EDITOR`/`VISUAL` 환경 변수이고, 아래 설치 스크립트의 `extras.bash`가 설정합니다.

**4. git 사용자 정보와 기본값을 넣습니다.**

```bash
git config --global user.name "PfClaKr"
git config --global user.email "me@example.com"
git config --global init.defaultBranch main
git config --global push.autoSetupRemote true  # 새 브랜치 첫 push에 -u가 필요 없음 (git 2.37+)
```

**5. SSH 키를 만들고 GitHub에 등록합니다.**

```bash
ssh-keygen -t ed25519 -C "me@example.com"
cat ~/.ssh/id_ed25519.pub                       # GitHub → Settings → SSH keys에 붙여 넣기
ssh -T git@github.com                           # "Hi PfClaKr! ..." 가 나오면 성공
```

**6. (서버) SSH 로그인을 키로만 받습니다.** `/etc/ssh/sshd_config`를 직접 고치지 말고 드롭인 파일을 만듭니다.

```bash
sudo tee /etc/ssh/sshd_config.d/10-local.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
EOF
sudo sshd -t && sudo systemctl restart ssh
```

`sshd_config`는 같은 옵션이 여러 번 나오면 **처음 값**을 씁니다. 우분투 클라우드 이미지에는
`50-cloud-init.conf`가 `PasswordAuthentication yes`를 넣어 두기도 해서, 파일 이름을 그보다 앞서게(`10-`) 지어야
합니다. 재시작하기 전에 다른 터미널에서 키로 접속되는지 꼭 확인합니다. 이미 연결된 세션은 끊기지 않습니다.

**7. (서버) 방화벽과 자동 보안 업데이트를 켭니다.**

```bash
sudo ufw allow OpenSSH && sudo ufw enable
sudo apt install -y unattended-upgrades && sudo dpkg-reconfigure -plow unattended-upgrades
```

`ufw enable` 전에 SSH를 허용하지 않으면 원격 접속이 그대로 끊깁니다.

**8. setting_linux를 적용합니다.**

```bash
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

저장소를 받아 두고 고쳐 가며 쓰려면 `git clone https://github.com/PfClaKr/setting_linux.git` 후 `./install.sh`를 실행합니다.

## install.sh

설치 스크립트는 파일을 제자리에 복사하고 `~/.bashrc`에 한 블록을 추가합니다.

| 파일 | 설치 위치 |
|---|---|
| `vimrc` | `~/.vimrc` |
| `ychun.bash-theme`, `extras.bash` | `~/.config/setting_linux/`, `~/.bashrc` 끝의 블록에서 불러옴 |
| `ychun.zsh-theme` (`--zsh`일 때) | `$ZSH_CUSTOM/themes/`, `~/.zshrc`의 `ZSH_THEME="ychun"` |

```bash
./install.sh             # vimrc + bash 프롬프트 + extras
./install.sh --zsh       # oh-my-zsh 테마도 설치
./install.sh --no-vim    # vimrc는 건드리지 않음
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

`~/.bashrc`에 들어가는 블록은 이렇습니다.

```bash
# >>> setting_linux >>>
for f in "${XDG_CONFIG_HOME:-$HOME/.config}"/setting_linux/{extras.bash,ychun.bash-theme}; do
  [ -f "$f" ] && . "$f"
done
unset f
# <<< setting_linux <<<
```

- **백업:** 이미 있는 파일이 내용이 다르면 `파일.bak.20261105-141235`처럼 시각을 붙여 남깁니다.
- **다시 실행해도 안전합니다.** 파일만 새로 복사하고, `~/.bashrc`의 블록은 표시 줄을 보고 하나만 유지합니다.
- **클론 없이도 됩니다.** `curl … | bash`로 실행하면 스크립트 옆에 파일이 없으므로 GitHub에서 받아 임시 디렉터리에 놓고
  설치한 뒤 지웁니다. 다른 위치에서 받으려면 `SETTING_LINUX_RAW`로 주소를 바꿀 수 있습니다.
- **배포판 기본값은 그대로 둡니다.** `~/.bashrc`를 갈아엎지 않고 끝에 덧붙이기 때문에, 우분투 기본 `.bashrc`의 설정 위에
  필요한 것만 덮어씁니다.

빈 HOME에 우분투 기본 `.bashrc`만 둔 상태로 설치, 재실행, 기존 `~/.vimrc`가 있을 때, `--zsh`(oh-my-zsh가 없을 때와
있을 때)를 각각 돌려 보며 확인했습니다. GitHub에서 바로 받는 `curl … | bash`와, curl이 없는 머신에서 쓰는 wget 경로도
같은 결과였습니다.

## vimrc

`install.sh`가 `~/.vimrc`로 복사하는 파일입니다.

```vim
" ~/.vimrc -- installed by setting_linux/install.sh

" A ~/.vimrc stops Vim from loading defaults.vim on its own, so load it here:
" syntax highlighting, filetype indent, incsearch, scrolloff, showcmd, ...
if filereadable(expand('$VIMRUNTIME/defaults.vim'))
	source $VIMRUNTIME/defaults.vim
endif

set mouse=a
set tabstop=4
set shiftwidth=0          " indent by one tabstop (4), not the default 8
set smarttab
set autoindent
set number
set ruler
set laststatus=2
set showmatch
set hlsearch
set ignorecase            " smartcase only works together with ignorecase
set smartcase
set nobackup
set fileencodings=utf-8,cp949   " cp949 also covers EUC-KR files
```

| 설정 | 하는 일 |
|---|---|
| `source $VIMRUNTIME/defaults.vim` | Vim의 기본 설정을 불러옵니다. 아래에서 따로 설명합니다. |
| `mouse=a` | 모든 모드에서 마우스를 씁니다. 클릭으로 커서 이동, 드래그로 비주얼 선택, 휠 스크롤, 창 경계 드래그가 됩니다. |
| `tabstop=4` | 파일 안의 탭 문자 하나를 4칸 폭으로 보여 줍니다. |
| `shiftwidth=0` | 들여쓰기 한 단계의 폭. `0`은 `tabstop`을 따른다는 뜻이라 한 단계가 탭 하나(4칸)입니다. |
| `smarttab` | 줄 맨 앞에서 <kbd>Tab</kbd>을 누르면 `shiftwidth`만큼 들여씁니다. |
| `autoindent` | 새 줄을 시작할 때 윗줄의 들여쓰기를 그대로 가져옵니다. |
| `number` | 줄 번호를 보여 줍니다. |
| `ruler` | 커서의 줄·열 위치를 보여 줍니다. `laststatus=2`와 함께면 상태 줄 오른쪽에 나옵니다. |
| `laststatus=2` | 창이 하나뿐이어도 상태 줄을 항상 보여 줍니다. |
| `showmatch` | 닫는 괄호를 입력하면 여는 괄호로 커서가 잠깐 이동했다가 돌아옵니다. |
| `hlsearch` | 마지막 검색어와 일치하는 곳을 모두 강조합니다. 강조를 끄려면 `:noh`. |
| `ignorecase`, `smartcase` | 검색할 때 대소문자를 무시하되, 검색어에 대문자가 있으면 구분합니다. |
| `nobackup` | 저장할 때 `파일~` 백업 파일을 남기지 않습니다. 스왑 파일(`.swp`)과는 다른 옵션입니다. |
| `fileencodings=utf-8,cp949` | 파일을 열 때 시도할 인코딩 순서입니다. 아래에서 따로 설명합니다. |

### defaults.vim을 직접 불러오는 이유

사용자 vimrc가 **없을 때** Vim은 `$VIMRUNTIME/defaults.vim`을 읽습니다. 구문 강조(`syntax on`), 파일 형식별 플러그인과
들여쓰기(`filetype plugin indent on`), 입력하는 동안 검색(`incsearch`), 커서 위아래로 다섯 줄 여유(`scrolloff=5`),
`showcmd`, `wildmenu`, 파일을 다시 열면 마지막 위치로 이동하는 설정 등이 여기에 있습니다. 그런데 `~/.vimrc`가 생기는
순간 이 파일은 더 이상 자동으로 읽히지 않습니다. 그래서 첫 줄에서 직접 불러옵니다. `defaults.vim`이 없는 최소 설치
(`vim-tiny`)에서도 에러가 나지 않도록 `filereadable`로 감쌌습니다.

들여쓰기를 파일 형식별 규칙에 맡기는 것도 이 덕분입니다. C 파일에서는 Vim의 C 들여쓰기 플러그인이 `cindent`를 켜고,
Python이나 셸 스크립트에서는 그 언어의 규칙을 씁니다. `cindent`를 vimrc에서 전역으로 켜면 모든 파일이 C 규칙을 따르게
되는데, C에서 `#`은 `#include` 같은 전처리 지시문이라 0열로 보내는 규칙이 있어서 Python이나 셸의 `# 주석`까지 줄 맨
앞으로 붙어 버립니다.

### 탭 폭

`shiftwidth`의 기본값은 8입니다. `tabstop`만 4로 두면 `>>`나 자동 들여쓰기가 8칸씩, 즉 4칸 탭 두 개씩 들어갑니다.
`shiftwidth=0`으로 두 값을 묶으면 들여쓰기 한 단계가 탭 하나가 됩니다. 이 vimrc로 C 코드를 입력하면 이렇게
저장됩니다(`^I`가 탭 문자).

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}
```

Python 파일은 Vim의 Python 플러그인이 PEP 8에 맞춰 공백 4칸(`expandtab`, `shiftwidth=4`)으로 바꿉니다.

### `fileencodings`와 한글 파일

Vim의 기본값(`ucs-bom,utf-8,default,latin1`)에서는 UTF-8이 아닌 파일이 `latin1`로 열립니다. EUC-KR로 저장된
"안녕하세요"는 `¾È³çÇÏ¼¼¿ä`처럼 보입니다. `utf-8,cp949`는 UTF-8로 먼저 읽어 보고, 잘못된 바이트가 있으면 CP949로 다시
읽습니다. 저장할 때는 원래 인코딩으로 씁니다.

- **순서:** UTF-8은 바이트 규칙이 엄격해서 다른 인코딩의 파일을 UTF-8로 잘못 받아들이는 일이 거의 없습니다. 엄격한
  쪽을 먼저 둬야 합니다. ASCII만 있는 파일은 UTF-8로 판정됩니다.
- **EUC-KR이 아니라 CP949인 이유:** EUC-KR은 완성형 한글 2,350자만 담습니다. 윈도우에서 만든 한글 파일은 보통 그
  확장판인 CP949라서, EUC-KR로 읽으면 "똠" 같은 글자가 `<8c>c`처럼 깨집니다. CP949는 EUC-KR의 상위 집합이라 EUC-KR
  파일도 그대로 읽습니다.

EUC-KR 파일과 "똠방각하"가 든 CP949 파일을 열어 본 결과입니다.

```text
fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

### `mouse=a`와 터미널 복사

마우스를 Vim이 가져가기 때문에 터미널 자체의 드래그 선택으로는 복사가 되지 않습니다. 대부분의 터미널에서는
<kbd>Shift</kbd>를 누른 채 드래그하면 Vim을 거치지 않고 터미널 선택이 됩니다. SSH로 붙은 서버에서 복사가 안 된다면
대개 이 때문입니다.

## bash 프롬프트: ychun.bash-theme

oh-my-zsh도 zsh도 필요 없이 bash만으로 동작하는 두 줄짜리 프롬프트입니다. 실제 터미널(pty)에서 명령을 몇 개 입력하고
화면을 재현한 결과입니다.

```text
╭─  root@vm ~                                                 [14:13:09]
╰─➤ cd ~/.config/setting_linux
╭─  root@vm ~/.config/setting_linux                           [14:13:10]
╰─➤ false
╭─  root@vm ~/.config/setting_linux                      1 ↵  [14:13:11]
╰─➤ cd /tmp
╭─  root@vm /tmp                                              [14:13:12]
╰─➤ echo hi; (exit 127)
hi
╭─  root@vm /tmp                                       127 ↵  [14:13:12]
╰─➤ git init -q -b main demo && cd demo && touch f
╭─  root@vm /tmp/demo ‹main*›                                 [14:13:13]
╰─➤ ls  # typing here
```

| 위치 | 내용 |
|---|---|
| 첫 줄 왼쪽 | 사용자@호스트, 현재 디렉터리(홈은 `~`), git 브랜치 |
| 첫 줄 오른쪽 | 직전 명령이 실패했을 때의 종료 코드와 `↵`, 그리고 프롬프트가 그려진 시각 |
| 둘째 줄 | `➤` 뒤에 명령 입력 |

- **root**이면 사용자 이름과 `➤`가 빨간색입니다. `sudo -i`로 root 셸에 들어갔다는 것을 잊기 어렵게 합니다.
- **SSH로 접속한 셸**(`SSH_CONNECTION`, `SSH_CLIENT`가 있을 때)이면 호스트 이름이 빨간색입니다. 로컬 터미널과 서버에
  붙은 터미널을 바로 구분할 수 있습니다.
- **git 브랜치**는 노란색 `‹main›`이고, 커밋하지 않은 변경이 있으면 `‹main*›`입니다. 브랜치가 없으면(detached HEAD)
  태그나 짧은 커밋 해시를 보여 줍니다.
- 시각은 프롬프트가 그려진 시각, 즉 직전 명령이 끝난 시각이라 오래 걸린 명령이 언제 끝났는지 볼 때 쓸모가 있습니다.
- 사용자와 호스트는 터미널 기본색이라 밝은 배경에서도 잘 보입니다.

### 어떻게 동작하나

**`PROMPT_COMMAND`에서 값을 계산하고, `PS1`은 변수를 참조만 합니다.** bash는 프롬프트를 그리기 직전에
`PROMPT_COMMAND`에 든 명령을 실행합니다. 테마는 여기에 `__ychun_prompt`를 맨 앞에 끼워 넣습니다.

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

직전 명령의 종료 코드 `$?`는 다른 명령을 하나라도 실행하면 덮어써지므로 함수의 **첫 줄**에서 받아 둡니다. 끝에서 같은
값으로 `return`하는 것은 뒤에 이어지는 다른 `PROMPT_COMMAND`(터미널이 넣는 것 등)도 원래 `$?`를 보게 하려는 것입니다.

**브랜치 이름을 `PS1`에 문자열로 넣지 않습니다.** 매번 `PS1`을 새로 조립하면서 브랜치 이름을 끼워 넣으면, bash는
`PS1`을 펼칠 때 그 안의 `$(…)`까지 실행합니다(`promptvars`). git은 `$(id)` 같은 브랜치 이름을 허용합니다.

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- 브랜치 이름을 PS1에 그대로 넣은 경우
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme: PS1은 변수 이름만 참조
╭─  root@vm …/inject ‹$(id)›
```

남이 만든 저장소를 클론해서 들어가기만 해도 명령이 실행될 수 있다는 뜻입니다. `PS1`에 `${__ychun_git}`처럼 변수 이름을
두면 bash는 값을 한 번 펼칠 뿐 그 안을 다시 해석하지 않습니다.

**색 코드는 `\[ \]`로 감쌉니다.** readline은 프롬프트의 글자 수를 세서 커서 위치와 줄바꿈을 계산합니다. 화면에 보이지 않는
색 코드를 `\[`와 `\]`로 감싸지 않으면 글자 수에 포함되어, 긴 명령을 입력하거나 히스토리를 넘길 때 줄이 엉뚱한 곳에서
꺾이고 글자가 겹칩니다.

**오른쪽 표시는 커서를 옮겨서 그립니다.** bash에는 zsh의 `RPROMPT` 같은 오른쪽 프롬프트가 없습니다. 그래서 첫 줄을 그리기
전에 커서를 저장하고(`\e7`), 오른쪽 끝 열로 옮겨(`\e[열G`) 종료 코드와 시각을 출력한 뒤, 커서를 되돌립니다(`\e8`). 이 전체를
`\[ \]` 안에 넣어 readline에게는 폭이 0으로 보이게 합니다. 왼쪽 내용이 길어 겹칠 것 같으면 오른쪽을 아예 그리지 않습니다.
44열 터미널에서 돌려 보면 긴 경로에서는 시각이 사라지고, 입력이 줄을 넘어가도 깨지지 않습니다.

```text
╭─  root@vm ~                     [14:13:24]
╰─➤ cd ~/.config/setting_linux
╭─  root@vm ~/.config/setting_linux
╰─➤ false
╭─  root@vm ~/.config/setting_linux
╰─➤ cd /tmp
╭─  root@vm /tmp                  [14:13:27]
╰─➤ echo hi; (exit 127)
hi
╭─  root@vm /tmp           127 ↵  [14:13:27]
╰─➤ git init -q -b main demo && cd demo && t
ouch f
╭─  root@vm /tmp/demo ‹main*›
╰─➤ ls  # typing here
```

**git 정보는 git 명령 세 개로 구합니다.**

```bash
__ychun_git_info() {
  local ref
  ref=$(GIT_OPTIONAL_LOCKS=0 git symbolic-ref --short -q HEAD 2>/dev/null) ||
    ref=$(GIT_OPTIONAL_LOCKS=0 git describe --tags --exact-match HEAD 2>/dev/null) ||
    ref=$(GIT_OPTIONAL_LOCKS=0 git rev-parse --short HEAD 2>/dev/null) || return 0
  local dirty=
  if [ -n "$(GIT_OPTIONAL_LOCKS=0 git status --porcelain --ignore-submodules=dirty 2>/dev/null | head -n 1)" ]; then
    dirty='*'
  fi
  printf '‹%s%s› ' "$ref" "$dirty"
}
```

브랜치 이름, 없으면 태그, 그것도 없으면 짧은 커밋 해시 순서입니다. `GIT_OPTIONAL_LOCKS=0`은 `git status`가 인덱스를
갱신하려고 잠금을 잡지 않게 해서, 다른 터미널에서 돌고 있는 git 명령과 부딪히지 않게 합니다. 저장소 밖에서는 아무것도
출력하지 않습니다.

## extras.bash

프롬프트 외에 bash에서 쓰는 설정입니다.

```bash
HISTSIZE=50000
HISTFILESIZE=100000
HISTCONTROL=ignoreboth:erasedups
HISTTIMEFORMAT='%F %T  '
shopt -s histappend cmdhist

export EDITOR=vim VISUAL=vim

alias c='clear'
alias sysupdate='sudo apt-get update && sudo apt-get upgrade -y && sudo apt-get autoclean && sudo apt-get autoremove -y'
alias ll='ls -alF'
alias grep='grep --color=auto'
```

| 설정 | 하는 일 |
|---|---|
| `HISTSIZE`, `HISTFILESIZE` | 메모리와 `~/.bash_history`에 남길 명령 수. 우분투 기본값은 1000과 2000입니다. |
| `HISTCONTROL=ignoreboth:erasedups` | 공백으로 시작하는 명령과 바로 앞과 같은 명령은 남기지 않고, 예전에 같은 명령이 있으면 지웁니다. 비밀번호가 들어간 명령은 앞에 공백을 붙여 실행하면 기록되지 않습니다. |
| `HISTTIMEFORMAT` | `history` 출력에 실행 시각을 붙입니다. |
| `histappend` | 셸이 끝날 때 히스토리 파일을 덮어쓰지 않고 덧붙입니다. 터미널 여러 개가 서로의 기록을 지우지 않습니다. |
| `cmdhist` | 여러 줄짜리 명령을 한 항목으로 남깁니다. |

zsh의 `zsh-autosuggestions`, `zsh-syntax-highlighting` 같은 기능은 bash 자체에는 없습니다. 필요하면 두 기능을 모두 제공하는
bash용 라인 에디터 ble.sh가 있습니다.

## zsh를 쓴다면: ychun.zsh-theme

oh-my-zsh를 쓰는 머신에서는 `./install.sh --zsh`로 같은 모양의 zsh 테마를 설치합니다. oh-my-zsh의 `bira` 테마를 바탕으로
만든 것으로, `$ZSH_CUSTOM/themes/`에 들어가고 `~/.zshrc`의 `ZSH_THEME`이 `"ychun"`으로 바뀝니다.

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

bash 버전과의 차이는 오른쪽 표시입니다. zsh는 `RPROMPT`가 있어서 종료 코드와 시각이 입력 줄 오른쪽에 나옵니다.

```zsh
# Based on bira theme

setopt prompt_subst

() {

local PR_USER PR_USER_OP PR_PROMPT PR_HOST

# Check the UID
if [[ $UID -ne 0 ]]; then # normal user
  PR_USER='%F{white}%n%f'
  PR_USER_OP='%F{white}%#%f'
  PR_PROMPT='%f➤ %f'
else # root
  PR_USER='%F{red}%n%f'
  PR_USER_OP='%F{red}%#%f'
  PR_PROMPT='%F{red}➤ %f'
fi

# Check if we are on SSH or not
if [[ -n "$SSH_CLIENT"  ||  -n "$SSH2_CLIENT" ]]; then
  PR_HOST='%F{red}%M%f' # SSH
else
  PR_HOST='%F{white}%M%f' # no SSH
fi


local return_code="%(?..%F{red}%? ↵%f)"

local user_host="  ${PR_USER}%F{cyan}@${PR_HOST}"
local current_dir="%B%F{blue}%~%f%b"
local git_branch='$(git_prompt_info)'

PROMPT="╭─${user_host} ${current_dir} \$(ruby_prompt_info) ${git_branch}
╰─$PR_PROMPT "
RPROMPT="${return_code}  [%*]"

ZSH_THEME_GIT_PROMPT_PREFIX="%F{yellow}‹"
ZSH_THEME_GIT_PROMPT_SUFFIX="› %f"
ZSH_THEME_RUBY_PROMPT_PREFIX="%F{red}‹"
ZSH_THEME_RUBY_PROMPT_SUFFIX="›%f"

}
```

zsh는 `PROMPT`를 그릴 때마다 `%`로 시작하는 이스케이프를 바꿔 넣습니다.

| 이스케이프 | 뜻 |
|---|---|
| `%n` | 사용자 이름 |
| `%M` | 전체 호스트 이름 (`%m`은 첫 번째 `.` 앞까지만) |
| `%~` | 현재 디렉터리, 홈은 `~`로 줄여서 |
| `%B` … `%b` | 굵게 시작 … 끝 |
| `%F{색}` … `%f` | 글자색 시작 … 기본색으로 |
| `%?` | 직전 명령의 종료 코드 |
| `%(?.참.거짓)` | 종료 코드가 0이면 앞부분, 아니면 뒷부분 |
| `%*` | 초까지 나오는 24시간제 시각 |

따옴표가 중요합니다. `PROMPT`를 큰따옴표로 정의하므로 `${user_host}`, `${current_dir}`, `$PR_PROMPT`는 **정의하는 순간**
값으로 바뀝니다. `PR_*` 변수들이 익명 함수 `() { … }` 안의 `local`인데도 괜찮은 이유입니다. 반면 `$(git_prompt_info)`는
작은따옴표로 저장된 문자열을 끼워 넣었고 `\$(ruby_prompt_info)`는 백슬래시로 막아서, 둘 다 `PROMPT` 안에 글자 그대로
남습니다. `setopt prompt_subst`가 켜져 있으면 zsh가 프롬프트를 그릴 때마다 이 `$(…)`를 실행합니다.

`git_prompt_info`와 `ruby_prompt_info`는 oh-my-zsh가 제공하는 함수라, 이 테마는 oh-my-zsh 없이는 동작하지 않습니다.
변경이 있을 때의 `*`는 oh-my-zsh의 기본값(`ZSH_THEME_GIT_PROMPT_DIRTY`)이고, `ruby_prompt_info`는 rvm, rbenv, chruby가
없으면 빈 문자열이라 디렉터리와 `‹main›` 사이에 공백이 두 칸 생깁니다.
