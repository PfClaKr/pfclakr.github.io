+++
title = "42 Paris"
description = "Tous les projets que j’ai réalisés à l’École 42 Paris — de libft en C à un noyau en Rust, en passant par BGP-EVPN et Kubernetes — avec le code et des articles détaillés."
in_menu = true
+++

À l’[École 42](https://42.fr), il n’y a ni professeurs ni cours : on apprend en réalisant des
projets, et chaque projet est évalué en face à face par d’autres étudiants.
Voici les projets que j’ai validés à **42 Paris** (login `ychun`), classés par thème.
Chaque nom renvoie vers son dépôt ; l’index complet se trouve dans
[PfClaKr/Ecole42Paris](https://github.com/PfClaKr/Ecole42Paris).

<p class="stats-42"><a href="https://42cv.dev/ychun" rel="noopener"><img src="https://42cv.dev/api/badge/cmp1jhhsl00020jt1xrx8atv5/stats?cursusId=21&amp;coalitionId=48" alt="Statistiques 42 de ychun" loading="lazy" decoding="async"></a></p>

## Fondamentaux en C

| Projet | Description | Technos |
|---|---|---|
| [libft](https://github.com/PfClaKr/Ecole42Paris_libft) | Ma propre bibliothèque standard C (chaînes, mémoire, listes chaînées), réutilisée dans les projets suivants · [article](/fr/posts/42/libft/) | C |
| [ft_printf](https://github.com/PfClaKr/Ecole42Paris_ftprintf) | `printf` recodé avec des arguments variadiques · [article](/fr/posts/42/ft-printf/) | C |
| [get_next_line](https://github.com/PfClaKr/Ecole42Paris_GNL) | Lecture d’un descripteur ligne par ligne avec un buffer statique ; le bonus gère plusieurs fd à la fois · [article](/fr/posts/42/get-next-line/) | C |
| [push_swap](https://github.com/PfClaKr/Ecole42Paris_PushSwap) | Tri d’entiers sur deux piles avec un minimum d’opérations, par partitionnement autour de pivots, avec un `checker` · [article](/fr/posts/42/push-swap/) | C |
| [so_long](https://github.com/PfClaKr/Ecole42Paris_Solong) | Un petit jeu 2D affiché avec la MiniLibX · [article](/fr/posts/42/so-long/) | C, MiniLibX |

## Processus Unix et concurrence

| Projet | Description | Technos |
|---|---|---|
| [minitalk](https://github.com/PfClaKr/Ecole42Paris_Minitalk) | Un client et un serveur qui échangent du texte bit par bit avec seulement `SIGUSR1`/`SIGUSR2` · [article](/fr/posts/42/minitalk/) | C, signaux |
| [minishell](https://github.com/PfClaKr/Ecole42Paris_MiniShell) | Un shell inspiré de bash : parsing, pipes, redirections, heredoc, variables d’environnement et builtins (en binôme) · [article](/fr/posts/42/minishell/) | C |
| [Philosophers](https://github.com/PfClaKr/Ecole42Paris_Philosophers) | Le dîner des philosophes avec des threads et des mutex · [article](/fr/posts/42/philosophers/) | C, pthreads |

## Graphisme

| Projet | Description | Technos |
|---|---|---|
| [miniRT](https://github.com/PfClaKr/Ecole42Paris_miniRT) | Un ray tracer : sphères, plans, cylindres et cônes, éclairage de Phong et ombres ; le bonus ajoute damier, bump mapping et textures · [article](/fr/posts/42/minirt/) | C, MiniLibX |

## C++

| Projet | Description | Technos |
|---|---|---|
| [Modules CPP 00–09](https://github.com/PfClaKr/Ecole42Paris_CppModule) | Des classes et du polymorphisme jusqu’aux templates et aux conteneurs de la STL, pour finir par un échange de bitcoins, une calculatrice RPN et le tri de Ford-Johnson · [article](/fr/posts/42/cpp-modules/) | C++98 |
| [webserv](https://github.com/PfClaKr/Ecole42Paris_Webserv) | Un serveur HTTP/1.1 avec une configuration à la nginx, des E/S non bloquantes, fichiers statiques, upload et CGI (en équipe) · [article](/fr/posts/42/webserv/) | C++98 |

## Bas niveau et noyau

| Projet | Description | Technos |
|---|---|---|
| [libasm](https://github.com/PfClaKr/libasm-x86_64) | `strlen`, `strcpy`, `strcmp`, `strdup`, `read` et `write` en assembleur x86-64, selon l’ABI System V · [article](/fr/posts/42/libasm/) | NASM |
| [little-penguin-1](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin) | Exercices sur le noyau Linux : compilation d’un noyau, modules, hot-plug USB, périphériques misc, debugfs et procfs · [article](/fr/posts/42/little-penguin/) | C, Linux |
| [KFS 1–4](https://github.com/PfClaKr/KFS) | Un noyau de zéro : boot, GDT/IDT, interruptions, pagination, allocateur de tas et shell (en binôme) · [article](/fr/posts/42/kfs-rust-kernel/) | Rust |

## Sécurité

| Projet | Description | Technos |
|---|---|---|
| [RainFall](https://github.com/PfClaKr/Ecole42Paris_Rainfall) | 14 niveaux d’exploitation de binaires : rétro-ingénierie, débordements de pile et de tas, format strings · [article](/fr/posts/42/rainfall/) | GDB, Python |
| [OverRide](https://github.com/PfClaKr/Ecole42Paris_Override) | 10 niveaux supplémentaires, la suite de RainFall · [article](/fr/posts/42/override/) | GDB, Python |
| [ft_shield](https://github.com/PfClaKr/Ecole42Paris_ft_shield) | Un cheval de Troie pédagogique, à n’exécuter que dans une VM isolée, pour comprendre la persistance et les portes dérobées et savoir les repérer · [article](/fr/posts/42/ft-shield/) | C, systemd |

## Réseau

| Projet | Description | Technos |
|---|---|---|
| [ft_ping](https://github.com/PfClaKr/Ecole42Paris_ft_ping) | `ping` recodé avec des sockets ICMP brutes : checksum, TTL, délais et statistiques de RTT · [article](/fr/posts/42/ft-ping-raw-sockets/) | C |
| [BADASS](https://github.com/PfClaKr/Ecole42Paris_BADASS) | Des labos GNS3 avec FRRouting : VXLAN statique et multicast, puis une fabric spine-leaf BGP-EVPN avec route reflector · [article](/fr/posts/42/badass-bgp-evpn-vxlan/) | GNS3, FRR, Docker |

## DevOps et web

| Projet | Description | Technos |
|---|---|---|
| [Inception](https://github.com/PfClaKr/Ecole42Paris_Inception) | nginx (TLS), WordPress/PHP-FPM et MariaDB, chacun dans un conteneur écrit à la main · [article](/fr/posts/42/inception/) | Docker Compose |
| [Inception-of-Things](https://github.com/PfClaKr/Ecole42Paris_Inception_of_Things) | Un cluster K3s sur Vagrant, du routage Ingress par hôte, puis du GitOps avec Argo CD sur K3d (en binôme) · [article](/fr/posts/42/inception-of-things/) | Vagrant, K3s, Argo CD |
| [ft_transcendence](https://github.com/sleepychloe/ft_transcendance) | Une application web de Pong multijoueur en temps réel avec OAuth 42, un mode 3D, des tournois et des logs ELK (équipe de trois, 125/100) · [article](/fr/posts/42/ft-transcendence/) | Django, Channels, PostgreSQL, Three.js |
