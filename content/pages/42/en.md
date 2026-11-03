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

- [libft — writing my own C standard library](/posts/libft/)
- [get_next_line — reading a file one line at a time](/posts/get-next-line/)
- [KFS — writing an x86 kernel from scratch in Rust](/posts/kfs-rust-kernel/)
- [BADASS — putting L2 on top of L3 with VXLAN and BGP-EVPN](/posts/badass-bgp-evpn-vxlan/)
- [ft_ping — rebuilding ping with raw ICMP sockets](/posts/ft-ping-raw-sockets/)

## C foundations

| Project | What it is | Stack |
|---|---|---|
| [libft](https://github.com/PfClaKr/Ecole42Paris_libft) | My own C standard library (strings, memory, linked lists), reused in later projects · [write-up](/posts/libft/) | C |
| [ft_printf](https://github.com/PfClaKr/Ecole42Paris_ftprintf) | `printf` re-implemented with variadic arguments | C |
| [get_next_line](https://github.com/PfClaKr/Ecole42Paris_GNL) | Reads a file descriptor line by line with a static buffer; the bonus handles several fds at once · [write-up](/posts/get-next-line/) | C |
| [push_swap](https://github.com/PfClaKr/Ecole42Paris_PushSwap) | Sorts integers on two stacks with a minimal number of operations, using pivot-based partitioning between the stacks plus a `checker` | C |
| [so_long](https://github.com/PfClaKr/Ecole42Paris_Solong) | A small 2D game rendered with MiniLibX | C, MiniLibX |

## Unix processes & concurrency

| Project | What it is | Stack |
|---|---|---|
| [minitalk](https://github.com/PfClaKr/Ecole42Paris_Minitalk) | Client/server that sends text bit by bit using only `SIGUSR1`/`SIGUSR2` | C, signals |
| [minishell](https://github.com/PfClaKr/Ecole42Paris_MiniShell) | A bash-like shell: parsing, pipes, redirections, heredoc, environment and builtins (team of two) | C |
| [Philosophers](https://github.com/PfClaKr/Ecole42Paris_Philosophers) | The dining philosophers problem with threads and mutexes | C, pthreads |

## Graphics

| Project | What it is | Stack |
|---|---|---|
| [miniRT](https://github.com/PfClaKr/Ecole42Paris_miniRT) | A ray tracer: spheres, planes, cylinders and cones, Phong lighting and shadows; the bonus adds checkerboard, bump and image textures | C, MiniLibX |

## C++

| Project | What it is | Stack |
|---|---|---|
| [CPP Modules 00–09](https://github.com/PfClaKr/Ecole42Paris_CppModule) | From classes and polymorphism to templates and STL containers, ending with a Bitcoin exchange, an RPN calculator and Ford-Johnson merge-insert sort | C++98 |
| [webserv](https://github.com/PfClaKr/Ecole42Paris_Webserv) | An HTTP/1.1 server with an nginx-style config, non-blocking I/O, static files, uploads and CGI (team project) | C++98 |

## Low level & kernel

| Project | What it is | Stack |
|---|---|---|
| [libasm](https://github.com/PfClaKr/libasm-x86_64) | `strlen`, `strcpy`, `strcmp`, `strdup`, `read` and `write` in x86-64 assembly, following the System V ABI | NASM |
| [little-penguin-1](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin) | Linux kernel assignments: building a custom kernel, modules, USB hot-plug, misc devices, debugfs and procfs | C, Linux |
| [KFS 1–4](https://github.com/PfClaKr/KFS) | A kernel from scratch: boot, GDT/IDT, interrupts, paging, a heap allocator and a shell (team of two) · [write-up](/posts/kfs-rust-kernel/) | Rust |

## Security

| Project | What it is | Stack |
|---|---|---|
| [RainFall](https://github.com/PfClaKr/Ecole42Paris_Rainfall) | 14 binary-exploitation levels: reverse engineering, stack and heap overflows, format strings | GDB, Python |
| [OverRide](https://github.com/PfClaKr/Ecole42Paris_Override) | 10 more exploitation levels, the follow-up to RainFall | GDB, Python |
| [ft_shield](https://github.com/PfClaKr/Ecole42Paris_ft_shield) | An educational trojan for a sandboxed VM, to understand how persistence and backdoors work and how to spot them | C, systemd |

## Networking

| Project | What it is | Stack |
|---|---|---|
| [ft_ping](https://github.com/PfClaKr/Ecole42Paris_ft_ping) | `ping` re-implemented with raw ICMP sockets: checksum, TTL, timeouts and RTT statistics · [write-up](/posts/ft-ping-raw-sockets/) | C |
| [BADASS](https://github.com/PfClaKr/Ecole42Paris_BADASS) | GNS3 labs with FRRouting: static and multicast VXLAN, then a BGP-EVPN spine-leaf fabric with a route reflector · [write-up](/posts/badass-bgp-evpn-vxlan/) | GNS3, FRR, Docker |

## DevOps & web

| Project | What it is | Stack |
|---|---|---|
| [Inception](https://github.com/PfClaKr/Ecole42Paris_Inception) | nginx (TLS), WordPress/PHP-FPM and MariaDB, each in its own hand-written container | Docker Compose |
| [Inception-of-Things](https://github.com/PfClaKr/Ecole42Paris_Inception_of_Things) | A K3s cluster on Vagrant, Ingress routing by host, then GitOps with Argo CD on K3d (team of two) | Vagrant, K3s, Argo CD |
| [ft_transcendence](https://github.com/sleepychloe/ft_transcendance) | A real-time multiplayer Pong web app with 42 OAuth, a 3D mode, tournaments and ELK logging (team of three, 125/100) | Django, Channels, PostgreSQL, Three.js |
