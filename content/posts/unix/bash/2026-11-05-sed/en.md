+++
title = "sed: the stream editor, beyond s///"
description = "How sed actually works — pattern space, addresses, hold space — and the one-liners that make it worth learning: ranges, in-place edits, multi-line joins, and the GNU vs BSD traps."
tags = ["bash", "sed", "unix", "shell"]
+++

Most people know exactly one sed command: `s/old/new/g`. That covers a lot, but sed is a small
programming language, and the rest of it is what makes one-liners like "change `port` only inside
the `[server]` section" or "join lines that end with a backslash" possible. This post goes through
the model first, because once the model clicks, the cryptic scripts stop being cryptic.

All examples were run with GNU sed 4.9. Where BSD/macOS sed differs, it is called out.

## How sed thinks: one line at a time

sed reads its input one line at a time into a buffer called the **pattern space**, runs the whole
script against it, prints the pattern space, and moves on to the next line. That is the entire loop.

```bash
printf 'a\nb\nc\n' | sed 's/b/B/'
# a
# B
# c
```

The automatic print at the end of each cycle is why `-n` exists: it turns that print off, so only
lines you print explicitly with `p` come out.

```bash
printf 'a\nb\nc\n' | sed -n '2p'     # b
sed -n '5,10p' big.log              # lines 5 to 10, like head | tail
sed 3q file                         # print 3 lines then quit: head -n 3
```

## Substitution, properly

The full form is `s/regex/replacement/flags`.

- **Flags**: `g` replaces every match on the line, a number replaces only the Nth match, `p` prints
  the line if a substitution happened (useful with `-n`), and GNU adds `I` for case-insensitive.
- **Any delimiter**: the character after `s` is the delimiter, so paths don't need escaping.
- **`&`** in the replacement is the whole match; **`\1`…`\9`** are capture groups.
- **`-E`** switches to extended regular expressions (`+`, `?`, `()`, `|` without backslashes).
  Both GNU and BSD sed accept `-E`.

```bash
echo 'one one one' | sed 's/one/two/2'                 # one two one
echo 'PATH=/usr/local/bin' | sed 's#/usr/local#/opt#'  # PATH=/opt/bin
echo 'error 42' | sed -E 's/[0-9]+/[&]/'               # error [42]
echo 'Turing, Alan' | sed -E 's/^([^,]+), (.+)$/\2 \1/' # Alan Turing
```

GNU sed also understands case conversion in the replacement: `\U` uppercases until `\E`,
`\L` lowercases, `\u`/`\l` change one character. Handy for renaming identifiers:

```bash
echo 'max_retry_count' | sed -E 's/_([a-z])/\U\1/g'    # maxRetryCount (GNU only)
```

## Addresses: choosing which lines

A command can be prefixed by an **address** that decides which lines it runs on. Without one, it
runs on every line.

| Address | Meaning |
|---|---|
| `3` | line 3 |
| `$` | the last line |
| `/re/` | lines matching `re` |
| `10,20` | lines 10 to 20 |
| `/BEGIN/,/END/` | from a line matching `BEGIN` to the next line matching `END` |
| `/re/,+2` | the matching line and the two after it (GNU) |
| `0~2` | every second line: first~step (GNU) |
| `addr!` | every line that does **not** match `addr` |

Ranges are where sed becomes genuinely useful for config files. Change `port` only inside the
`[server]` section of an INI file, and leave `[client]` alone:

```bash
cat config.ini
# [server]
# port=80
# [client]
# port=1

sed '/^\[server\]/,/^\[/ s/^port=.*/port=8080/' config.ini
# [server]
# port=8080
# [client]
# port=1
```

The range starts at `[server]` and ends at the next line starting with `[`, so the substitution only
sees the lines in between. A few more:

```bash
sed -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' app.conf  # drop comments and blank lines
sed -n '/ERROR/,+2p' app.log                                # each error and two lines of context
sed '/keep/!d' list.txt                                     # delete everything that doesn't match
seq 1 6 | sed -n '0~2p'                                     # 2 4 6
```

## Commands other than s

