+++
title = "setting_linux: setting up a new Linux machine, a vimrc and a bash prompt"
description = "The checklist I go through on a fresh Linux machine, and the setting_linux repository I bring along: an install.sh that applies everything at once, each vimrc setting and why it is there, how the bash prompt works without oh-my-zsh, and the theme for when you use zsh. All checked by actually running it."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

The files I copy first onto every new Linux machine live in the
[setting_linux](https://github.com/PfClaKr/setting_linux) repository. At its core are a vimrc and a bash prompt,
applied by a single install script. Needing nothing but bash, the same environment works on servers and containers,
and a zsh theme with the same look is there for machines that use oh-my-zsh.

```text
setting_linux/
├── install.sh          applies the vimrc + prompt in one go
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    the bash prompt
├── extras.bash         history settings, aliases
├── ychun.zsh-theme     theme for oh-my-zsh (installed with --zsh)
└── .zshrc
```

This post goes through the new-machine checklist, the install script, and then the repository's files in turn
(vimrc, bash prompt, `extras.bash`, zsh theme). Everything was checked on Ubuntu 24.04 with Vim 9.1, bash 5.2 and zsh 5.9.

## On a new Linux machine

This is for Debian and Ubuntu. Desktop or server, it's mostly the same; server-only steps are marked.

**1. Bring packages up to date and install the basic tools.**

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y git vim curl wget build-essential htop tmux tree unzip ripgrep fd-find jq
```

On Debian-family systems the `fd-find` binary is called `fdfind`, not `fd`.

**2. Set the hostname, time zone and locales.**

```bash
sudo hostnamectl set-hostname robot-dev        # also fix the 127.0.1.1 line in /etc/hosts
sudo timedatectl set-timezone Europe/Paris
sudo locale-gen en_US.UTF-8 ko_KR.UTF-8
sudo update-locale LANG=en_US.UTF-8
```

Messages stay in English, but generating `ko_KR.UTF-8` too avoids trouble with Korean file names and programs.

**3. Make vim the default editor.** `visudo`, `crontab -e`, `git commit` and others follow this setting.

```bash
sudo update-alternatives --config editor       # system-wide (pick vim.basic)
```

Per user it's the `EDITOR`/`VISUAL` environment variables, which `extras.bash` from the install script sets.

**4. Give git a name and some defaults.**

```bash
git config --global user.name "PfClaKr"
git config --global user.email "me@example.com"
git config --global init.defaultBranch main
git config --global push.autoSetupRemote true  # no -u needed on a new branch's first push (git 2.37+)
```

**5. Create an SSH key and add it to GitHub.**

```bash
ssh-keygen -t ed25519 -C "me@example.com"
cat ~/.ssh/id_ed25519.pub                       # paste into GitHub → Settings → SSH keys
ssh -T git@github.com                           # "Hi PfClaKr! ..." means it works
```

**6. (Server) Only accept key-based SSH logins.** Don't edit `/etc/ssh/sshd_config` directly; add a drop-in file:

```bash
sudo tee /etc/ssh/sshd_config.d/10-local.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
EOF
sudo sshd -t && sudo systemctl restart ssh
```

When an option appears several times, `sshd_config` uses the **first** value. Ubuntu cloud images sometimes ship a
`50-cloud-init.conf` with `PasswordAuthentication yes`, so the file name has to sort before it (`10-`). Before
restarting, check from another terminal that key login works. Sessions already connected aren't dropped.

**7. (Server) Turn on the firewall and automatic security updates.**

```bash
sudo ufw allow OpenSSH && sudo ufw enable
sudo apt install -y unattended-upgrades && sudo dpkg-reconfigure -plow unattended-upgrades
```

Running `ufw enable` before allowing SSH cuts off a remote session on the spot.

**8. Apply setting_linux.**

```bash
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

To keep a copy of the repository to tweak, `git clone https://github.com/PfClaKr/setting_linux.git` and run `./install.sh`.

## install.sh

The install script copies files into place and adds one block to `~/.bashrc`.

| File | Installed to |
|---|---|
| `vimrc` | `~/.vimrc` |
| `ychun.bash-theme`, `extras.bash` | `~/.config/setting_linux/`, loaded by a block at the end of `~/.bashrc` |
| `ychun.zsh-theme` (with `--zsh`) | `$ZSH_CUSTOM/themes/`, plus `ZSH_THEME="ychun"` in `~/.zshrc` |

```bash
./install.sh             # vimrc + bash prompt + extras
./install.sh --zsh       # also install the oh-my-zsh theme
./install.sh --no-vim    # leave the vimrc alone
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

The block added to `~/.bashrc`:

```bash
# >>> setting_linux >>>
for f in "${XDG_CONFIG_HOME:-$HOME/.config}"/setting_linux/{extras.bash,ychun.bash-theme}; do
  [ -f "$f" ] && . "$f"
done
unset f
# <<< setting_linux <<<
```

- **Backups:** an existing file with different content is kept with a timestamp, as `file.bak.20261105-141235`.
- **Safe to run again:** files are just copied again, and the marker lines keep `~/.bashrc` to a single block.
- **No clone needed:** run as `curl … | bash`, the script has no files next to it, so it downloads them from GitHub
  into a temporary directory, installs them and removes it. `SETTING_LINUX_RAW` points it at another location.
- **The distribution's defaults stay:** `~/.bashrc` isn't replaced, only appended to, so only what's needed is
  overridden on top of Ubuntu's default `.bashrc`.

I checked it in empty HOMEs holding only Ubuntu's default `.bashrc`: a first install, a second run, an existing
`~/.vimrc`, and `--zsh` with and without oh-my-zsh. The `curl … | bash` straight from GitHub, and the wget path used on
machines without curl, gave the same result.

## vimrc

The file `install.sh` copies to `~/.vimrc`:

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

| Setting | Does |
|---|---|
| `source $VIMRUNTIME/defaults.vim` | Loads Vim's defaults; explained below. |
| `mouse=a` | Mouse in every mode: click to move the cursor, drag for a visual selection, wheel to scroll, drag window borders. |
| `tabstop=4` | A tab character in the file is displayed 4 columns wide. |
| `shiftwidth=0` | The width of one indent level. `0` means "follow `tabstop`", so one level is one tab (4 columns). |
| `smarttab` | <kbd>Tab</kbd> at the start of a line indents by `shiftwidth`. |
| `autoindent` | A new line starts with the previous line's indent. |
| `number` | Line numbers. |
| `ruler` | The cursor's line and column; with `laststatus=2` it appears at the right of the status line. |
| `laststatus=2` | Always show the status line, even with a single window. |
| `showmatch` | Typing a closing bracket briefly jumps the cursor to the opening one. |
| `hlsearch` | Highlights every match of the last search. `:noh` clears it. |
| `ignorecase`, `smartcase` | Searches ignore case, unless the pattern contains an uppercase letter. |
| `nobackup` | No `file~` backup on write. A different option from the `.swp` swap file. |
| `fileencodings=utf-8,cp949` | The encodings tried when opening a file; explained below. |

### Why load defaults.vim explicitly

When there is **no** user vimrc, Vim loads `$VIMRUNTIME/defaults.vim`: syntax highlighting (`syntax on`), filetype
plugins and indenting (`filetype plugin indent on`), search as you type (`incsearch`), five lines of context
around the cursor (`scrolloff=5`), `showcmd`, `wildmenu`, jumping back to the last position when reopening a file,
and more. But as soon as `~/.vimrc` exists, that file is no longer loaded automatically, so the first lines load it
by hand. `filereadable` keeps minimal installs without `defaults.vim` (`vim-tiny`) from erroring out.

That is also what lets indenting follow each filetype's rules: in C files Vim's C indent plugin turns on
`cindent`, while Python and shell scripts use their own rules. Turning `cindent` on globally in a vimrc would apply
C rules to every file, and since `#` in C is a preprocessor directive like `#include` that goes to column 0, `#`
comments in Python or shell would snap to the left margin.

### Tab width

`shiftwidth` defaults to 8. With only `tabstop` set to 4, `>>` and automatic indenting would add 8 columns, two
4-column tabs, per level. Tying the two with `shiftwidth=0` makes one indent level one tab. Typing C code with this
vimrc saves this (`^I` is a tab character):

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}
```

In Python files, Vim's Python plugin switches to four spaces (`expandtab`, `shiftwidth=4`) as PEP 8 recommends.

### `fileencodings` and Korean files

With Vim's default (`ucs-bom,utf-8,default,latin1`), a file that isn't UTF-8 opens as `latin1`: an EUC-KR file
containing "안녕하세요" shows as `¾È³çÇÏ¼¼¿ä`. `utf-8,cp949` tries UTF-8 first and, if the bytes aren't valid
UTF-8, reads the file again as CP949. Saving writes it back in its original encoding.

- **Order:** UTF-8's byte rules are strict enough that files in other encodings are almost never mistaken for
  UTF-8, so the strict one goes first. ASCII-only files are detected as UTF-8.
- **CP949 rather than EUC-KR:** EUC-KR only covers 2,350 precomposed Hangul syllables. Korean files made on Windows
  are usually CP949, its superset, and read as EUC-KR a syllable like "똠" breaks into `<8c>c`. CP949 is a superset of
  EUC-KR, so EUC-KR files read fine too.

Opening an EUC-KR file and a CP949 file containing "똠방각하":

```text
fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

