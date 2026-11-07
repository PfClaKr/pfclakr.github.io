+++
title = "vimzip: a Vim dev environment you just unzip"
description = "A single statically-linked tarball that turns any x86-64 Linux box into my full Vim setup — no root, no package manager, no glibc surprises. How it's built and why each piece is there."
tags = ["vim", "linux", "tools"]
+++

I spend a lot of time on machines that aren't mine: a fresh VM, a 42 cluster box, a customer's server over SSH. Every time, the same friction — the system Vim is old, there's no `clangd`, I can't `apt install` anything, and `pip`/`npm` are a minefield behind a proxy. **vimzip** is my answer: one tarball you unzip in your home directory, and you have my entire editor.

```sh
curl -fsSL https://github.com/PfClaKr/vimzip/releases/latest/download/vimzip-linux-x86_64.tar.gz | tar -xz -C ~
~/vimzip/install.sh
vim .
```

No root, no system packages touched. It symlinks `~/.local/bin/vim`, sets `EDITOR=vim`, and that's it. To uninstall, delete the folder.

![vim . opens the tree, the last file, and the editor](layout.png)

([source](https://github.com/PfClaKr/vimzip))

## Why static linking is the whole trick

The reason "just copy my Vim over" normally fails is shared libraries. A Vim built on Ubuntu 24.04 links against that box's glibc and ncurses; drop it on CentOS 7 and it dies with `GLIBC_2.34 not found`. So vimzip builds Vim **statically against musl**, not glibc:

```sh
PKG_CONFIG=false ./configure CC="$CC" \
  CPPFLAGS="-I$W/nc/include -I$W/nc/include/ncursesw" \
  LDFLAGS="-static -L$W/nc/lib" \
  --with-features=huge --with-tlib=ncursesw --enable-multibyte \
  --disable-gui --without-x --disable-gpm ...
```

ncurses is compiled first, also static (`--without-shared`), with a generous `--with-fallbacks` list (`xterm-256color`, `screen`, `tmux-256color`…) baked in so the terminal still works even where `/usr/share/terminfo` is missing. The result is one `vim` binary with zero shared-library dependencies — it runs the same on Alpine, Debian oldstable, or a scratch container. The same approach covers the companion tools: `ctags`, `fzf`, `ripgrep`, `shellcheck` are pulled in as upstream's static release binaries at pinned versions, never from the host.

## No plugin manager, just `pack/`

Vim 8+ has a native package layout, so vimzip needs no vim-plug or Lazy. Plugins live under `pack/vimzip/start/` and load automatically; in the repo they're git submodules, and the release just ships the checked-out copies with `.git`/tests stripped. The set is deliberately small and all native-Vimscript (no Node, no Python host):

- **ALE** — linting and LSP client (uses `clangd` when the host has it, falls back to the system compiler otherwise)
- **gutentags** + **tagbar** — automatic ctags and a symbol outline
- **fzf.vim**, **nerdtree** (+git), **gitgutter**, **fugitive**, **lightline** (+bufferline)

plus Vim's own built-ins that people forget are there: `comment`, `editorconfig`, `matchit`, `termdebug`, and `osc52`.

## The config decisions I actually care about

A portable binary is half of it; the other half is that it behaves like an editor you've already tuned.

**`vim .` is a project, not a file.** Starting in a directory restores a per-folder session (open splits, the last file you touched) and opens the tree. The session and everything else — undo history, swap, tags — live in `~/.cache/vimzip`, never scattered inside your project.

**Autosave, because I forget.** Leaving insert mode, any normal-mode change, losing focus, or switching buffers all trigger a silent `update`. Combined with persistent undo (`undofile`), it feels closer to a modern editor than to classic Vim.

**Clipboard over SSH.** Yanks are mirrored to the system clipboard via OSC 52 escape sequences, so `y` on a remote box lands in your *local* clipboard if the terminal supports it — no X forwarding, no `xclip`.

**Jump-to-definition that degrades gracefully.** With `clangd` it's a real LSP jump; without it, vimzip falls back to project tags, and if even those miss, it generates tags on the fly for the *system headers the current file includes* (walking `cc -M`), so `gd` on `malloc` still lands in the libc header.

**Completion from the whole project.** `Ctrl+n` offers words collected by ripgrep across the git repo in a background job — cheap, no indexer, and it covers files you haven't opened.

There's even an AI-agent panel (`Space a`) that launches whichever of `claude`/`codex`/`gemini`/`aider`/`opencode` it finds. The [full keymap, with screenshots, is its own post](/posts/etc/vimzip-keymap/).

## Keeping it fresh without touching it

Tool versions are pinned in one `versions.env`. A weekly GitHub Action resolves the latest upstream tags, and if anything changed it rebuilds the whole tarball, runs a smoke test (boot Vim headless, confirm plugins loaded, generate help tags), and opens a PR. Merging the PR publishes a new release. So the thing stays current on its own, and every release is one that actually booted.

## Why I like it

The philosophy is boring on purpose: **one file, no install step beyond unzip, no state outside two folders, and deleting it leaves no trace.** It isn't trying to be Neovim-with-everything. It's the smallest thing that makes a stranger's Linux box feel like my machine, and it works the same whether that box is from 2014 or yesterday.