| Command | Does |
|---|---|
| `d` | delete the pattern space and start the next cycle |
| `p` | print the pattern space |
| `q` | print (unless `-n`) and quit |
| `a text`, `i text`, `c text` | append after, insert before, or replace the line |
| `y/abc/xyz/` | transliterate characters, like `tr` |
| `=` | print the current line number |
| `r file`, `w file` | read a file into the output, write the line to a file |

```bash
sed '/^\[server\]/a timeout=30' config.ini   # add a line after [server]
printf 'a\nold\nb\n' | sed '/old/c\
new'                                         # replace a whole line
sed -n '$=' file                             # count lines, like wc -l
```

The one-line `a text` form is a GNU extension. POSIX (and BSD) want a backslash and a newline:
`a\` followed by the text on the next line, which is why scripts written for macOS look so odd.

## In-place editing, and its portability trap

`-i` writes the result back to the file instead of standard output. The catch is that GNU and BSD
disagree on the syntax:

```bash
sed -i 's/foo/bar/' file        # GNU
sed -i '' 's/foo/bar/' file     # BSD / macOS: the backup suffix is mandatory, '' means none
sed -i.bak 's/foo/bar/' file && rm file.bak   # works on both
```

Two more things to know about `-i`: it writes a new file and renames it over the old one, so hard
links to the original are broken (and a symlink is replaced by a regular file unless you pass GNU's
`--follow-symlinks`). And it happily rewrites a file even when nothing matched, which changes its
modification time.

## The hold space: multi-line tricks

Besides the pattern space, sed has a second buffer, the **hold space**, that survives from one line
to the next. Five commands move data between them:

| Command | Effect |
|---|---|
| `h` / `H` | copy / append the pattern space to the hold space |
| `g` / `G` | copy / append the hold space to the pattern space |
| `x` | swap the two |

Together with `N` (append the next input line to the pattern space), labels (`:a`) and branches
(`b a` always, `t a` only if a substitution succeeded), this is enough for most multi-line jobs.

**Reverse a file** (what `tac` does):

```bash
printf '1\n2\n3\n' | sed -n '1!G;h;$p'
# 3
# 2
# 1
```

Read it per line: on every line except the first, append what we held so far (`G`); store the
result back (`h`); on the last line, print it. The hold space grows as "current line, then everything
before it".

**Join all lines with a comma:**

```bash
printf 'a\nb\nc\n' | sed ':a;N;$!ba;s/\n/,/g'      # a,b,c
```

`N` keeps appending lines to the pattern space and `$!ba` loops until the last line; then a single
substitution replaces the embedded newlines. (`paste -sd, file` does the same and is easier to read.)

**Join lines that end with a backslash**, as in shell scripts or Makefiles:

```bash
printf 'one \\\ntwo \\\nthree\nfour\n' | sed -e ':a' -e '/\\$/N; s/\\\n//; ta'
# one two three
# four
```

**Print the line before each match:**

```bash
printf 'a\nb\nMATCH\nc\n' | sed -n '/MATCH/{x;p;x;}; h'    # b
```

Every line is saved with `h`; when a line matches, swap to get the previous one, print it, and
swap back.

## GNU vs BSD at a glance

| | GNU sed | BSD / macOS sed |
|---|---|---|
| In-place edit | `-i` | `-i ''` |
| `\n`, `\t` in the replacement | yes | no |
| `\U`, `\L` case conversion | yes | no |
| `first~step`, `addr,+N` | yes | no |
| `a text` on one line | yes | no, use `a\` + newline |
| `-E` | yes | yes |

On macOS, `brew install gnu-sed` gives you `gsed`. If a script has to run everywhere, stick to the
POSIX subset and test it with `sed --posix` on GNU.

## When not to use sed

sed is line-oriented and has no notion of fields or nesting. When the job is "the third column
where the second is greater than 100", reach for awk. For JSON use `jq`, for CSV with quoted commas
use a real parser, and if the multi-line script needs more than a couple of `N`/`G` tricks, a few
lines of awk or Python will be easier to read in six months. The
[next post](/posts/unix/bash/awk-xargs-find/) covers awk and the other tools that tend to show up
next to sed in pipelines.
