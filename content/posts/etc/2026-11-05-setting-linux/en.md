+++
title = "setting_linux: setting up a new Linux machine, a vimrc, and moving from zsh to bash"
description = "The checklist I go through on a fresh Linux machine, and the setting_linux repository I bring along: a vimrc, a prompt and an install script. How the vimrc's traps (tab width, smartcase, # comments, EUC-KR) were fixed, and what changed when the oh-my-zsh theme was ported to plain bash, all checked by actually running it."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

The files I copy first onto every new Linux machine live in the
[setting_linux](https://github.com/PfClaKr/setting_linux) repository. It started as a `vimrc`, an oh-my-zsh theme
and a `.zshrc`; wanting the same environment on servers and containers that have no zsh, I ported the prompt to
plain bash and added a script that applies everything at once.

```text
setting_linux/
├── install.sh          applies the vimrc + prompt in one go
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    the bash prompt
├── extras.bash         history settings, aliases
├── ychun.zsh-theme     the original oh-my-zsh theme (installed with --zsh)
└── .zshrc
```

This post goes through the new-machine checklist, the install script, the vimrc, and then the zsh theme and its
bash port. Everything was checked on Ubuntu 24.04 with Vim 9.1, bash 5.2 and zsh 5.9.

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
git clone git@github.com:PfClaKr/setting_linux.git && ./setting_linux/install.sh
```

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
`~/.vimrc`, and `--zsh` with and without oh-my-zsh.

## vimrc

### The first version

The repository's original vimrc was 18 lines:

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

| Option | Does |
|---|---|
| `syntax on` | Turns on syntax highlighting. `has("syntax")` checks that Vim was compiled with that feature, so minimal builds (`vim-tiny`) don't error out. |
| `mouse=a` | Mouse in every mode: click to move the cursor, drag for a visual selection, wheel to scroll, drag window borders. |
| `ts=4` | `tabstop`: a tab character in the file is displayed 4 columns wide. |
| `hlsearch` | Highlights every match of the last search. `:noh` clears it. |
| `nobackup` | No `file~` backup on write. This is already the default, and it is a different option from the `.swp` swap file. |
| `nu` | `number`: line numbers. |
| `autoindent` | A new line starts with the previous line's indent. |
| `cindent` | Indents by C rules: one level after `{`, back to the matching column on `}`. |
| `smartindent` | A simpler rule that indents after `{` or keywords like `if` and `for`. It has no effect while `cindent` is on. |
| `smarttab` | <kbd>Tab</kbd> at the start of a line indents by `shiftwidth` instead of `tabstop`. |
| `laststatus=2` | Always show the status line, even with a single window. |
| `ruler` | Show the cursor's line and column; with `laststatus=2` it appears at the right of the status line. |
| `showmatch` | Typing a closing bracket briefly jumps the cursor to the opening one. |
| `smartcase` | Case-sensitive search when the pattern has an uppercase letter. As shown below, it does nothing on its own. |
| `fileencodings=utf8,euc-kr` | The encodings tried when opening a file: UTF-8 first, EUC-KR if the bytes aren't valid UTF-8. |

The `if has("syntax")` block, `smartcase` and `mouse=a` have the same shape as the commented examples in
Debian/Ubuntu's `/etc/vim/vimrc`; the file is essentially a pick of lines from there. Actually running it showed
five places where it didn't do what was intended.

### A `~/.vimrc` turns off defaults.vim

When there is **no** user vimrc, Vim loads `$VIMRUNTIME/defaults.vim`, which sets `incsearch` (search as you type),
`scrolloff=5` (five lines of context around the cursor), `showcmd`, `wildmenu`, `filetype plugin indent on` and
more. The moment `~/.vimrc` exists, that file is no longer loaded. If search stopped following your typing after
you created your first vimrc, this is why. On Debian-based systems the system vimrc still loads `debian.vim`, which
keeps the basics such as `nocompatible` and `backspace=indent,eol,start`.

### Tab width: `ts=4`, but indents are 8 columns

Only `tabstop` was changed to 4; `shiftwidth` stayed at its default of 8. `cindent`, `>>` and `smarttab`'s
<kbd>Tab</kbd> all indent by `shiftwidth`, so one indent level became two 4-column tabs. Typing C code with this
vimrc saved this (`^I` is a tab character):

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

On screen each level is 8 columns; in another editor or `cat` with 8-column tabs it is 16.

### `smartcase` needs `ignorecase`

`smartcase` is an exception to `ignorecase`: "ignore case, unless the pattern contains an uppercase letter". With
`ignorecase` off it does nothing. The two lines sit next to each other in the Debian template, and only
`smartcase` came along.

### A global `cindent` and `#` comments

`cindent` is buffer-local, but `set` in the vimrc sets the value every new buffer inherits, and filetype indenting
wasn't enabled. So Python, shell scripts and YAML were indented by C rules too. The most visible symptom is lines
starting with `#`: in C that is a preprocessor directive like `#include`, which belongs in column 0. Typing a
comment inside a Python block gives:

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

The line snaps to the left margin as soon as `#` is typed. `smartindent` has the same rule, so turning off
`cindent` alone doesn't fix it.

### `fileencodings` and Korean files

With the default (`ucs-bom,utf-8,default,latin1`), a file that isn't UTF-8 opens as `latin1`: an EUC-KR file
containing "안녕하세요" shows as `¾È³çÇÏ¼¼¿ä`. The first vimrc avoided that by retrying a failed UTF-8 read as
EUC-KR, and saving writes the file back in its original encoding. UTF-8's byte rules are strict enough that files
in other encodings are almost never mistaken for UTF-8, so putting the strict one first is the right order.

The gap is that EUC-KR only covers 2,350 precomposed Hangul syllables. Korean files made on Windows are usually
CP949, its superset, and break as soon as they contain a syllable like "똠":

```text
fileencoding=euc-kr   <8c>c방각하
```

### `mouse=a` and copying from the terminal

Because Vim takes the mouse, the terminal's own drag-to-select no longer works inside it. In most terminals,
holding <kbd>Shift</kbd> while dragging bypasses Vim and selects in the terminal. If copying doesn't work on a
server over SSH, this option is the usual reason. It's less a trap than behaviour worth knowing, so it stays in
the current version.

### The current version

With all of the above applied, the repository's vimrc now reads:

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

- Loading `defaults.vim` explicitly brings back `syntax on`, `incsearch`, `scrolloff` and filetype indenting. It's
  wrapped in `filereadable` so minimal installs without `defaults.vim` (`vim-tiny`) don't error out.
- `shiftwidth=0` means "follow `tabstop`", so one indent level is one tab.
- `cindent` and `smartindent` are gone in favour of filetype indenting: in C files Vim's C indent plugin turns
  `cindent` on, while Python and shell get their own rules.
- `cp949` is a superset of EUC-KR, so EUC-KR files still read fine.

The same tests again: C indents by one tab per level, `# note` in a shell script stays where it was typed, and both
Korean files open correctly.

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}

fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

## The zsh theme: ychun.zsh-theme

The theme is based on oh-my-zsh's `bira`. `./install.sh --zsh` puts it in `$ZSH_CUSTOM/themes/` (usually
`~/.oh-my-zsh/custom/themes/`) and selects it with `ZSH_THEME="ychun"` in `.zshrc`. On screen:

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