### `mouse=a` and copying from the terminal

Because Vim takes the mouse, the terminal's own drag-to-select doesn't copy inside it. In most terminals, holding
<kbd>Shift</kbd> while dragging bypasses Vim and selects in the terminal. If copying doesn't work on a server over
SSH, that is usually why.

## The bash prompt: ychun.bash-theme

A two-line prompt that needs neither oh-my-zsh nor zsh, only bash. Here is the screen of a real terminal (a pty)
after typing a few commands:

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

| Where | What |
|---|---|
| first line, left | user@host, current directory (home as `~`), git branch |
| first line, right | the exit code and `↵` when the previous command failed, and the time the prompt was drawn |
| second line | commands are typed after `➤` |

- **As root**, the user name and `➤` are red: hard to forget you are in a root shell from `sudo -i`.
- **In a shell reached over SSH** (`SSH_CONNECTION` or `SSH_CLIENT` set), the host name is red, so a local terminal
  and one attached to a server are easy to tell apart.
- **The git branch** is a yellow `‹main›`, or `‹main*›` with uncommitted changes. Without a branch (detached HEAD)
  it shows a tag or a short commit hash.
- The time is when the prompt was drawn, i.e. when the previous command finished: handy to see when a long command ended.
- User and host use the terminal's default colour, so they stay readable on light backgrounds.

