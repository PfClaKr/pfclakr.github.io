+++
title = "setting_linux: reading an 18-line vimrc and a zsh theme, line by line"
description = "The vimrc and the bira-based oh-my-zsh theme I carry to every new Linux machine, explained line by line: what each option does, the traps found by actually running it (tab width, smartcase, # comments, EUC-KR), and how the prompt string gets drawn."
tags = ["linux", "vim", "zsh", "shell", "dotfiles"]
+++

The files I copy first onto every new Linux machine live in the
[setting_linux](https://github.com/PfClaKr/setting_linux) repository: `vimrc`, `ychun.zsh-theme` and
`.zshrc`. This post covers the `vimrc` and the theme. They are short files, but each option does more
than it looks like, and running them turned up a few places where they don't do what they were meant to.

Everything below was checked with Vim 9.1, zsh 5.9 and a current oh-my-zsh.

## vimrc

The whole file, copied to `~/.vimrc`:

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

### What each option does

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
Debian/Ubuntu's `/etc/vim/vimrc`; the file is essentially a pick of lines from there.

### A `~/.vimrc` turns off defaults.vim

When there is **no** user vimrc, Vim loads `$VIMRUNTIME/defaults.vim`, which sets `incsearch` (search as
you type), `scrolloff=5` (five lines of context around the cursor), `showcmd`, `wildmenu`,
`filetype plugin indent on` and more. The moment `~/.vimrc` exists, that file is no longer loaded. If
search stopped following your typing after you created your first vimrc, this is why.

On Debian-based systems the system vimrc still loads `debian.vim`, which keeps the basics such as
`nocompatible` and `backspace=indent,eol,start`. For the rest, put `source $VIMRUNTIME/defaults.vim` at
the top of the vimrc.

### Tab width: `ts=4`, but indents are 8 columns

The file changes `tabstop` to 4 and leaves `shiftwidth` alone, whose default is 8. `cindent`, `>>` and
`smarttab`'s <kbd>Tab</kbd> all indent by `shiftwidth`, so one indent level becomes two 4-column tabs.
Typing C code with this vimrc saves this (`^I` is a tab character):

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

On screen each level is 8 columns; in another editor or `cat` with 8-column tabs it is 16. Adding
`set shiftwidth=4`, or `set shiftwidth=0` to follow `tabstop`, makes one level a single tab.

### `smartcase` needs `ignorecase`

`smartcase` is an exception to `ignorecase`: "ignore case, unless the pattern contains an uppercase
letter". With `ignorecase` off it does nothing, so with this file searches are always case-sensitive. The
two lines sit next to each other in the Debian template, and only `smartcase` came along. For the
intended behaviour, add `set ignorecase`.

### A global `cindent` and `#` comments

`cindent` is buffer-local, but `set` in the vimrc sets the value every new buffer inherits, and
filetype indenting (`filetype indent on`) isn't enabled. So Python, shell scripts and YAML are indented
by C rules too. The most visible symptom is lines starting with `#`: in C that is a preprocessor
directive like `#include`, which belongs in column 0. Typing a comment inside a Python block gives:

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

The line snaps to the left margin as soon as `#` is typed. `smartindent` has the same rule, so turning
off `cindent` alone doesn't fix it. Dropping both and using `filetype plugin indent on` gives C rules in
C files and Python rules in Python files.

### `fileencodings` and Korean files

With the default (`ucs-bom,utf-8,default,latin1`), a file that isn't UTF-8 opens as `latin1`. An
EUC-KR file containing "안녕하세요" looks like this:

```text
fileencoding=latin1   ¾È³çÇÏ¼¼¿ä
```

With this vimrc, a failed UTF-8 read is retried as EUC-KR and displays correctly, and saving writes it
back in its original encoding (`fileencoding=euc-kr`). The order matters: UTF-8's byte rules are strict
enough that files in other encodings are almost never mistaken for UTF-8, so the strict one goes first.
ASCII-only files are detected as UTF-8.

The gap is that EUC-KR only covers 2,350 precomposed Hangul syllables. Korean files made on Windows are
usually CP949, its superset, and break as soon as they contain a syllable like "똠":

```text
fileencoding=euc-kr   <8c>c방각하
```

Using `cp949` in place of `euc-kr` reads both EUC-KR and CP949 files correctly.

### `mouse=a` and copying from the terminal

Because Vim takes the mouse, the terminal's own drag-to-select no longer works inside it. In most
terminals, holding <kbd>Shift</kbd> while dragging bypasses Vim and selects in the terminal. If copying
doesn't work on a server over SSH, this option is the usual reason.

### With all of the above applied

```vim
source $VIMRUNTIME/defaults.vim   " incsearch, scrolloff, filetype plugin indent on, ...

set mouse=a
set tabstop=4 shiftwidth=0        " one indent level = one tab
set hlsearch
set number
set autoindent
set laststatus=2
set showmatch
set ignorecase smartcase
set smarttab
set fileencodings=utf-8,cp949
```

`defaults.vim` turns on `syntax on`, `ruler` and filetype indenting, so the `cindent`, `smartindent`
and `syntax on` lines are gone.

## The zsh theme: ychun.zsh-theme

The theme is based on oh-my-zsh's `bira`. It goes in `$ZSH_CUSTOM/themes/` (usually
`~/.oh-my-zsh/custom/themes/`) and is selected with `ZSH_THEME="ychun"` in `.zshrc`. On screen:

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

