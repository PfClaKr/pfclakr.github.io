+++
title = "setting_linux: 새 리눅스에서 하는 일, vimrc, 그리고 zsh에서 bash로"
description = "새 리눅스 머신을 받으면 하는 설정 목록과, 그때 가져가는 setting_linux 저장소의 vimrc·프롬프트·설치 스크립트. vimrc의 함정(탭 폭, smartcase, # 주석, EUC-KR)을 고친 과정, oh-my-zsh 테마를 순수 bash로 옮기면서 달라진 점까지 실제로 돌려 보며 정리합니다."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

새 리눅스 머신을 받으면 가장 먼저 옮기는 파일들을 [setting_linux](https://github.com/PfClaKr/setting_linux)
저장소에 모아 두었습니다. 처음에는 `vimrc`와 oh-my-zsh 테마, `.zshrc`뿐이었는데, 서버나 컨테이너처럼 zsh가 없는
곳에서도 같은 환경을 쓰고 싶어서 프롬프트를 순수 bash로 옮기고 한 번에 적용하는 설치 스크립트를 추가했습니다.

```text
setting_linux/
├── install.sh          vimrc + 프롬프트를 한 번에 적용
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    bash 프롬프트
├── extras.bash         히스토리 설정, alias
├── ychun.zsh-theme     원래의 oh-my-zsh 테마 (--zsh로 설치)
└── .zshrc
```

이 글은 새 머신에서 하는 일의 목록, 설치 스크립트, vimrc, 그리고 zsh 테마와 그것을 bash로 옮긴 과정 순서로
갑니다. 예제는 Ubuntu 24.04, Vim 9.1, bash 5.2, zsh 5.9로 확인했습니다.

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
git clone git@github.com:PfClaKr/setting_linux.git && ./setting_linux/install.sh
```

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
있을 때)를 각각 돌려 보며 확인했습니다.

## vimrc

### 처음 버전

저장소의 처음 vimrc는 18줄이었습니다.

```vim
if has("syntax")
	syntax on
endif

set mouse=a
set ts=4
set hlsearch
set nobackup
set nu
set autoindent
set cindent
set laststatus=2
set showmatch
set smartcase
set smarttab
set smartindent
set ruler
set fileencodings=utf8,euc-kr
```

| 옵션 | 하는 일 |
|---|---|
| `syntax on` | 구문 강조를 켭니다. `has("syntax")`는 Vim이 구문 강조 기능을 넣고 컴파일됐는지 확인하는 것으로, 최소 빌드(`vim-tiny`)에서 에러가 나지 않게 해 줍니다. |
| `mouse=a` | 모든 모드에서 마우스를 씁니다. 클릭으로 커서 이동, 드래그로 비주얼 선택, 휠 스크롤, 창 경계 드래그가 됩니다. |
| `ts=4` | `tabstop`. 파일 안의 탭 문자 하나를 4칸 폭으로 보여 줍니다. |
| `hlsearch` | 마지막 검색어와 일치하는 곳을 모두 강조합니다. 강조를 끄려면 `:noh`. |
| `nobackup` | 저장할 때 `파일~` 백업 파일을 남기지 않습니다. 원래 기본값이 꺼짐이라 확인용에 가깝고, 스왑 파일(`.swp`)과는 다른 옵션입니다. |
| `nu` | `number`. 줄 번호를 보여 줍니다. |
| `autoindent` | 새 줄을 시작할 때 윗줄의 들여쓰기를 그대로 가져옵니다. |
| `cindent` | C 문법 규칙으로 들여씁니다. `{` 다음 줄은 한 단계 들어가고 `}`는 맞는 위치로 돌아옵니다. |
| `smartindent` | `{` 다음이나 `if`, `for` 같은 키워드 다음 줄을 들여쓰는 간단한 규칙입니다. 다만 `cindent`가 켜져 있으면 아무 효과가 없습니다. |
| `smarttab` | 줄 맨 앞에서 <kbd>Tab</kbd>을 누르면 `tabstop`이 아니라 `shiftwidth`만큼 들여씁니다. |
| `laststatus=2` | 창이 하나뿐이어도 상태 줄을 항상 보여 줍니다. |
| `ruler` | 커서의 줄·열 위치를 보여 줍니다. `laststatus=2`와 함께면 상태 줄 오른쪽에 나옵니다. |
| `showmatch` | 닫는 괄호를 입력하면 여는 괄호로 커서가 잠깐 이동했다가 돌아옵니다. |
| `smartcase` | 검색어에 대문자가 있으면 대소문자를 구분합니다. 아래에서 다루지만 이것만으로는 동작하지 않습니다. |
| `fileencodings=utf8,euc-kr` | 파일을 열 때 시도할 인코딩 순서입니다. UTF-8로 읽어 보고, 잘못된 바이트가 있으면 EUC-KR로 읽습니다. |

`if has("syntax")` 블록과 `smartcase`, `mouse=a`는 데비안/우분투의 `/etc/vim/vimrc`에 주석으로 들어 있는 예시와 같은
모양입니다. 거기서 필요한 줄을 골라 온 파일이라고 보면 됩니다. 실제로 돌려 보니 의도와 다르게 동작하는 곳이 다섯
군데 있었습니다.

### `~/.vimrc`가 생기면 defaults.vim은 읽히지 않는다

사용자 vimrc가 **없을 때** Vim은 `$VIMRUNTIME/defaults.vim`을 읽습니다. 여기에 `incsearch`(입력하는 동안 검색),
`scrolloff=5`(커서 위아래로 다섯 줄 여유), `showcmd`, `wildmenu`, `filetype plugin indent on` 같은 설정이 들어 있습니다.
`~/.vimrc`를 만드는 순간 이 파일은 더 이상 읽히지 않습니다. vimrc를 처음 만들고 나서 "검색할 때 바로바로 안 따라오네"
하고 느꼈다면 이것 때문입니다. 데비안 계열에서는 시스템 vimrc가 `debian.vim`을 불러서 `nocompatible`,
`backspace=indent,eol,start` 같은 최소한은 유지됩니다.

### 탭 폭: `ts=4`인데 들여쓰기는 8칸

`tabstop`만 4로 바꾸고 `shiftwidth`는 기본값 8 그대로였습니다. `cindent`, `>>`, `smarttab`의 <kbd>Tab</kbd>은 모두
`shiftwidth`를 기준으로 들여쓰므로 들여쓰기 한 단계가 4칸 탭 두 개가 됩니다. 이 vimrc로 C 코드를 입력하면 이렇게
저장됩니다(`^I`가 탭 문자).

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

화면에서는 8칸씩 들어가 보이고, 탭 폭이 8인 다른 편집기나 `cat`으로 보면 한 단계가 16칸이 됩니다.

### `smartcase`는 `ignorecase`가 있어야 동작한다

`smartcase`는 `ignorecase`의 예외 규칙입니다. "대소문자를 무시하되, 검색어에 대문자가 있으면 구분한다"는 뜻이라서,
`ignorecase`가 꺼져 있으면 아무 일도 하지 않습니다. 데비안 템플릿에서도 두 줄이 붙어 있는데, `smartcase`만 가져온
경우였습니다.

### 전역 `cindent`와 `#` 주석

`cindent`는 버퍼마다 따로 갖는 옵션이지만, vimrc에서 `set`으로 켜면 새로 여는 모든 버퍼가 그 값을 물려받습니다. 파일
형식별 들여쓰기도 켜져 있지 않아서 Python, 셸 스크립트, YAML도 C 규칙으로 들여쓰기됩니다. 가장 눈에 띄는 증상은 `#`으로
시작하는 줄입니다. C에서 `#`은 `#include` 같은 전처리 지시문이라 0열로 보내는 규칙이 있어서, Python 블록 안에서 주석을
입력하면 이렇게 됩니다.

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

`#`을 치는 순간 줄이 맨 앞으로 붙습니다. `smartindent`에도 같은 규칙이 있어서 `cindent`만 꺼서는 해결되지 않습니다.

### `fileencodings`와 한글 파일

기본값(`ucs-bom,utf-8,default,latin1`)에서는 UTF-8이 아닌 파일이 `latin1`로 열립니다. EUC-KR로 저장된 "안녕하세요"는
`¾È³çÇÏ¼¼¿ä`로 보입니다. 처음 vimrc는 UTF-8로 읽기에 실패하면 EUC-KR로 다시 읽어서 이 문제를 피했고, 저장할 때도
원래 인코딩으로 씁니다. UTF-8은 바이트 규칙이 엄격해서 다른 인코딩의 파일을 UTF-8로 잘못 받아들이는 일이 거의 없으니,
엄격한 쪽을 먼저 두는 순서도 맞습니다.

빈틈은 EUC-KR이 완성형 한글 2,350자만 담는다는 점입니다. 윈도우에서 만든 한글 파일은 보통 그 확장판인 CP949라서
"똠" 같은 글자가 들어 있으면 깨집니다.

```text
fileencoding=euc-kr   <8c>c방각하
```

### `mouse=a`와 터미널 복사

마우스를 Vim이 가져가기 때문에 터미널 자체의 드래그 선택으로 텍스트를 복사할 수 없게 됩니다. 대부분의 터미널에서는
<kbd>Shift</kbd>를 누른 채 드래그하면 Vim을 거치지 않고 터미널 선택이 됩니다. SSH로 붙은 서버에서 복사가 안 된다면 대개
이 옵션 때문입니다. 이것은 함정이라기보다 알고 쓰면 되는 동작이라 지금 버전에도 남겨 두었습니다.

### 지금 버전

위 내용을 반영해 저장소의 vimrc를 바꿨습니다.

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

- `defaults.vim`을 직접 불러서 `syntax on`, `incsearch`, `scrolloff`, 파일 형식별 들여쓰기를 되찾습니다. `defaults.vim`이 없는
  최소 설치(`vim-tiny`)에서도 에러가 나지 않도록 `filereadable`로 감쌌습니다.
- `shiftwidth=0`은 "`tabstop` 값을 따른다"는 뜻이라 들여쓰기 한 단계가 탭 하나가 됩니다.
- `cindent`와 `smartindent`를 빼고 파일 형식별 들여쓰기에 맡깁니다. C 파일에서는 Vim의 C 들여쓰기 플러그인이 `cindent`를
  켜 주고, Python이나 셸에서는 그 언어의 규칙을 씁니다.
- `cp949`는 EUC-KR의 상위 집합이라 EUC-KR 파일도 그대로 읽습니다.

같은 테스트를 다시 돌린 결과입니다. C는 한 단계가 탭 하나가 되었고, 셸 스크립트의 `# note`는 제자리에 남았고, 두 한글
파일이 모두 제대로 열렸습니다.

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}

fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

## zsh 테마: ychun.zsh-theme

oh-my-zsh의 `bira` 테마를 바탕으로 만든 테마입니다. `./install.sh --zsh`가 `$ZSH_CUSTOM/themes/`(보통
`~/.oh-my-zsh/custom/themes/`)에 넣고 `.zshrc`에서 `ZSH_THEME="ychun"`으로 고릅니다. 화면에는 이렇게 그려집니다.

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

두 줄짜리 프롬프트입니다. 첫 줄에 사용자, 호스트, 현재 디렉터리, git 브랜치가 오고, 둘째 줄의 `➤` 뒤에 명령을
입력합니다. 오른쪽에는 직전 명령의 종료 코드(실패했을 때만)와 시각이 나옵니다.

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

### 프롬프트 이스케이프

zsh는 `PROMPT`를 그릴 때마다 `%`로 시작하는 이스케이프를 바꿔 넣습니다.

| 이스케이프 | 뜻 |
|---|---|
| `%n` | 사용자 이름 |
| `%M` | 전체 호스트 이름 (`%m`은 첫 번째 `.` 앞까지만) |
| `%~` | 현재 디렉터리, 홈은 `~`로 줄여서 |
| `%#` | 일반 사용자면 `%`, root면 `#` |
| `%B` … `%b` | 굵게 시작 … 끝 |
| `%F{색}` … `%f` | 글자색 시작 … 기본색으로 |
| `%?` | 직전 명령의 종료 코드 |
| `%(?.참.거짓)` | 종료 코드가 0이면 앞부분, 아니면 뒷부분 |
| `%*` | 초까지 나오는 24시간제 시각 |

`return_code`의 `%(?..%F{red}%? ↵%f)`는 "성공이면 아무것도 출력하지 않고, 실패면 빨간색으로 코드와 `↵`"라는 뜻입니다.
`[%*]`는 프롬프트가 그려진 시각, 즉 직전 명령이 끝난 시각이라 오래 걸린 명령이 언제 끝났는지 볼 때 쓸모가 있습니다.

### root와 SSH를 색으로 구분하기

- **UID가 0이면** 사용자 이름과 `➤`가 빨간색이 됩니다. `sudo -i`로 root 셸에 들어갔다는 것을 잊기 어렵게 만드는
  장치입니다.
- **`SSH_CLIENT`나 `SSH2_CLIENT`가 있으면** 호스트 이름이 빨간색이 됩니다. OpenSSH는 원격 셸에 `SSH_CLIENT`를 설정하므로,
  로컬 터미널과 서버에 붙은 터미널을 호스트 이름 색으로 바로 구분할 수 있습니다. `SSH2_CLIENT`는 상용 SSH 구현이 쓰던
  변수입니다.

`PR_USER_OP`(`%#`)도 같은 방식으로 정의되어 있지만 `PROMPT`에서는 쓰이지 않습니다. bira의 `$`/`#` 기호 대신 `➤`를 쓰도록
바꾸면서 남은 변수입니다.

### 따옴표와 `prompt_subst`

`PROMPT`에는 정의할 때 값이 정해지는 부분과, 프롬프트를 그릴 때마다 다시 계산되는 부분이 섞여 있습니다.

- `${user_host}`, `${current_dir}`, `$PR_PROMPT`는 큰따옴표 안이라 **정의하는 순간** 값으로 바뀝니다. `PR_*` 변수들이 익명
  함수 `() { … }` 안의 `local`인데도 문제가 없는 이유입니다.
- `$(git_prompt_info)`는 작은따옴표로 저장된 문자열을 끼워 넣었고, `\$(ruby_prompt_info)`는 백슬래시로 펼쳐지는 것을
  막았습니다. 둘 다 `PROMPT` 안에 `$(…)` 글자 그대로 남습니다.
- `setopt prompt_subst`가 켜져 있으면 zsh는 프롬프트를 그릴 때마다 `PROMPT` 안의 `$(…)`를 실행합니다.

실제로 저장된 `PROMPT` 값을 출력해 보면 이렇게 되어 있습니다(root로 실행한 경우).

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

`git_prompt_info`와 `ruby_prompt_info`는 oh-my-zsh가 제공하는 함수입니다. 수정된 파일이 있으면 `‹main*›`처럼 `*`가
붙는데, 이 테마는 `ZSH_THEME_GIT_PROMPT_DIRTY`를 정하지 않아서 oh-my-zsh 기본값 `*`가 쓰입니다(bira는 빨간 `●`).
`ruby_prompt_info`는 rvm, rbenv, chruby가 없으면 빈 문자열이라 디렉터리와 `‹main›` 사이에 공백이 두 칸 생깁니다.

## bash로 옮기기: ychun.bash-theme

zsh 테마는 oh-my-zsh 없이는 동작하지 않습니다. `git_prompt_info`가 oh-my-zsh 함수이고, 테마 파일을 불러오는 것도
oh-my-zsh입니다. 서버, 컨테이너, 임베디드 보드에는 zsh조차 없는 경우가 많아서, 같은 모양을 bash만으로 만들었습니다.
실제 터미널(pty)에서 명령을 몇 개 입력하고 화면을 재현한 결과입니다.

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

전체 파일은 저장소의 `ychun.bash-theme`에 있고, 옮기면서 달라진 점은 다섯 가지입니다.

**1. `prompt_subst` 대신 `PROMPT_COMMAND`.** bash는 프롬프트를 그리기 직전에 `PROMPT_COMMAND`에 든 명령을 실행합니다.
여기서 git 정보, 시각, 종료 코드를 계산해 변수에 넣고, `PS1`은 그 변수를 참조만 합니다.

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

직전 명령의 종료 코드 `$?`는 다른 명령을 하나라도 실행하면 덮어써지므로 함수의 **첫 줄**에서 받아 두어야 합니다. 끝에서
같은 값으로 `return`하는 것은 뒤에 이어지는 다른 `PROMPT_COMMAND`(터미널이 넣는 것 등)도 원래 `$?`를 보게 하려는 것입니다.

**2. 브랜치 이름을 `PS1`에 직접 넣지 않습니다.** 매번 `PS1`을 새로 조립하면서 브랜치 이름을 문자열로 끼워 넣으면, bash는
`PS1`을 펼칠 때 그 안의 `$(…)`까지 실행합니다(`promptvars`). git은 `$(id)` 같은 브랜치 이름을 허용합니다.

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- 브랜치 이름을 PS1에 그대로 넣은 경우
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme: PS1은 변수 이름만 참조
╭─  root@vm …/inject ‹$(id)›
```

남이 만든 저장소를 클론해서 들어가기만 해도 명령이 실행될 수 있다는 뜻입니다. `PS1`에 `${__ychun_git}`처럼 변수 이름을
두면 bash는 변수 값을 한 번 펼칠 뿐 그 안을 다시 해석하지 않습니다. zsh 테마는 `$(git_prompt_info)`의 출력이 다시 해석되지
않아서 이 문제가 없었습니다.

**3. 색 코드는 `\[ \]`로 감쌉니다.** readline은 프롬프트의 글자 수를 세서 커서 위치와 줄바꿈을 계산합니다. 화면에 보이지
않는 색 코드를 `\[`와 `\]`로 감싸지 않으면 글자 수에 포함되어, 긴 명령을 입력하거나 히스토리를 넘길 때 줄이 엉뚱한 곳에서
꺾이고 글자가 겹칩니다. zsh의 `%F{…}`는 zsh가 폭을 알아서 계산하므로 필요 없던 일입니다.

**4. 오른쪽 프롬프트가 없습니다.** bash에는 `RPROMPT`가 없어서, 첫 줄을 그리기 전에 커서를 저장하고(`\e7`) 오른쪽
끝 열로 옮겨(`\e[열G`) 종료 코드와 시각을 출력한 뒤 커서를 되돌립니다(`\e8`). 이 전체를 `\[ \]` 안에 넣어 readline에게는 폭이
0인 것으로 보이게 합니다. 그래서 zsh와 달리 시각이 입력 줄이 아니라 첫 줄 오른쪽에 나옵니다. 왼쪽 내용이 길어 겹칠 것 같으면
오른쪽을 아예 그리지 않습니다. 44열 터미널에서 돌려 보면 긴 경로에서는 시각이 사라지고, 입력이 줄을 넘어가도 깨지지 않습니다.

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

**5. git 정보를 직접 구합니다.** oh-my-zsh의 `git_prompt_info`가 하던 일을 git 명령 세 개로 대신합니다.

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

브랜치 이름, 없으면(detached HEAD) 태그, 그것도 없으면 짧은 커밋 해시 순서입니다. `GIT_OPTIONAL_LOCKS=0`은 `git status`가
인덱스를 갱신하려고 잠금을 잡지 않게 해서, 다른 터미널에서 돌고 있는 git 명령과 부딪히지 않게 합니다. oh-my-zsh도 같은
이유로 이 변수를 씁니다.

그 밖에 zsh 쪽에서 쓰던 것들입니다.

- `.zshrc`의 alias(`c`, `sysupdate`)와 히스토리 설정은 `extras.bash`로 옮겼습니다. 히스토리는 5만 줄, 중복 제거,
  실행 시각 기록(`HISTTIMEFORMAT`), 여러 터미널이 서로 덮어쓰지 않게 `histappend`입니다.
- oh-my-zsh의 `zsh-autosuggestions`, `zsh-syntax-highlighting` 같은 플러그인은 bash 자체로는 대신할 수 없습니다. 꼭 필요하면
  두 기능을 모두 제공하는 bash용 라인 에디터 ble.sh가 있습니다.
- 사용자 이름과 호스트는 zsh 테마의 흰색(`%F{white}`) 대신 터미널 기본색으로 두었습니다. 밝은 배경의 터미널에서 흰 글자가
  안 보이는 문제를 피하려는 것입니다.
