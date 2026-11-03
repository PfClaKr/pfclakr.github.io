+++
title = "ft_printf: variadic functions without the magic"
description = "Re-implementing printf for %c %s %p %d %i %u %x %X and %%: walking the format string, pulling arguments with va_arg, and counting every byte written."
tags = ["42", "c"]
+++

After libft, the next step is **ft_printf**: rewrite `printf` for the conversions `%c`, `%s`, `%p`,
`%d`, `%i`, `%u`, `%x`, `%X` and `%%`, and return the number of characters written, exactly like the
real one. The point isn't formatting. It's learning how a C function can take *any number* of
arguments ([source](https://github.com/PfClaKr/Ecole42Paris_ftprintf)).

## Variadic arguments

C has no idea how many arguments were passed after `format`. `<stdarg.h>` gives you a cursor over
them, and the format string is the only thing that says how far to move it and what type to read.

```c
int ft_printf(const char *format, ...)
{
    va_list ap;
    int     len = 0;
    int     i = 0;

    va_start(ap, format);          // start right after the last named parameter
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

`ft_type` is a plain dispatch on the conversion letter. Each branch calls `va_arg` with the type
the *caller promised*:

```c
if (format == 'c')
    len += ft_putchar(va_arg(ap, int));          // char is promoted to int
else if (format == 'p')
    len += ft_putpointer(va_arg(ap, unsigned long));
else if (format == 'x' || format == 'X')
    len += ft_puthex(va_arg(ap, unsigned int), format);
```

Two details hide in there:

- **Default promotions.** A `char` passed through `...` arrives as an `int`, and a `float` as a
  `double`. `va_arg(ap, char)` is undefined behaviour.
- **Nothing is checked.** If the format says `%s` and the caller passed an `int`, `va_arg` happily
  reinterprets it as a pointer. That's why compilers warn about format strings and why
  `printf(user_input)` is a security bug (see [RainFall](/posts/rainfall/)).

## Printing numbers

Every number goes through the same recursive idea: print the higher digits first, then the last one.
Hex just swaps the base string.

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

`%p` is the same thing on an `unsigned long` with a `0x` prefix. The special cases are where the
grading tests look:

| Input | Output |
|---|---|
| `%s` with `NULL` | `(null)` |
| `%d` with `INT_MIN` | `-2147483648`, which can't be negated in an `int` |
| `%p` with `NULL` | `0x0` on macOS, `(nil)` with glibc |
| `%u` with `-1` | `4294967295` |

## The return value

The return value is what makes this harder than it looks: every helper has to report how many
bytes it wrote, and the counts have to add up across recursion. I kept a separate `*_len` helper
per type that computes the length up front, which is easier to get right than threading a counter
through the recursion.

It's a small project, but variadic functions show up again in every logging function you'll ever
write, and knowing that `va_arg` trusts you blindly changes how you read `printf` calls.
