+++
title = "42 Paris"
description = "Every project I completed at École 42 Paris — from libft in C to a Rust kernel, BGP-EVPN labs and Kubernetes — with links to the code and write-ups."
in_menu = true
+++

[École 42](https://42.fr) has no teachers and no lectures: you learn by building projects,
and every project is graded by other students in face-to-face peer evaluations.
These are the projects I completed at **42 Paris** (login `ychun`), grouped by theme.
Each name links to its repository; the full index lives in
[PfClaKr/Ecole42Paris](https://github.com/PfClaKr/Ecole42Paris).

## Write-ups

- [libft: writing my own C standard library](/posts/libft/)
- [get_next_line: reading a file one line at a time](/posts/get-next-line/)
- [ft_printf: variadic functions without the magic](/posts/ft-printf/)
- [minitalk: sending text one signal at a time](/posts/minitalk/)
- [so_long: a tiny 2D game and a map validator](/posts/so-long/)
- [push_swap: sorting with two stacks and eleven moves](/posts/push-swap/)
- [Philosophers: threads, mutexes and not starving](/posts/philosophers/)
- [minishell: writing a small bash](/posts/minishell/)
- [miniRT: a ray tracer from first principles](/posts/minirt/)
- [CPP Modules 00–09: learning C++98 the hard way](/posts/cpp-modules/)
- [Inception: WordPress in three hand-written containers](/posts/inception/)
- [webserv: an HTTP/1.1 server on a single epoll loop](/posts/webserv/)
- [ft_transcendence: server-side Pong for four players](/posts/ft-transcendence/)
- [libasm: back to libft, in x86-64 assembly](/posts/libasm/)
- [little-penguin-1: ten small steps into the Linux kernel](/posts/little-penguin/)
- [KFS: writing an x86 kernel from scratch in Rust](/posts/kfs-rust-kernel/)
- [RainFall: a tour of memory-corruption bugs](/posts/rainfall/)
- [OverRide: exploitation past the easy mitigations](/posts/override/)
- [BADASS: putting L2 on top of L3 with VXLAN and BGP-EVPN](/posts/badass-bgp-evpn-vxlan/)
- [ft_ping: rebuilding ping with raw ICMP sockets](/posts/ft-ping-raw-sockets/)
- [ft_shield: how a backdoor hides, and how to find it](/posts/ft-shield/)
- [Inception-of-Things: Kubernetes from VMs to GitOps](/posts/inception-of-things/)

## C foundations

| Project | What it is | Stack |
|---|---|---|
| [libft](https://github.com/PfClaKr/Ecole42Paris_libft) | My own C standard library (strings, memory, linked lists), reused in later projects · [write-up](/posts/libft/) | C |
| [ft_printf](https://github.com/PfClaKr/Ecole42Paris_ftprintf) | `printf` re-implemented with variadic arguments · [write-up](/posts/ft-printf/) | C |
| [get_next_line](https://github.com/PfClaKr/Ecole42Paris_GNL) | Reads a file descriptor line by line with a static buffer; the bonus handles several fds at once · [write-up](/posts/get-next-line/) | C |
| [push_swap](https://github.com/PfClaKr/Ecole42Paris_PushSwap) | Sorts integers on two stacks with a minimal number of operations, using pivot-based partitioning between the stacks plus a `checker` · [write-up](/posts/push-swap/) | C |
| [so_long](https://github.com/PfClaKr/Ecole42Paris_Solong) | A small 2D game rendered with MiniLibX · [write-up](/posts/so-long/) | C, MiniLibX |

## Unix processes & concurrency

| Project | What it is | Stack |
|---|---|---|
| [minitalk](https://github.com/PfClaKr/Ecole42Paris_Minitalk) | Client/server that sends text bit by bit using only `SIGUSR1`/`SIGUSR2` · [write-up](/posts/minitalk/) | C, signals |
| [minishell](https://github.com/PfClaKr/Ecole42Paris_MiniShell) | A bash-like shell: parsing, pipes, redirections, heredoc, environment and builtins (team of two) · [write-up](/posts/minishell/) | C |
| [Philosophers](https://github.com/PfClaKr/Ecole42Paris_Philosophers) | The dining philosophers problem with threads and mutexes · [write-up](/posts/philosophers/) | C, pthreads |

## Graphics

| Project | What it is | Stack |
|---|---|---|
| [miniRT](https://github.com/PfClaKr/Ecole42Paris_miniRT) | A ray tracer: spheres, planes, cylinders and cones, Phong lighting and shadows; the bonus adds checkerboard, bump and image textures · [write-up](/posts/minirt/) | C, MiniLibX |

## C++

| Project | What it is | Stack |
|---|---|---|
| [CPP Modules 00–09](https://github.com/PfClaKr/Ecole42Paris_CppModule) | From classes and polymorphism to templates and STL containers, ending with a Bitcoin exchange, an RPN calculator and Ford-Johnson merge-insert sort · [write-up](/posts/cpp-modules/) | C++98 |
| [webserv](https://github.com/PfClaKr/Ecole42Paris_Webserv) | An HTTP/1.1 server with an nginx-style config, non-blocking I/O, static files, uploads and CGI (team project) · [write-up](/posts/webserv/) | C++98 |

## Low level & kernel

| Project | What it is | Stack |
|---|---|---|
| [libasm](https://github.com/PfClaKr/libasm-x86_64) | `strlen`, `strcpy`, `strcmp`, `strdup`, `read` and `write` in x86-64 assembly, following the System V ABI · [write-up](/posts/libasm/) | NASM |
| [little-penguin-1](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin) | Linux kernel assignments: building a custom kernel, modules, USB hot-plug, misc devices, debugfs and procfs · [write-up](/posts/little-penguin/) | C, Linux |
| [KFS 1–4](https://github.com/PfClaKr/KFS) | A kernel from scratch: boot, GDT/IDT, interrupts, paging, a heap allocator and a shell (team of two) · [write-up](/posts/kfs-rust-kernel/) | Rust |

## Security

| Project | What it is | Stack |
|---|---|---|
| [RainFall](https://github.com/PfClaKr/Ecole42Paris_Rainfall) | 14 binary-exploitation levels: reverse engineering, stack and heap overflows, format strings · [write-up](/posts/rainfall/) | GDB, Python |
| [OverRide](https://github.com/PfClaKr/Ecole42Paris_Override) | 10 more exploitation levels, the follow-up to RainFall · [write-up](/posts/override/) | GDB, Python |
| [ft_shield](https://github.com/PfClaKr/Ecole42Paris_ft_shield) | An educational trojan for a sandboxed VM, to understand how persistence and backdoors work and how to spot them · [write-up](/posts/ft-shield/) | C, systemd |

## Networking

| Project | What it is | Stack |
|---|---|---|
| [ft_ping](https://github.com/PfClaKr/Ecole42Paris_ft_ping) | `ping` re-implemented with raw ICMP sockets: checksum, TTL, timeouts and RTT statistics · [write-up](/posts/ft-ping-raw-sockets/) | C |
| [BADASS](https://github.com/PfClaKr/Ecole42Paris_BADASS) | GNS3 labs with FRRouting: static and multicast VXLAN, then a BGP-EVPN spine-leaf fabric with a route reflector · [write-up](/posts/badass-bgp-evpn-vxlan/) | GNS3, FRR, Docker |

## DevOps & web

| Project | What it is | Stack |
|---|---|---|
| [Inception](https://github.com/PfClaKr/Ecole42Paris_Inception) | nginx (TLS), WordPress/PHP-FPM and MariaDB, each in its own hand-written container · [write-up](/posts/inception/) | Docker Compose |
| [Inception-of-Things](https://github.com/PfClaKr/Ecole42Paris_Inception_of_Things) | A K3s cluster on Vagrant, Ingress routing by host, then GitOps with Argo CD on K3d (team of two) · [write-up](/posts/inception-of-things/) | Vagrant, K3s, Argo CD |
| [ft_transcendence](https://github.com/sleepychloe/ft_transcendance) | A real-time multiplayer Pong web app with 42 OAuth, a 3D mode, tournaments and ELK logging (team of three, 125/100) · [write-up](/posts/ft-transcendence/) | Django, Channels, PostgreSQL, Three.js |
