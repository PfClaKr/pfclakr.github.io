+++
title = "ft_shield: how a backdoor hides, and how to find it"
description = "An educational trojan for a disposable VM, read from the defender's side: what persistence through systemd actually looks like on disk and in process state, and the exact places a responder checks to catch it."
tags = ["42", "security", "linux"]
+++

**ft_shield** is a 42 security project: build a small "trojan" and run it **only inside a disposable
virtual machine**. The point isn't to attack anyone. It's to see, from the inside, the handful of things
every persistent backdoor must do, so you recognise them instantly when you're the one defending a
machine ([source](https://github.com/PfClaKr/Ecole42Paris_ft_shield)). The repository leads with that
warning, ships a `reset.sh` to remove the program, and this write-up stays on the defender's side: what
to look for and where.

> ⚠️ A program that opens a remote shell is dangerous by definition. ft_shield belongs in an isolated,
> throwaway VM and nowhere else. Everything below is framed as detection, because that's the transferable
> skill.

## What any persistent backdoor must do

Strip away the specifics and a backdoor has only four jobs. Each one leaves a trace.

1. **Survive a reboot** (persistence) — register somewhere the system reads at boot.
2. **Run unattended** (daemonize) — detach from any terminal and keep running in the background.
3. **Listen for the operator** (a channel) — usually a network port, gated by a secret.
4. **Stay installed** (resilience) — resist casual stopping or removal.

ft_shield does the first three in the most ordinary way possible, which is exactly the lesson: modern
persistence doesn't look exotic. It looks like a normal service.

## Persistence that looks legitimate

ft_shield installs itself as a **systemd service**: it writes a unit file, copies its binary to a system
path (`/bin/ft_shield`), and asks systemd to enable it. From then on the init system starts it on every
boot, for free, with no clever tricks. It also detects whether it was launched by the init system
(by checking whether its parent process is `systemd`, PID 1) so it only installs itself once.

For a defender, this is the first and best place to look. A rogue service is visible in all the normal
places, if you know they exist:

```bash
systemctl list-unit-files --state=enabled      # does anything here look unfamiliar?
ls -lt /etc/systemd/system/ /lib/systemd/system # newest unit files first
systemctl cat <suspicious-service>              # read its ExecStart path
```

The giveaways are a unit whose name imitates a real daemon, an `ExecStart` pointing at an odd path
(`/bin`, `/tmp`, a home directory), and a creation time that matches an incident rather than an install.
A legitimate service has a package behind it; `dpkg -S /bin/ft_shield` (or `rpm -qf`) returning "not owned
by any package" is a loud signal.

## Daemonizing, and what it leaves behind

To run unattended, a backdoor detaches from the terminal that started it: it forks, the parent exits, the
child keeps going in the background. Under systemd the service manager does this for you, which is cleaner
for the attacker and still visible to the defender. A background process with no controlling terminal shows
up plainly:

```bash
ps -eo pid,ppid,user,tty,cmd | grep '?'    # TTY of '?' = no terminal
ss -lntp                                   # which process owns each listening port
lsof -p <pid>                              # its open files and sockets
```

The process is running as **root**, re-parented to PID 1, listening on a port. None of that is hidden by
the program; it's just easy to miss if you don't routinely look.

## The channel: a listening port behind a secret

ft_shield listens on TCP **4242** and, before giving a shell, asks for a `Keycode`. The secret keeps a
random port scanner from stumbling into it, but it does nothing to hide the **open port itself**. That's
the detection angle: you don't need the password to notice the door.

```bash
ss -lntp | grep 4242        # an unexpected listener
# from another box, carefully, in the lab only:
nmap <vm-ip>                # an unexplained open port is the finding
```

An unexplained listening socket on a server is one of the highest-signal indicators there is. A real
service maps to a known package and a documented port; a backdoor's port maps to nothing.

## Resilience, and its limits

ft_shield tries to stay installed, but as the project's own `reset.sh` shows, systemd-based persistence is
also systemd-based *removal*. The teardown is exactly the inverse of the install:

```bash
systemctl stop ft_shield && systemctl disable ft_shield
rm -f /etc/systemd/system/ft_shield.service /bin/ft_shield
systemctl daemon-reload
```

The deeper lesson is that persistence and detection use the same handful of system interfaces. Anything an
attacker can register, a defender can enumerate. The asymmetry is only attention: the attacker needs one
spot, the defender has to know all of them.

## The defender's checklist

What ft_shield really teaches is a short list of places to sweep on a Linux host you suspect:

- **Boot persistence:** systemd units (`/etc/systemd/system`, `/lib/systemd/system`, user units), plus the
  classics — cron (`/etc/cron*`, `crontab -l`), `rc.local`, init scripts, shell rc files, `~/.ssh/authorized_keys`.
- **Processes:** anything running as root with no TTY, re-parented to PID 1, or with a name that mimics a
  real daemon but no package behind it.
- **Network:** every listening socket (`ss -lntp`) reconciled against the services you expect.
- **Files:** binaries in system paths not owned by any package, and files whose timestamps cluster around a
  suspicious time.