It is a two-line prompt. The first line has the user, host, current directory and git branch; commands are typed
after the `➤` on the second. On the right: the previous command's exit code (only when it failed) and the time.

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

### Prompt escapes

Every time zsh draws `PROMPT` it replaces the escapes starting with `%`:

| Escape | Meaning |
|---|---|
| `%n` | user name |
| `%M` | full host name (`%m` stops at the first `.`) |
| `%~` | current directory, with the home directory shortened to `~` |
| `%#` | `%` for a normal user, `#` for root |
| `%B` … `%b` | start … end bold |
| `%F{colour}` … `%f` | start a foreground colour … back to the default |
| `%?` | exit status of the previous command |
| `%(?.true.false)` | the first part if that status is 0, the second otherwise |
| `%*` | 24-hour time with seconds |

`return_code`'s `%(?..%F{red}%? ↵%f)` reads "nothing on success, the code and `↵` in red on failure". `[%*]` is
the time the prompt was drawn, which is when the previous command finished: handy to see when a long command ended.

### Colouring root and SSH sessions

- **When the UID is 0**, the user name and `➤` turn red: hard to forget you are in a root shell from `sudo -i`.
- **When `SSH_CLIENT` or `SSH2_CLIENT` is set**, the host name turns red. OpenSSH sets `SSH_CLIENT` in remote
  shells, so the colour of the host name tells a local terminal from one attached to a server at a glance.
  `SSH2_CLIENT` is the variable a commercial SSH implementation used.

`PR_USER_OP` (`%#`) is defined the same way but never used in `PROMPT`; it is left over from replacing bira's
`$`/`#` with `➤`.

### Quotes and `prompt_subst`

