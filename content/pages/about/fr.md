+++
title = "À propos"
description = "Qui je suis, sur quoi je travaille et comment me contacter."
in_menu = true
+++

## Bonjour 👋

Je suis **YuGeon CHUN** (PfClaKr), ingénieur Linux embarqué basé à Paris.
Chez BALYO, je travaille sur la couche OS de chariots élévateurs autonomes, de la chaîne de boot au déploiement sur la flotte.

J’ai reconstruit from scratch l’OS embarqué des robots, migré le boot legacy vers UEFI et mis en place
un build OS automatisé et reproductible là où aucune procédure n’existait. Cette base fait aujourd’hui tourner
un pipeline de maintenance de sécurité : build, correctifs et déploiement automatisés.

En dehors du travail, j’aime descendre bas niveau : un [noyau x86 32 bits en Rust](/fr/posts/kfs-rust-kernel/)
et des [fonctions de la libc en assembleur x86-64](/fr/posts/libasm/).

Avant la tech, j’ai dirigé un restaurant sur les Champs-Élysées pendant près de sept ans : équipe, comptabilité, exploitation.
J’y ai appris le sens des responsabilités et le travail sous contraintes de production.

## Expérience

**Ingénieur Linux embarqué — R&D System Interface**, BALYO · avr. 2026 – aujourd’hui

- Pipeline de maintenance de sécurité de l’OS : build → correctifs de sécurité → image OS (≈ 40 min par build) ; déploiement sur la flotte via Ansible en cours.
- Portage de plus de 20 versions de SDK robotiques (10 ans d’historique ; TinyCore, Ubuntu 14–16, Debian 11) vers Debian 13.
- Conception d’une mise à jour OTA à partitions A/B (RAUC) : données persistantes séparées du système et rollback sûr, sur des robots où la sûreté est critique.

**Stagiaire ingénieur système Linux — R&D System Interface**, BALYO · sept. 2025 – mars 2026

- Reconstruction from scratch de l’OS embarqué des robots et migration du boot legacy vers UEFI.
- Build OS entièrement automatisé et reproductible (CI/CD), à la place d’un OS modifié à la main depuis des années.
- Adaptation des anciennes plateformes PC industrielles et centralisation des logs de plusieurs robots pour le débogage de flotte.
- Débogage bas niveau en C d’outils robotiques critiques.

**Responsable de restaurant**, Restaurant Soon (Champs-Élysées) · mai 2018 – janv. 2025

- Direction d’un restaurant de 56 couverts : équipe, comptabilité, maintenance, service. Poursuivi en parallèle de 42 à partir de 2021.

## Formation

**42 Paris** · 2021 – oct. 2026 —
Titre RNCP niveau 7 (Bac+5) *Expert en architecture informatique*, option systèmes d’information et réseaux.
Tous les projets sont sur la [page 42 Paris](/fr/42/).

## Compétences

| Domaine          | Outils                                                 |
|------------------|--------------------------------------------------------|
| Langages         | C, Rust, assembleur x86, C++, Python, Bash             |
| Systèmes         | Linux embarqué, noyau Linux, UEFI, systemd, Debian     |
| Build & déploiement | GitLab CI, Ansible, RAUC (OTA A/B), Docker          |
| Bas niveau       | Développement noyau, rétro-ingénierie, débogage        |

Langues : coréen (natif), français (courant professionnel), anglais (professionnel).

## Contact

- GitHub : [@PfClaKr](https://github.com/PfClaKr)
- LinkedIn : [pfclakr](https://www.linkedin.com/in/pfclakr)
