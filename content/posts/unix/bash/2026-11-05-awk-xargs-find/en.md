+++
title = "awk, xargs and find: the commands that take a while to click"
description = "The pipeline tools people copy from Stack Overflow without quite understanding them: awk's pattern–action model, xargs and filenames with spaces, find -exec with ; vs +, and the small helpers tr, cut, comm, paste and column."
tags = ["bash", "awk", "unix", "shell"]
+++

The [sed post](/posts/unix/bash/sed/) ended with "use awk when the job is about fields". This one
picks up from there, and adds the other commands that show up in every serious pipeline but rarely
get explained: `xargs`, `find -exec`, and a handful of small tools that each replace a loop you would
otherwise write by hand.

Examples were run with GNU coreutils/findutils and `mawk`; anything specific to one implementation
is pointed out.

## awk: patterns and actions

An awk program is a list of `pattern { action }` rules. For every input line (a **record**), awk
splits it into **fields** `$1`, `$2`, … `$NF`, then runs each action whose pattern matches. A missing
pattern matches every line; a missing action prints the line.

```bash
awk '{ print $1 }' file                      # first field of each line
awk -F: '$3 >= 1000 { print $1, $7 }' /etc/passwd
# alice /bin/zsh
# bob /bin/bash
```

`-F` sets the input field separator (whitespace by default). A few built-in variables do most of the work:

| Variable | Meaning |
|---|---|
| `NR` | number of the current record, across all files |
| `FNR` | record number within the current file |
| `NF` | number of fields in the current record (`$NF` is the last field) |
| `FS` / `OFS` | input / output field separator |
| `BEGIN` / `END` | special patterns run before the first and after the last line |

```bash
awk -F: -v OFS='\t' '{ print $1, $6 }' /etc/passwd   # user<TAB>home
awk '{ s += $NF } END { print s }' sizes.txt         # sum the last column
awk -F: '{ printf "%-8s %5d\n", $1, $3 }' /etc/passwd # aligned columns
```

### Associative arrays: group by in one line

awk arrays are hash maps indexed by strings, which turns "group by" into a one-liner. In the common
log format the client IP is `$1`, the path `$7`, the status `$9` and the size `$10`:

```bash
# 203.0.113.7 - - [05/Nov/2026:10:12:01 +0100] "GET /posts/ HTTP/1.1" 200 5120 "-" "curl/8.5"

awk '$9 == 404 { print $7 }' access.log                   # paths that returned 404
awk '{ bytes[$1] += $10 } END { for (ip in bytes) print bytes[ip], ip }' access.log | sort -rn
```

Two idioms are worth memorising because you will see them everywhere.

**Remove duplicate lines without sorting** (keeps the first occurrence, in order):

```bash
printf 'b\na\nb\nc\na\n' | awk '!seen[$0]++'
# b
# a
# c
```

`seen[$0]++` returns the old count, which is 0 (false) the first time a line appears; `!` makes it
true, and a pattern with no action prints the line.

**Filter one file by the keys of another:**

```bash
awk -F: 'NR == FNR { ok[$1]; next } $1 in ok' allowed-users.txt /etc/passwd
```

While awk reads the first file, `NR` (global) equals `FNR` (per file), so the first rule stores each
key and `next` skips the rest. For the second file that is no longer true, and `$1 in ok` keeps only
the lines whose key was seen. It is a join without sorting either input.

## xargs: turning lines into arguments

Many commands take file names as arguments, not on standard input. The tempting fix,
`rm $(find . -name '*.log')`, breaks as soon as a name contains a space, and can exceed the maximum
argument length on big trees:

```bash
for f in $(find files -name '*.log'); do echo "[$f]"; done
# [files/a]
# [b.log]        ← "a b.log" was split in two
# [files/c.log]
```

`xargs` reads items from standard input and runs a command with them as arguments, batching them so
the command line never gets too long. To make it safe with any file name, separate items with a NUL
byte on both sides:

```bash
find files -name '*.log' -print0 | xargs -0 rm --
```

