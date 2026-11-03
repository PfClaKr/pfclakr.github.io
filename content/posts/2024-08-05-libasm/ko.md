+++
title = "libasm: x86-64 어셈블리로 다시 만드는 libft"
description = "NASM으로 쓴 strlen, strcpy, strcmp, strdup, read, write: System V 호출 규약, 날것의 시스템 콜과 errno, PLT를 거치는 위치 독립 호출, 스택 정렬, 그리고 오류 경로에 숨은 버그."
tags = ["42", "assembly", "low-level"]
+++

**libasm**은 libft의 맨 처음 함수들인 `strlen`, `strcpy`, `strcmp`, `strdup`에 `read`와 `write`를 더해,
64비트 어셈블리(인텔 문법, NASM)로 다시 쓰게 합니다. 진짜 함수처럼 C에서 호출할 수 있어야 하니, 호출
규약을 글자 그대로 지켜야 합니다([소스](https://github.com/PfClaKr/libasm-x86_64)).

## System V AMD64 ABI

리눅스 x86-64에서 C 함수 호출은 System V ABI를 따릅니다.

| | 레지스터 |
|---|---|
| 정수/포인터 인자, 순서대로 | `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` |
| 반환값 | `rax` |
| 호출된 함수가 덮어써도 됨 | `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`–`r11` |
| 호출된 함수가 보존해야 함 | `rbx`, `rbp`, `r12`–`r15`, `rsp` |

그래서 `ft_strlen(const char *s)`는 `s`를 `rdi`로 받고 길이를 `rax`로 돌려줍니다. 보존할 것이 없으니
카운터와 반복문이면 됩니다.

```nasm
ft_strlen:
        xor     rcx, rcx              ; 카운터 = 0 (자기 자신과 xor는 관용적인 0)
.loop:
        cmp     byte [rdi + rcx], 0   ; s[i] == '\0' ?
        jz      .done
        inc     rcx
        jmp     .loop
.done:
        mov     rax, rcx              ; 반환값은 rax로
        ret
```

`strcmp`에는 미묘한 점이 하나 있습니다. 결과는 바이트를 **부호 없는** 값으로 비교해서 나와야 합니다.
`movzx`는 빼기 전에 각 바이트를 64비트 레지스터로 0 확장하므로, man 페이지가 요구하는 대로 `'\x80'`이
`'a'`보다 크게 비교됩니다.

## 시스템 콜과 errno

`ft_read`와 `ft_write`는 커널을 직접 부릅니다. 시스템 콜 번호는 `rax`에(`read`는 0, `write`는 1), 인자는
C 호출자가 넣어 둔 그대로 이미 `rdi`, `rsi`, `rdx`에 있습니다.

흥미로운 건 오류입니다. 커널은 `errno`를 설정하지 않습니다. `rax`에 `-errno`를 돌려줄 뿐입니다. 예를
들어 `EBADF`면 `-9`입니다. 이걸 `errno = 9; return -1;`로 바꾸는 게 libc 래퍼의 일이니, 저희 함수도 똑같이
해야 합니다. `errno`는 스레드 지역 변수라서 전역 심볼에 그냥 쓸 수 없습니다. 현재 스레드의 `errno`를
가리키는 포인터를 돌려주는 `__errno_location()`을 부릅니다.

```nasm
ft_write:
        mov     rax, 1                ; 시스템 콜 번호: write
        syscall
        cmp     rax, 0
        jl      .error                ; 음수 → -errno
        ret
.error:
        neg     rax                   ; -9 → 9
        push    rax                   ; 값 보관 (그리고 스택 재정렬, 아래 참고)
        call    __errno_location wrt ..plt
        pop     qword [rax]           ; *__errno_location() = 9
        mov     rax, -1
        ret
```

이 몇 줄에 두 가지가 숨어 있습니다.

- **`wrt ..plt`.** 요즘 툴체인은 기본으로 위치 독립 실행 파일을 만듭니다. libc 심볼을 직접 호출하면 링크
  시점에 주소를 정할 수 없으니, 동적 링커가 로드할 때 채워 주는 Procedure Linkage Table을 거쳐 호출합니다.
- **스택 정렬.** ABI는 모든 `call` 시점에 `rsp`가 16바이트로 정렬되어 있기를 요구합니다. 함수에 들어온
  직후에는 `call`이 반환 주소를 쌓았으니 8만큼 어긋나 있습니다. `push rax`는 정렬을 맞추는 *동시에*,
  `rax`는 보존되지 않는 레지스터이니 호출 동안 값을 지켜 줍니다. 이걸 잊어도 대부분은 동작하다가, 어느 날
  libc 안의 `movaps`에서 죽습니다.

## C를 부르는 strdup

`ft_strdup`은 모든 걸 엮습니다. 저희 `ft_strlen`, libc의 `malloc`, 그리고 저희 `ft_strcpy`.

```nasm
ft_strdup:
        call    ft_strlen
        inc     rax                   ; '\0'을 위해 + 1
        push    rdi                   ; 원본 보관 (rdi는 보존되지 않음)
        mov     rdi, rax
        call    malloc wrt ..plt
        cmp     rax, 0
        jz      .error
        pop     rsi                   ; 원본 → 두 번째 인자
        mov     rdi, rax              ; 새 버퍼 → 첫 번째 인자
        call    ft_strcpy             ; 버퍼를 rax로 돌려줌
        ret
```

## 오류 경로의 버그

이 글을 쓰려고 다시 읽다가 테스트가 한 번도 건드리지 않은 버그를 찾았습니다. `malloc`이 실패하면 코드는
쌓아 둔 `rdi`를 **꺼내지 않은 채** `.error`로 갑니다. 오류 경로는 자기 값을 쌓았다 꺼내고, `rax = 0`으로
설정한 뒤 `ret`을 실행하는데, 이때 저장해 둔 문자열 포인터가 꺼내져서 반환 주소인 것처럼 그리로 점프합니다.
`malloc`의 `NULL`이 `NULL` 반환이 아니라 크래시가 됩니다.

고치는 건 명령어 하나입니다(오류 처리 전에 `pop rdi`나 `add rsp, 8`). `errno = ENOMEM`을 직접 설정할
필요도 없습니다. `malloc`이 이미 했으니까요. 어셈블리가 무엇을 빼앗는지 보여 주는 좋은 예입니다. 모든
경로가 스택을 처음 상태로 돌려놓는지 검사해 주는 컴파일러가 없습니다. 백만 번에 한 번 도는 오류 경로도
정상 경로만큼 정확해야 합니다.

libasm은 짧지만, 하고 나면 디스어셈블 결과가 더는 소음으로 보이지 않습니다. 함수 프롤로그, PLT를 거치는
`call`, 비교 전의 `movzx`가 익숙한 문장처럼 읽힙니다.