It is a two-line prompt. The first line has the user, host, current directory and git branch; commands
are typed after the `➤` on the second. On the right: the previous command's exit code (only when it
failed) and the time.

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

Every time zsh draws `PROMPT` it replaces the escapes starting with `%`. The ones used here:

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

`return_code`'s `%(?..%F{red}%? ↵%f)` reads "nothing on success, the code and `↵` in red on failure", so
the number on the right only appears when something failed. `[%*]` is the time the prompt was drawn,
which is when the previous command finished: handy to see when a long command ended.

### Colouring root and SSH sessions

The two `if`s at the top are the core of the theme.

- **When the UID is 0**, the user name and `➤` turn red: hard to forget you are in a root shell from
  `sudo -i`.
- **When `SSH_CLIENT` or `SSH2_CLIENT` is set**, the host name turns red. OpenSSH sets `SSH_CLIENT` in
  remote shells, so the colour of the host name tells a local terminal from one attached to a server at a
  glance. `SSH2_CLIENT` is the variable a commercial SSH implementation used.

`PR_USER_OP` (`%#`) is defined the same way but never used in `PROMPT`; it is left over from replacing
bira's `$`/`#` with `➤`.

### Quotes and `prompt_subst`

The confusing part of the theme is the quoting. `PROMPT` mixes parts whose value is fixed when it is
defined with parts that are recomputed every time the prompt is drawn.

```zsh
local git_branch='$(git_prompt_info)'          # single quotes: stored literally
PROMPT="╭─${user_host} ${current_dir} \$(ruby_prompt_info) ${git_branch}
╰─$PR_PROMPT "                                 # double quotes: expanded now
```

- `${user_host}`, `${current_dir}` and `$PR_PROMPT` are inside double quotes, so they are replaced by
  their values **at definition time**. That is why it's fine for the `PR_*` variables to be `local` to the
  anonymous function `() { … }`: when the function returns and they disappear, `PROMPT` already holds
  their values. The anonymous function also keeps them out of the global shell namespace.
- `$(git_prompt_info)` is spliced in from a single-quoted string and `\$(ruby_prompt_info)` is protected
  by a backslash. Both stay in `PROMPT` as literal `$(…)`.
- With `setopt prompt_subst`, zsh runs the `$(…)` inside `PROMPT` each time it draws the prompt, so the
  git information follows as soon as you change directory or branch.

Printing the stored `PROMPT` shows exactly that (run as root):

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

### git and ruby information

`git_prompt_info` and `ruby_prompt_info` come from oh-my-zsh; the theme only sets the strings around them
through `ZSH_THEME_*` variables. Inside a git repository you get a yellow `‹main›`, outside it nothing.

- **With modified files** it becomes `‹main*›`. The theme doesn't set `ZSH_THEME_GIT_PROMPT_DIRTY`, so
  oh-my-zsh's default `*` is used; bira puts a red `●` there.
- `ruby_prompt_info` only prints a Ruby version when rvm, rbenv or chruby is present. On most machines it
  is empty, which leaves two spaces between the directory and `‹main›`, as in the example above.
- Current oh-my-zsh computes `git_prompt_info` asynchronously: in a large repository where `git status`
  is slow, the prompt is drawn first and the branch fills in a moment later.

### What changed from bira

| | bira | ychun |
|---|---|---|
| user@host | bold green, red for root | white user, cyan `@`, red host over SSH |
| input symbol | `$` / `#` | `➤` (red for root) |
| right side | failed exit code | failed exit code + time |
| colours | `%{$fg[red]%}` … `%{$reset_color%}` | `%F{red}` … `%f` |
| other info | conda, virtualenv, hg, kube-ps1 | git and ruby only |

The colour syntax is more than taste. `$fg[red]` inserts the terminal escape code directly, so it has to
be wrapped in `%{ … %}` or zsh miscounts the prompt's width. `%F{…}` is handled by zsh itself, needs no
wrapping, and also accepts 256-colour numbers such as `%F{208}`.
