+++
title = "42 Paris"
description = "École 42 Paris에서 완료한 모든 프로젝트 — C로 만든 libft부터 Rust 커널, BGP-EVPN 실습, 쿠버네티스까지 — 코드와 상세 글 모음."
in_menu = true
+++

[École 42](https://42.fr)에는 교수도 강의도 없습니다. 프로젝트를 직접 만들면서 배우고,
모든 프로젝트는 다른 학생들이 대면으로 평가합니다.
아래는 제가 **42 Paris**(로그인 `ychun`)에서 통과한 프로젝트를 주제별로 정리한 것입니다.
이름을 누르면 해당 저장소로 이동하고, 전체 목록은
[PfClaKr/Ecole42Paris](https://github.com/PfClaKr/Ecole42Paris)에 있습니다.

## 상세 글

- [libft — 나만의 C 표준 라이브러리 만들기](/ko/posts/libft/)
- [get_next_line — 파일을 한 줄씩 읽기](/ko/posts/get-next-line/)
- [KFS — Rust로 x86 커널을 처음부터 만들기](/ko/posts/kfs-rust-kernel/)
- [BADASS — VXLAN과 BGP-EVPN으로 L3 위에 L2 얹기](/ko/posts/badass-bgp-evpn-vxlan/)
- [ft_ping — raw ICMP 소켓으로 ping 다시 만들기](/ko/posts/ft-ping-raw-sockets/)

## C 기초

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [libft](https://github.com/PfClaKr/Ecole42Paris_libft) | 문자열·메모리·연결 리스트 함수를 직접 구현한 C 표준 라이브러리. 이후 프로젝트에서 계속 재사용 · [상세 글](/ko/posts/libft/) | C |
| [ft_printf](https://github.com/PfClaKr/Ecole42Paris_ftprintf) | 가변 인자로 `printf` 재구현 | C |
| [get_next_line](https://github.com/PfClaKr/Ecole42Paris_GNL) | static 버퍼로 파일 디스크립터를 한 줄씩 읽기. 보너스는 여러 fd 동시 처리 · [상세 글](/ko/posts/get-next-line/) | C |
| [push_swap](https://github.com/PfClaKr/Ecole42Paris_PushSwap) | 스택 두 개와 제한된 연산만으로 정수를 최소 횟수로 정렬. 피벗 기반 분할 + `checker` 구현 | C |
| [so_long](https://github.com/PfClaKr/Ecole42Paris_Solong) | MiniLibX로 렌더링하는 작은 2D 게임 | C, MiniLibX |

## 유닉스 프로세스와 동시성

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [minitalk](https://github.com/PfClaKr/Ecole42Paris_Minitalk) | `SIGUSR1`/`SIGUSR2` 시그널만으로 문자열을 비트 단위로 주고받는 클라이언트/서버 | C, 시그널 |
| [minishell](https://github.com/PfClaKr/Ecole42Paris_MiniShell) | bash를 닮은 셸: 파싱, 파이프, 리다이렉션, heredoc, 환경 변수, 빌트인 (2인 팀) | C |
| [Philosophers](https://github.com/PfClaKr/Ecole42Paris_Philosophers) | 스레드와 뮤텍스로 푸는 식사하는 철학자 문제 | C, pthreads |

## 그래픽스

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [miniRT](https://github.com/PfClaKr/Ecole42Paris_miniRT) | 레이 트레이서: 구·평면·원기둥·원뿔, 퐁(Phong) 조명과 그림자. 보너스로 체커보드·범프·이미지 텍스처 | C, MiniLibX |

## C++

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [CPP 모듈 00–09](https://github.com/PfClaKr/Ecole42Paris_CppModule) | 클래스와 다형성부터 템플릿, STL 컨테이너까지. 마지막은 비트코인 환율 계산기, RPN 계산기, Ford-Johnson 병합 삽입 정렬 | C++98 |
| [webserv](https://github.com/PfClaKr/Ecole42Paris_Webserv) | nginx 스타일 설정, 논블로킹 I/O, 정적 파일, 업로드, CGI를 지원하는 HTTP/1.1 서버 (팀 프로젝트) | C++98 |

## 저수준과 커널

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [libasm](https://github.com/PfClaKr/libasm-x86_64) | System V ABI에 맞춰 x86-64 어셈블리로 작성한 `strlen`, `strcpy`, `strcmp`, `strdup`, `read`, `write` | NASM |
| [little-penguin-1](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin) | 리눅스 커널 과제: 커스텀 커널 빌드, 모듈, USB 핫플러그, misc 디바이스, debugfs, procfs | C, Linux |
| [KFS 1–4](https://github.com/PfClaKr/KFS) | 처음부터 만든 커널: 부팅, GDT/IDT, 인터럽트, 페이징, 힙 할당자, 셸 (2인 팀) · [상세 글](/ko/posts/kfs-rust-kernel/) | Rust |

## 보안

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [RainFall](https://github.com/PfClaKr/Ecole42Paris_Rainfall) | 바이너리 익스플로잇 14단계: 리버스 엔지니어링, 스택·힙 오버플로, 포맷 스트링 | GDB, Python |
| [OverRide](https://github.com/PfClaKr/Ecole42Paris_Override) | RainFall의 후속 과제, 익스플로잇 10단계 | GDB, Python |
| [ft_shield](https://github.com/PfClaKr/Ecole42Paris_ft_shield) | 격리된 VM 전용 교육용 트로이 목마. 지속성(persistence)과 백도어가 어떻게 동작하고 어떻게 찾아내는지 이해하기 위한 과제 | C, systemd |

## 네트워크

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [ft_ping](https://github.com/PfClaKr/Ecole42Paris_ft_ping) | raw ICMP 소켓으로 `ping` 재구현: 체크섬, TTL, 타임아웃, RTT 통계 · [상세 글](/ko/posts/ft-ping-raw-sockets/) | C |
| [BADASS](https://github.com/PfClaKr/Ecole42Paris_BADASS) | FRRouting을 쓴 GNS3 실습: 정적·멀티캐스트 VXLAN, 그리고 Route Reflector를 둔 BGP-EVPN 스파인-리프 구조 · [상세 글](/ko/posts/badass-bgp-evpn-vxlan/) | GNS3, FRR, Docker |

## DevOps와 웹

| 프로젝트 | 내용 | 기술 |
|---|---|---|
| [Inception](https://github.com/PfClaKr/Ecole42Paris_Inception) | nginx(TLS), WordPress/PHP-FPM, MariaDB를 각각 직접 작성한 컨테이너로 구성 | Docker Compose |
| [Inception-of-Things](https://github.com/PfClaKr/Ecole42Paris_Inception_of_Things) | Vagrant 위 K3s 클러스터, 호스트 기반 Ingress 라우팅, K3d 위 Argo CD로 GitOps (2인 팀) | Vagrant, K3s, Argo CD |
| [ft_transcendence](https://github.com/sleepychloe/ft_transcendance) | 42 OAuth, 3D 모드, 토너먼트, ELK 로깅을 갖춘 실시간 멀티플레이 퐁 웹앱 (3인 팀, 125/100) | Django, Channels, PostgreSQL, Three.js |
