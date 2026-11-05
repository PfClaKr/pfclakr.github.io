+++
title = "libft: writing my own C standard library"
description = "The first project of the 42 common core: re-implementing libc string and memory functions plus a linked-list API, and the details that make them subtle."
tags = ["42", "c"]
+++

**libft** is the first project of the 42 common core. You rewrite a slice of the C standard
library, add a few helpers that libc lacks, and package everything as a static library,
`libft.a`. From then on you are expected to reuse it in later projects instead of the real libc.
Mine shows up again in `push_swap`, `so_long` and `minishell`.

The [repository](https://github.com/PfClaKr/Ecole42Paris_libft) contains 43 functions in three
groups.

| Group | Functions |
|---|---|
| libc rewrites | `memset`, `bzero`, `memcpy`, `memmove`, `memchr`, `memcmp`, `strlen`, `strlcpy`, `strlcat`, `strchr`, `strrchr`, `strncmp`, `strnstr`, `atoi`, `calloc`, `strdup`, `isalpha`… |
| extra string helpers | `substr`, `strjoin`, `strtrim`, `split`, `itoa`, `strmapi`, `striteri`, `putstr_fd`… |
| linked list (bonus) | `lstnew`, `lstadd_front`, `lstadd_back`, `lstsize`, `lstlast`, `lstdelone`, `lstclear`, `lstiter`, `lstmap` |

## The subtle ones

Most functions are a loop, but a few teach something real.

**`memmove` vs `memcpy`.** Both copy `n` bytes, but `memmove` must work when the two regions
overlap. The trick is the copy direction: if the destination comes before the source, copy forward;
otherwise copy backward, so you never overwrite bytes you haven't read yet.

```c
if (dest < src)
    while (n--)
        *new_dest++ = *new_src++;      // front to back
else
{
    new_dest = (unsigned char *)dest + (n - 1);
    new_src = (unsigned char *)src + (n - 1);
    while (n--)
        *new_dest-- = *new_src--;      // back to front
}
```

**`strlcpy` and `strlcat`.** They come from BSD rather than standard C. They always NUL-terminate
and return the length of the string they *tried* to create, so the caller can detect truncation
with `if (ret >= size)`.

**`split`.** It returns an array of freshly allocated strings, so any allocation can fail halfway
through. When that happens, everything allocated so far must be freed before returning `NULL`:

```c
new_s[i] = ft_split_strdup(s, c);
if (!new_s[i])
    return (ft_split_free(new_s));   // don't leak the words already copied
```

**`calloc`.** `nmemb * size` can overflow and silently allocate a tiny buffer, so that
multiplication deserves a check.

## Building a static library

A static library is just an archive of object files:

```makefile
NAME = libft.a
AR   = ar rc

$(NAME): $(OBJS)
	$(AR) $(NAME) $(OBJS)
```

Linking against it (`-L. -lft`) copies only the object files a program actually uses.