`PROMPT` mixes parts whose value is fixed when it is defined with parts that are recomputed every time the prompt
is drawn.

- `${user_host}`, `${current_dir}` and `$PR_PROMPT` are inside double quotes, so they are replaced by their values
  **at definition time**. That is why it's fine for the `PR_*` variables to be `local` to the anonymous function
  `() { … }`.
- `$(git_prompt_info)` is spliced in from a single-quoted string and `\$(ruby_prompt_info)` is protected by a
  backslash. Both stay in `PROMPT` as literal `$(…)`.
- With `setopt prompt_subst`, zsh runs the `$(…)` inside `PROMPT` each time it draws the prompt.

Printing the stored `PROMPT` shows exactly that (run as root):

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

`git_prompt_info` and `ruby_prompt_info` come from oh-my-zsh. With modified files the branch becomes `‹main*›`;
the theme doesn't set `ZSH_THEME_GIT_PROMPT_DIRTY`, so oh-my-zsh's default `*` is used (bira has a red `●`).
`ruby_prompt_info` is empty without rvm, rbenv or chruby, which leaves two spaces between the directory and `‹main›`.

## Porting it to bash: ychun.bash-theme

The zsh theme doesn't work without oh-my-zsh: `git_prompt_info` is an oh-my-zsh function, and oh-my-zsh is what
loads theme files. Servers, containers and embedded boards often don't even have zsh, so I rebuilt the same look
with bash alone. Here is the screen of a real terminal (a pty) after typing a few commands:

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

The full file is `ychun.bash-theme` in the repository. Five things changed in the port.

**1. `PROMPT_COMMAND` instead of `prompt_subst`.** bash runs whatever is in `PROMPT_COMMAND` right before drawing the
prompt. That's where the git information, time and exit code are computed into variables; `PS1` only refers to them.

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

The previous command's exit code `$?` is overwritten by any command at all, so it has to be captured on the
function's **first line**. Returning the same value at the end lets any other `PROMPT_COMMAND` that follows (one
added by the terminal, for instance) see the original `$?` too.

**2. The branch name never goes into `PS1` itself.** If `PS1` is rebuilt each time with the branch name pasted in as
text, bash also runs any `$(…)` in it when it expands `PS1` (`promptvars`). And git accepts a branch named `$(id)`:

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- branch name pasted into PS1
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme: PS1 only refers to a variable name
╭─  root@vm …/inject ‹$(id)›
```

Just cloning someone else's repository and stepping into it could run a command. With a variable name such as
`${__ychun_git}` in `PS1`, bash expands the value once and doesn't interpret what's inside. The zsh theme didn't have
this problem, because the output of `$(git_prompt_info)` isn't interpreted again.

**3. Colour codes are wrapped in `\[ \]`.** readline counts the prompt's characters to place the cursor and wrap
lines. Invisible colour codes not wrapped in `\[` and `\]` get counted too, and long commands or history browsing
then wrap in the wrong place and overwrite text. zsh works out the width of `%F{…}` itself, so this wasn't needed there.

**4. There is no right prompt.** bash has no `RPROMPT`, so before drawing the first line the prompt saves the cursor
(`\e7`), moves to the right-hand column (`\e[colG`), prints the exit code and time, and restores the cursor (`\e8`).
All of it sits inside `\[ \]`, so readline sees it as zero width. That's why, unlike zsh, the time is on the first
line rather than the input line. When the left side is long enough to collide, the right side isn't drawn at all.
In a 44-column terminal the time disappears on long paths, and input wrapping past the edge stays intact:

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

**5. The git information is computed directly.** Three git commands replace oh-my-zsh's `git_prompt_info`:

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

The branch name; failing that (detached HEAD) a tag; failing that a short commit hash. `GIT_OPTIONAL_LOCKS=0` stops
`git status` from taking a lock to refresh the index, so it doesn't collide with a git command running in another
terminal; oh-my-zsh uses the same variable for the same reason.

The rest of what zsh provided:

- The aliases in `.zshrc` (`c`, `sysupdate`) and the history settings moved to `extras.bash`: 50,000 lines,
  duplicates removed, timestamps (`HISTTIMEFORMAT`), and `histappend` so several terminals don't overwrite each other.
- oh-my-zsh plugins such as `zsh-autosuggestions` and `zsh-syntax-highlighting` have no equivalent in bash itself.
  If you really want them, the bash line editor ble.sh provides both.
- The user and host use the terminal's default colour instead of the zsh theme's white (`%F{white}`), which is
  hard to read on light-background terminals.
