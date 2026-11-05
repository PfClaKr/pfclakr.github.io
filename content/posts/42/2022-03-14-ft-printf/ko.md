+++
title = "ft_printf: 마법 없이 가변 인자 함수 이해하기"
description = "%c %s %p %d %i %u %x %X %%를 지원하는 printf 재구현: 형식 문자열을 훑고, va_arg로 인자를 꺼내고, 출력한 바이트를 하나도 빠짐없이 세기."
tags = ["42", "c"]
+++

libft 다음 과제는 **ft_printf**입니다. `%c`, `%s`, `%p`, `%d`, `%i`, `%u`, `%x`, `%X`, `%%` 변환을
지원하는 `printf`를 다시 만들고, 진짜 `printf`처럼 출력한 문자 수를 돌려줘야 합니다. 핵심은 서식이
아니라, C 함수가 어떻게 *개수가 정해지지 않은* 인자를 받는지 배우는 것입니다
([소스](https://github.com/PfClaKr/Ecole42Paris_ftprintf)).

## 가변 인자

C는 `format` 뒤에 인자가 몇 개 왔는지 모릅니다. `<stdarg.h>`는 그 인자들을 가리키는 커서를 줄 뿐이고,
얼마나 움직이고 어떤 타입으로 읽을지는 형식 문자열만이 알려 줍니다.

```c
int ft_printf(const char *format, ...)
{
    va_list ap;
    int     len = 0;
    int     i = 0;

    va_start(ap, format);          // 마지막 이름 있는 매개변수 바로 뒤에서 시작
    while (format[i])
    {
        if (format[i] == '%')
            len += ft_type(ap, format[++i]);
        else
            len += ft_putchar(format[i]);
        i++;
    }
    va_end(ap);
    return (len);
}
```

`ft_type`은 변환 문자에 따라 분기할 뿐입니다. 각 분기는 호출한 쪽이 *약속한* 타입으로 `va_arg`를
부릅니다.

```c
if (format == 'c')
    len += ft_putchar(va_arg(ap, int));          // char는 int로 승격되어 들어옴
else if (format == 'p')
    len += ft_putpointer(va_arg(ap, unsigned long));
else if (format == 'x' || format == 'X')
    len += ft_puthex(va_arg(ap, unsigned int), format);
```

여기에 두 가지가 숨어 있습니다.

- **기본 인자 승격.** `...`로 넘긴 `char`는 `int`로, `float`는 `double`로 도착합니다.
  `va_arg(ap, char)`는 정의되지 않은 동작입니다.
- **아무것도 검사하지 않습니다.** 형식은 `%s`인데 `int`를 넘기면 `va_arg`는 그걸 그대로 포인터로
  해석합니다. 컴파일러가 형식 문자열을 경고하는 이유이고, `printf(user_input)`이 보안 취약점인
  이유입니다([RainFall](/ko/posts/42/rainfall/) 참고).

## 숫자 출력

모든 숫자는 같은 재귀 아이디어를 씁니다. 윗자리를 먼저 출력하고 마지막 자리를 출력합니다. 16진수는
진법 문자열만 바꾸면 됩니다.

```c
int ft_puthex(unsigned int x, char format)
{
    if (x >= 16)
    {
        ft_puthex(x / 16, format);
        ft_puthex(x % 16, format);
    }
    else
        write(1, &(format == 'x' ? "0123456789abcdef" : "0123456789ABCDEF")[x], 1);
    return (ft_puthex_len(x, 0));
}
```

`%p`는 `unsigned long`에 `0x` 접두사를 붙인 같은 작업입니다. 평가 테스트가 노리는 건 특수한 경우들입니다.

| 입력 | 출력 |
|---|---|
| `NULL`을 `%s`로 | `(null)` |
| `INT_MIN`을 `%d`로 | `-2147483648`. `int` 안에서는 부호를 바꿀 수 없음 |
| `NULL`을 `%p`로 | macOS는 `0x0`, glibc는 `(nil)` |
| `-1`을 `%u`로 | `4294967295` |

## 반환값

보기보다 까다로운 건 반환값입니다. 모든 도우미 함수가 몇 바이트를 썼는지 알려 줘야 하고, 재귀를
거쳐도 합이 정확해야 합니다. 저는 타입마다 길이를 미리 계산하는 `*_len` 함수를 따로 두었는데,
재귀 속으로 카운터를 넘기는 것보다 맞게 만들기 쉬웠습니다.
