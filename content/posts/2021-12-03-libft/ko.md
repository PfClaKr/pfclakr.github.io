+++
title = "libft: 나만의 C 표준 라이브러리 만들기"
description = "42 공통 과정의 첫 프로젝트: libc의 문자열·메모리 함수와 연결 리스트 API를 직접 구현하면서 만나는 까다로운 부분들."
tags = ["42", "c"]
+++

**libft**는 42 공통 과정의 첫 프로젝트입니다. C 표준 라이브러리 일부를 다시 작성하고, libc에 없는 도우미
함수를 몇 개 더한 뒤, 정적 라이브러리 `libft.a`로 묶습니다. 그 뒤로는 진짜 libc 대신 이걸 다음
프로젝트에서 계속 재사용해야 합니다. 제 libft도 `push_swap`, `so_long`, `minishell`에서 다시 쓰였습니다.

[저장소](https://github.com/PfClaKr/Ecole42Paris_libft)에는 함수 43개가 세 묶음으로 들어 있습니다.

| 묶음 | 함수 |
|---|---|
| libc 재구현 | `memset`, `bzero`, `memcpy`, `memmove`, `memchr`, `memcmp`, `strlen`, `strlcpy`, `strlcat`, `strchr`, `strrchr`, `strncmp`, `strnstr`, `atoi`, `calloc`, `strdup`, `isalpha`… |
| 추가 문자열 함수 | `substr`, `strjoin`, `strtrim`, `split`, `itoa`, `strmapi`, `striteri`, `putstr_fd`… |
| 연결 리스트 (보너스) | `lstnew`, `lstadd_front`, `lstadd_back`, `lstsize`, `lstlast`, `lstdelone`, `lstclear`, `lstiter`, `lstmap` |

## 까다로운 함수들

대부분은 반복문 하나면 끝나지만, 몇몇은 제대로 배울 거리가 있습니다.

**`memmove`와 `memcpy`.** 둘 다 `n`바이트를 복사하지만, `memmove`는 두 영역이 겹쳐도 동작해야 합니다.
핵심은 복사 방향입니다. 목적지가 원본보다 앞에 있으면 앞에서부터, 아니면 뒤에서부터 복사해서 아직
읽지 않은 바이트를 덮어쓰지 않게 합니다.

```c
if (dest < src)
    while (n--)
        *new_dest++ = *new_src++;      // 앞에서 뒤로
else
{
    new_dest = (unsigned char *)dest + (n - 1);
    new_src = (unsigned char *)src + (n - 1);
    while (n--)
        *new_dest-- = *new_src--;      // 뒤에서 앞으로
}
```

**`strlcpy`와 `strlcat`.** 표준 C가 아니라 BSD에서 온 함수입니다. 항상 NUL로 끝내 주고, *만들려고 했던*
문자열의 길이를 돌려줍니다. 그래서 호출하는 쪽은 `if (ret >= size)`로 잘림 여부를 알 수 있습니다.

**`split`.** 새로 할당한 문자열들의 배열을 돌려주기 때문에, 중간에 할당이 실패할 수 있습니다. 그러면
`NULL`을 돌려주기 전에 그때까지 할당한 것을 전부 해제해야 합니다.

```c
new_s[i] = ft_split_strdup(s, c);
if (!new_s[i])
    return (ft_split_free(new_s));   // 이미 복사한 단어들이 새지 않게
```

**`calloc`.** `nmemb * size`가 오버플로하면 아주 작은 버퍼가 조용히 할당될 수 있으니, 이 곱셈은 검사할
가치가 있습니다.

## 정적 라이브러리 만들기

정적 라이브러리는 오브젝트 파일을 모아 둔 아카이브일 뿐입니다.

```makefile
NAME = libft.a
AR   = ar rc

$(NAME): $(OBJS)
	$(AR) $(NAME) $(OBJS)
```

링크할 때(`-L. -lft`)는 프로그램이 실제로 쓰는 오브젝트 파일만 복사됩니다.

## 배운 것

- **엣지 케이스가 곧 명세입니다.** `n == 0`, 겹치는 버퍼, 빈 문자열, `itoa`의 `INT_MIN`. 동료 평가와 테스터는
  바로 이런 곳을 노립니다.
- **모든 `malloc`에는 주인이 있어야 합니다.** 누가 해제하는지, 중간에 실패하면 어떻게 되는지.
- **라이브러리는 계약입니다.** 다음 프로젝트들은 이 함수들이 man 페이지와 똑같이 동작한다고 믿고 씁니다.