### How it works

**Values are computed in `PROMPT_COMMAND`; `PS1` only refers to variables.** bash runs whatever is in
`PROMPT_COMMAND` right before drawing the prompt. The theme puts `__ychun_prompt` at its front:

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

The previous command's exit code `$?` is overwritten by any command at all, so it is captured on the function's
**first line**. Returning the same value at the end lets any other `PROMPT_COMMAND` that follows (one added by the
terminal, for instance) see the original `$?` too.

**The branch name never goes into `PS1` as text.** If `PS1` were rebuilt each time with the branch name pasted in,
bash would also run any `$(…)` in it when expanding `PS1` (`promptvars`). And git accepts a branch named `$(id)`:

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- branch name pasted into PS1
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme: PS1 only refers to a variable name
╭─  root@vm …/inject ‹$(id)›
```

Just cloning someone else's repository and stepping into it could run a command. With a variable name such as
`${__ychun_git}` in `PS1`, bash expands the value once and doesn't interpret what's inside.

**Colour codes are wrapped in `\[ \]`.** readline counts the prompt's characters to place the cursor and wrap
lines. Invisible colour codes not wrapped in `\[` and `\]` get counted too, and long commands or history browsing
then wrap in the wrong place and overwrite text.

**The right side is drawn by moving the cursor.** bash has no right prompt like zsh's `RPROMPT`. So before drawing
the first line, the prompt saves the cursor (`\e7`), moves to the right-hand column (`\e[colG`), prints the exit code
and time, and restores the cursor (`\e8`). All of it sits inside `\[ \]`, so readline sees it as zero width. When the
left side is long enough to collide, the right side isn't drawn at all. In a 44-column terminal the time disappears on
long paths, and input wrapping past the edge stays intact:

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

**The git information comes from three git commands.**

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

The branch name; failing that a tag; failing that a short commit hash. `GIT_OPTIONAL_LOCKS=0` stops `git status` from
taking a lock to refresh the index, so it doesn't collide with a git command running in another terminal. Outside a
repository it prints nothing.

## extras.bash

The bash settings other than the prompt:

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

| Setting | Does |
|---|---|
| `HISTSIZE`, `HISTFILESIZE` | How many commands to keep in memory and in `~/.bash_history`. Ubuntu's defaults are 1000 and 2000. |
| `HISTCONTROL=ignoreboth:erasedups` | Commands starting with a space and repeats of the previous one aren't kept, and older copies of a command are removed. A command containing a password isn't recorded if you type a space before it. |
| `HISTTIMEFORMAT` | Adds when each command ran to `history`'s output. |
| `histappend` | Appends to the history file on exit instead of overwriting it, so several terminals don't erase each other's history. |
| `cmdhist` | Keeps a multi-line command as one entry. |

Features like zsh's `zsh-autosuggestions` and `zsh-syntax-highlighting` don't exist in bash itself. If you want
them, the bash line editor ble.sh provides both.

## If you use zsh: ychun.zsh-theme

On machines with oh-my-zsh, `./install.sh --zsh` installs a zsh theme with the same look. It is based on oh-my-zsh's
`bira` theme, goes into `$ZSH_CUSTOM/themes/`, and `ZSH_THEME` in `~/.zshrc` becomes `"ychun"`.

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

The difference from the bash version is the right side: zsh has `RPROMPT`, so the exit code and time sit at the right
of the input line.

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

Every time zsh draws `PROMPT` it replaces the escapes starting with `%`:

| Escape | Meaning |
|---|---|
| `%n` | user name |
| `%M` | full host name (`%m` stops at the first `.`) |
| `%~` | current directory, with the home directory shortened to `~` |
| `%B` … `%b` | start … end bold |
| `%F{colour}` … `%f` | start a foreground colour … back to the default |
| `%?` | exit status of the previous command |
| `%(?.true.false)` | the first part if that status is 0, the second otherwise |
| `%*` | 24-hour time with seconds |

The quoting matters. `PROMPT` is defined in double quotes, so `${user_host}`, `${current_dir}` and `$PR_PROMPT` are
replaced by their values **at definition time**; that's why the `PR_*` variables can be `local` to the anonymous
function `() { … }`. `$(git_prompt_info)`, on the other hand, is spliced in from a single-quoted string and
`\$(ruby_prompt_info)` is protected by a backslash, so both stay in `PROMPT` literally. With `setopt prompt_subst`,
zsh runs those `$(…)` every time it draws the prompt.

`git_prompt_info` and `ruby_prompt_info` are oh-my-zsh functions, so this theme doesn't work without oh-my-zsh. The `*`
for changes is oh-my-zsh's default (`ZSH_THEME_GIT_PROMPT_DIRTY`), and `ruby_prompt_info` is empty without rvm, rbenv or
chruby, which leaves two spaces between the directory and `‹main›`.