| Option | Effect |
|---|---|
| `-0` | items are NUL-separated (pair with `find -print0`, `grep -lZ`, …) |
| `-n N` | at most N arguments per command |
| `-P N` | run up to N commands in parallel |
| `-I{}` | one command per input line, `{}` replaced by the line |
| `-r` | GNU: don't run the command at all if the input is empty |

```bash
echo 1 2 3 4 | xargs -n 2 echo           # "1 2" then "3 4"
printf 'x\ny\n' | xargs -I{} echo "item: {}"
find . -name '*.png' -print0 | xargs -0 -P 4 -n 16 optipng   # 4 workers, 16 files each
```

:::warning
The `-r` row matters more than it looks: GNU xargs runs the command once even with no input, so
`... | xargs rm` with an empty list runs a bare `rm`. Add `-r`, or use `find -exec`.
:::

## find -exec: `\;` versus `+`

`find` can run commands itself, which avoids the quoting problem entirely:

```bash
find files -name '*.log' -exec echo {} \;    # one echo per file
find files -name '*.log' -exec echo {} +     # one echo with all the files, like xargs
```

`\;` starts one process per file; `+` packs as many files as fit into each call, which is much faster
on large trees. Use `\;` only when the command takes exactly one file.

Tests are combined with implicit AND, `-o` for OR, and escaped parentheses for grouping. `-prune`
skips a whole directory:

```bash
find files \( -name '*.log' -o -name '*.txt' \) -type f
find proj -path proj/node_modules -prune -o -name '*.js' -print
find /var/log -type f -mtime -1 -size +100M          # modified in the last day, bigger than 100 MiB
find . -name '*.tmp' -delete                         # find's own delete, no xargs needed
```

The `-prune -o … -print` shape looks odd until you read it as: "if the path is `node_modules`, prune
it (and print nothing); otherwise, if it is a `.js` file, print it". Without the explicit `-print`,
the pruned directory itself would be printed too.

## Small tools that replace a loop

Each of these does one thing, and each one regularly saves writing a `while read` loop.

**`tr`** translates or deletes characters:

```bash
echo 'Hello   World' | tr -s ' ' | tr 'a-z' 'A-Z'   # HELLO WORLD
tr -d '\r' < windows.txt > unix.txt                 # strip CRLF line endings
```

**`cut`** picks columns by delimiter or by character position:

```bash
cut -d: -f1,7 /etc/passwd     # user:shell
cut -c1-8 file                # first 8 characters of each line
```

**`comm`** compares two **sorted** files line by line, in three columns: only in the first, only in
the second, in both. You hide columns with `-1`, `-2`, `-3`. Combined with process substitution
`<(…)` it needs no temporary files:

```bash
comm -12 <(sort a.txt) <(sort b.txt)     # lines in both
comm -23 <(sort a.txt) <(sort b.txt)     # lines only in a.txt
```

**`paste`** merges lines: side by side, or all lines of one file into one:

```bash
paste names.txt sizes.txt     # name<TAB>size
paste -sd, list.txt           # a,b,c
```

**`sort -k`**, **`uniq -c`**, **`column -t`** finish most reports:

```bash
sort -t, -k2,2n data.csv      # sort by the 2nd comma-separated field, numerically
sort | uniq -c | sort -rn     # count occurrences, most frequent first
column -t report.txt          # align whitespace-separated columns
```

`-k2,2n` means "key starts and ends at field 2, compare as numbers"; a bare `-k2` would compare from
field 2 to the end of the line.

**`tee`** writes its input to a file and to standard output, which is also the standard way to write
to a root-owned file from a pipeline, because the redirection in `sudo cmd > file` is done by your
shell, not by `sudo`:

```bash
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.d/99-local.conf > /dev/null
```

## Putting it together

```bash
# The ten busiest client IPs
awk '{ print $1 }' access.log | sort | uniq -c | sort -rn | head

# Source files changed in the last day that still contain TODO
find src -type f -mtime -1 -print0 | xargs -0 grep -l TODO

# Users that exist in /etc/passwd but are missing from an allow-list
comm -23 <(cut -d: -f1 /etc/passwd | sort) <(sort allowed-users.txt)
```
