+++
title = "setting_linux: vimrc 18줄과 zsh 테마 한 줄씩 읽기"
description = "새 리눅스 머신마다 가져가는 vimrc와 bira 기반 oh-my-zsh 테마를 한 줄씩 설명합니다. 각 옵션이 하는 일, 직접 실행해 보고 발견한 함정(탭 폭, smartcase, # 주석, EUC-KR), 프롬프트 문자열이 그려지는 방식까지."
tags = ["linux", "vim", "zsh", "shell", "dotfiles"]
+++

새 리눅스 머신을 받으면 가장 먼저 옮기는 파일들을 [setting_linux](https://github.com/PfClaKr/setting_linux)
저장소에 모아 두었습니다. 들어 있는 것은 `vimrc`, `ychun.zsh-theme`, `.zshrc` 세 개입니다. 이 글은
`vimrc`와 테마를 다룹니다. 짧은 파일들이지만 옵션 하나하나가 생각보다 많은 일을 하고, 실제로
돌려 보니 의도와 다르게 동작하는 부분도 있었습니다.

예제는 Vim 9.1과 zsh 5.9, 최신 oh-my-zsh로 확인했습니다.

## vimrc

전체 파일입니다. `~/.vimrc`로 복사해서 씁니다.

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

### 옵션별 의미

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

`if has("syntax")` 블록과 `smartcase`, `mouse=a`는 데비안/우분투의 `/etc/vim/vimrc`에 주석으로 들어
있는 예시와 같은 모양입니다. 거기서 필요한 줄을 골라 온 파일이라고 보면 됩니다.

### `~/.vimrc`가 생기면 defaults.vim은 읽히지 않는다

사용자 vimrc가 **없을 때** Vim은 `$VIMRUNTIME/defaults.vim`을 읽습니다. 여기에 `incsearch`(입력하는
동안 검색), `scrolloff=5`(커서 위아래로 다섯 줄 여유), `showcmd`, `wildmenu`,
`filetype plugin indent on` 같은 설정이 들어 있습니다. `~/.vimrc`를 만드는 순간 이 파일은 더 이상
읽히지 않습니다. vimrc를 처음 만들고 나서 "검색할 때 바로바로 안 따라오네" 하고 느꼈다면 이것 때문입니다.

데비안 계열에서는 시스템 vimrc가 `debian.vim`을 불러서 `nocompatible`, `backspace=indent,eol,start`
같은 최소한은 유지됩니다. 나머지가 필요하면 vimrc 맨 위에 `source $VIMRUNTIME/defaults.vim`을 넣으면 됩니다.

### 탭 폭: `ts=4`인데 들여쓰기는 8칸

이 파일은 `tabstop`만 4로 바꾸고 `shiftwidth`는 건드리지 않습니다. `shiftwidth`의 기본값은 8입니다.
그런데 `cindent`, `>>`, `smarttab`의 <kbd>Tab</kbd>은 모두 `shiftwidth`를 기준으로 들여씁니다. 그래서
들여쓰기 한 단계가 4칸 탭 두 개가 됩니다. 이 vimrc로 C 코드를 입력해 보면 이렇게 저장됩니다(`^I`가 탭 문자).

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

화면에서는 8칸씩 들어가 보이고, 탭 폭이 8인 다른 편집기나 `cat`으로 보면 한 단계가 16칸이 됩니다.
`set shiftwidth=4`, 또는 `tabstop` 값을 따라가게 하는 `set shiftwidth=0`을 추가하면 한 단계가 탭 하나가 됩니다.

### `smartcase`는 `ignorecase`가 있어야 동작한다

`smartcase`는 `ignorecase`의 예외 규칙입니다. "대소문자를 무시하되, 검색어에 대문자가 있으면 구분한다"는
뜻이라서, `ignorecase`가 꺼져 있으면 아무 일도 하지 않습니다. 이 파일에서는 검색이 항상 대소문자를
구분합니다. 데비안 템플릿에서도 두 줄이 붙어 있는데, `smartcase`만 가져온 경우입니다. 의도한 동작을
원하면 `set ignorecase`를 같이 넣어야 합니다.

### 전역 `cindent`와 `#` 주석

`cindent`는 버퍼마다 따로 갖는 옵션이지만, vimrc에서 `set`으로 켜면 새로 여는 모든 버퍼가 그 값을
물려받습니다. 파일 형식별 들여쓰기(`filetype indent on`)도 켜져 있지 않아서 Python, 셸 스크립트, YAML도
C 규칙으로 들여쓰기됩니다. 가장 눈에 띄는 증상은 `#`으로 시작하는 줄입니다. C에서 `#`은 `#include`
같은 전처리 지시문이라 0열로 보내는 규칙이 있어서, Python 블록 안에서 주석을 입력하면 이렇게 됩니다.

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

`#`을 치는 순간 줄이 맨 앞으로 붙습니다. `smartindent`에도 같은 규칙이 있어서, `cindent`만 꺼서는
해결되지 않습니다. 두 옵션을 빼고 `filetype plugin indent on`으로 파일 형식에 맞는 들여쓰기를 쓰면
C에서는 C 규칙, Python에서는 Python 규칙이 적용됩니다.

### `fileencodings`와 한글 파일

기본값(`ucs-bom,utf-8,default,latin1`)에서는 UTF-8이 아닌 파일이 `latin1`로 열립니다. EUC-KR로 저장된
"안녕하세요"를 열면 이렇게 보입니다.

```text
fileencoding=latin1   ¾È³çÇÏ¼¼¿ä
```

이 vimrc에서는 UTF-8로 읽기에 실패하면 EUC-KR로 다시 읽어서 제대로 보이고, 저장할 때도 원래
인코딩(`fileencoding=euc-kr`)으로 씁니다. 목록 순서도 중요합니다. UTF-8은 바이트 규칙이 엄격해서
다른 인코딩의 파일을 UTF-8로 잘못 받아들이는 일이 거의 없으니, 엄격한 쪽을 먼저 시도해야 합니다.
ASCII만 있는 파일은 UTF-8로 판정됩니다.

한 가지 빈틈은 EUC-KR이 완성형 한글 2,350자만 담는다는 점입니다. 윈도우에서 만든 한글 파일은 보통
그 확장판인 CP949라서 "똠" 같은 글자가 들어 있으면 깨집니다.

```text
fileencoding=euc-kr   <8c>c방각하
```

`euc-kr` 자리에 `cp949`를 쓰면 EUC-KR 파일과 CP949 파일을 모두 제대로 읽습니다.

### `mouse=a`와 터미널 복사

마우스를 Vim이 가져가기 때문에, 터미널 자체의 드래그 선택으로 텍스트를 복사할 수 없게 됩니다. 대부분의
터미널에서는 <kbd>Shift</kbd>를 누른 채 드래그하면 Vim을 거치지 않고 터미널 선택이 됩니다. SSH로 붙은
서버에서 복사가 안 된다면 대개 이 옵션 때문입니다.

### 위 내용을 반영하면

```vim
source $VIMRUNTIME/defaults.vim   " incsearch, scrolloff, filetype plugin indent on, ...

set mouse=a
set tabstop=4 shiftwidth=0        " 들여쓰기 한 단계 = 탭 하나
set hlsearch
set number
set autoindent
set laststatus=2
set showmatch
set ignorecase smartcase
set smarttab
set fileencodings=utf-8,cp949
```

`defaults.vim`이 `syntax on`, `ruler`와 파일 형식별 들여쓰기를 켜 주므로 `cindent`, `smartindent`,
`syntax on` 블록은 빠졌습니다.

## zsh 테마: ychun.zsh-theme

oh-my-zsh의 `bira` 테마를 바탕으로 만든 테마입니다. `$ZSH_CUSTOM/themes/`(보통
`~/.oh-my-zsh/custom/themes/`)에 넣고 `.zshrc`에서 `ZSH_THEME="ychun"`으로 고르면 됩니다. 화면에는
이렇게 그려집니다.

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

두 줄짜리 프롬프트입니다. 첫 줄에 사용자, 호스트, 현재 디렉터리, git 브랜치가 오고, 둘째 줄의 `➤`
뒤에 명령을 입력합니다. 오른쪽에는 직전 명령의 종료 코드(실패했을 때만)와 시각이 나옵니다.

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

zsh는 `PROMPT`를 그릴 때마다 `%`로 시작하는 이스케이프를 바꿔 넣습니다. 이 테마에 쓰인 것들입니다.

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

`return_code`의 `%(?..%F{red}%? ↵%f)`는 "성공이면 아무것도 출력하지 않고, 실패면 빨간색으로 코드와
`↵`"라는 뜻입니다. 그래서 오른쪽의 숫자는 무언가 실패했을 때만 나타납니다. `[%*]`는 프롬프트가 그려진
시각, 즉 직전 명령이 끝난 시각이라 오래 걸린 명령이 언제 끝났는지 볼 때 쓸모가 있습니다.

### root와 SSH를 색으로 구분하기

앞부분의 두 `if`가 테마의 핵심입니다.

- **UID가 0이면** 사용자 이름과 `➤`가 빨간색이 됩니다. `sudo -i`로 root 셸에 들어갔다는 것을
  잊기 어렵게 만드는 장치입니다.
- **`SSH_CLIENT`나 `SSH2_CLIENT`가 있으면** 호스트 이름이 빨간색이 됩니다. OpenSSH는 원격 셸에
  `SSH_CLIENT`를 설정하므로, 로컬 터미널과 서버에 붙은 터미널을 호스트 이름 색으로 바로 구분할 수 있습니다.
  `SSH2_CLIENT`는 상용 SSH 구현이 쓰던 변수입니다.

`PR_USER_OP`(`%#`)도 같은 방식으로 정의되어 있지만 `PROMPT`에서는 쓰이지 않습니다. bira의 `$`/`#`
기호 대신 `➤`를 쓰도록 바꾸면서 남은 변수입니다.

### 따옴표와 `prompt_subst`

이 테마에서 가장 헷갈리는 부분은 따옴표입니다. `PROMPT`에는 정의할 때 값이 정해지는 부분과, 프롬프트를
그릴 때마다 다시 계산되는 부분이 섞여 있습니다.

```zsh
local git_branch='$(git_prompt_info)'          # 작은따옴표: 글자 그대로 저장
PROMPT="╭─${user_host} ${current_dir} \$(ruby_prompt_info) ${git_branch}
╰─$PR_PROMPT "                                 # 큰따옴표: 지금 펼쳐짐
```

- `${user_host}`, `${current_dir}`, `$PR_PROMPT`는 큰따옴표 안이라 **정의하는 순간** 값으로 바뀝니다.
  `PR_*` 변수들이 익명 함수 `() { … }` 안의 `local`인데도 문제가 없는 이유입니다. 함수가 끝나 변수가
  사라져도 `PROMPT`에는 이미 값이 들어가 있습니다. 익명 함수 덕분에 이 변수들이 셸 전역을 어지럽히지도 않습니다.
- `$(git_prompt_info)`는 작은따옴표로 저장된 문자열을 끼워 넣었고, `\$(ruby_prompt_info)`는
  백슬래시로 펼쳐지는 것을 막았습니다. 둘 다 `PROMPT` 안에 `$(…)` 글자 그대로 남습니다.
- `setopt prompt_subst`가 켜져 있으면 zsh는 프롬프트를 그릴 때마다 `PROMPT` 안의 `$(…)`를 실행합니다.
  그래서 디렉터리를 옮기거나 브랜치를 바꾸면 git 정보가 바로 따라옵니다.

실제로 저장된 `PROMPT` 값을 출력해 보면 이렇게 되어 있습니다(root로 실행한 경우).

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

### git과 ruby 정보

`git_prompt_info`와 `ruby_prompt_info`는 oh-my-zsh가 제공하는 함수이고, 테마는 앞뒤에 붙일 문자열만
`ZSH_THEME_*` 변수로 정합니다. git 저장소 안에서는 노란색 `‹main›`이 나오고, 저장소 밖에서는 빈 문자열입니다.

- **수정된 파일이 있으면** `‹main*›`처럼 `*`가 붙습니다. 이 테마는 `ZSH_THEME_GIT_PROMPT_DIRTY`를
  정하지 않아서 oh-my-zsh 기본값 `*`가 쓰입니다. bira는 이 자리에 빨간 `●`를 씁니다.
- `ruby_prompt_info`는 rvm, rbenv, chruby 중 하나가 있을 때만 루비 버전을 보여 줍니다. 대부분의 머신에서는
  빈 문자열이라 디렉터리와 `‹main›` 사이에 공백이 두 칸 생깁니다. 위 예시의 두 칸이 그것입니다.
- 최신 oh-my-zsh는 `git_prompt_info`를 비동기로 계산합니다. 큰 저장소에서 `git status`가 느려도
  프롬프트는 먼저 그려지고 브랜치 정보가 조금 늦게 채워집니다.

### bira와 달라진 점

| | bira | ychun |
|---|---|---|
| 사용자@호스트 | 굵은 초록, root면 빨강 | 흰색 사용자, 하늘색 `@`, SSH면 호스트가 빨강 |
| 입력 기호 | `$` / `#` | `➤` (root면 빨강) |
| 오른쪽 | 실패한 종료 코드 | 실패한 종료 코드 + 시각 |
| 색 지정 | `%{$fg[red]%}` … `%{$reset_color%}` | `%F{red}` … `%f` |
| 그 밖의 정보 | conda, virtualenv, hg, kube-ps1 | git, ruby만 |

색 지정 방식의 차이는 취향 이상의 의미가 있습니다. `$fg[red]`는 터미널 이스케이프 코드를 직접 넣는
방식이라 zsh가 글자 폭을 잘못 세지 않도록 `%{ … %}`로 감싸야 합니다. `%F{…}`는 zsh가 직접 처리하는
이스케이프라 감쌀 필요가 없고, `%F{208}`처럼 256색 번호도 쓸 수 있습니다.
