+++
title = "About"
description = "Who I am, what I work on, and how to reach me."
in_menu = true
+++

## Hello 👋

I'm **YuGeon CHUN** (PfClaKr), an embedded Linux engineer based in Paris, France.
At BALYO I work on the OS layer of autonomous forklifts, from the boot chain to fleet deployment.

I rebuilt the robots' embedded OS from scratch, migrating legacy boot to UEFI, and set up a reproducible,
automated OS build where no procedure existed. That foundation now powers a security maintenance pipeline:
automated build, patching and deployment.

Outside work I like going low-level: a [32-bit x86 kernel in Rust](/posts/42/kfs-rust-kernel/)
and [libc functions in x86-64 assembly](/posts/42/libasm/).

Before tech, I ran a restaurant on the Champs-Élysées for almost seven years — team, accounting, operations.
It taught me ownership and how to work under production constraints.

## Experience

**Embedded Linux Engineer — R&D System Interface**, BALYO · Apr 2026 – present

- OS security maintenance pipeline: build → security patches → OS image (≈ 40 min per build), with fleet rollout via Ansible in progress.
- Ported 20+ versions of robotics SDKs (10 years of history on TinyCore, Ubuntu 14–16, Debian 11) to Debian 13.
- Designed an A/B-partition OTA update mechanism with RAUC: persistent data split from the system, safe rollback on safety-critical robots.

**Linux Systems Engineer Intern — R&D System Interface**, BALYO · Sep 2025 – Mar 2026

- Rebuilt the robots' embedded OS from scratch and migrated legacy boot to UEFI.
- Replaced a hand-modified OS with a fully automated, reproducible build (CI/CD).
- Ported older industrial PC platforms to the new OS and centralised logs from many robots for fleet debugging.
- Low-level debugging of critical robotics tools in C.

**Restaurant Manager**, Restaurant Soon (Champs-Élysées) · May 2018 – Jan 2025

- Ran a 56-seat restaurant: team management, accounting, maintenance, service. Continued alongside 42 from 2021.

## Education

**42 Paris** · 2021 – Oct 2026 —
RNCP level 7 (Master's) *Expert en architecture informatique*, option information systems and networks.
Every project is listed on the [42 Paris page](/42/).

## Skills

| Area           | Tools                                                  |
|----------------|--------------------------------------------------------|
| Languages      | C, Rust, x86 assembly, C++, Python, Bash               |
| Systems        | Embedded Linux, Linux kernel, UEFI, systemd, Debian    |
| Build & deploy | GitLab CI, Ansible, RAUC (A/B OTA), Docker             |
| Low-level      | Kernel development, reverse engineering, debugging     |

Languages: Korean (native), French (full professional), English (professional working).

## Contact

- GitHub: [@PfClaKr](https://github.com/PfClaKr)
- LinkedIn: [pfclakr](https://www.linkedin.com/in/pfclakr)
