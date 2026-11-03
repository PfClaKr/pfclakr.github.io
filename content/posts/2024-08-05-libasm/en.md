+++
title = "libasm: back to libft, in x86-64 assembly"
description = "strlen, strcpy, strcmp, strdup, read and write in NASM: the System V calling convention, raw syscalls and errno, position-independent calls through the PLT, stack alignment, and a bug hiding in an error path."
tags = ["42", "assembly", "low-level"]
+++

**libasm** sends you back to the very first functions of libft, `strlen`, `strcpy`, `strcmp`,
`strdup`, plus `read` and `write`, and asks you to write them in 64-bit assembly (Intel syntax, NASM).
They must be callable from C like the real ones, which means respecting the calling convention to the
letter ([source](https://github.com/PfClaKr/libasm-x86_64)).

## The System V AMD64 ABI

On Linux x86-64, a C function call follows the System V ABI:

| | Registers |
|---|---|
| integer/pointer arguments, in order | `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` |
| return value | `rax` |
| the callee may destroy | `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`–`r11` |
| the callee must preserve | `rbx`, `rbp`, `r12`–`r15`, `rsp` |

So `ft_strlen(const char *s)` receives `s` in `rdi` and returns the length in `rax`. With nothing to
preserve, it's a counter and a loop:

```nasm
ft_strlen:
        xor     rcx, rcx              ; counter = 0 (xor with itself is the idiomatic zero)
.loop:
        cmp     byte [rdi + rcx], 0   ; s[i] == '\0' ?
        jz      .done
        inc     rcx
        jmp     .loop
.done:
        mov     rax, rcx              ; return value goes in rax
        ret
```

`strcmp` has one subtle point: the result must come from the bytes compared as **unsigned**. `movzx`
zero-extends each byte into a 64-bit register before subtracting, so `'\x80'` compares greater than
`'a'`, as the man page requires.

## Syscalls and errno

`ft_read` and `ft_write` call the kernel directly. The syscall number goes in `rax` (0 for `read`, 1 for
`write`), and the arguments are already in `rdi`, `rsi`, `rdx`, exactly where the C caller put them.

The interesting part is errors. The kernel doesn't set `errno`. It returns `-errno` in `rax`, for example
`-9` for `EBADF`. The libc wrapper's job is to turn that into `errno = 9; return -1;`, so ours has to do
the same. `errno` is thread-local, so you can't just write to a global symbol. You call
`__errno_location()`, which returns a pointer to the current thread's `errno`:

```nasm
ft_write:
        mov     rax, 1                ; syscall number: write
        syscall
        cmp     rax, 0
        jl      .error                ; negative → -errno
        ret
.error:
        neg     rax                   ; -9 → 9
        push    rax                   ; save it (and realign the stack, see below)
        call    __errno_location wrt ..plt
        pop     qword [rax]           ; *__errno_location() = 9
        mov     rax, -1
        ret
```

Two details in those few lines:

- **`wrt ..plt`.** Modern toolchains build position-independent executables by default. A direct call to
  a libc symbol can't be resolved at link time, so the call goes through the Procedure Linkage Table,
  which the dynamic linker fills in at load time.
- **Stack alignment.** The ABI requires `rsp` to be 16-byte aligned at every `call`. On entry to our
  function it's off by 8, because `call` pushed the return address. The `push rax` fixes the alignment
  *and* keeps the value safe across the call, since `rax` is caller-saved. Forgetting this works most
  of the time and then crashes inside libc on a `movaps`.

## strdup calls into C

`ft_strdup` chains everything: our `ft_strlen`, libc's `malloc`, then our `ft_strcpy`.

```nasm
ft_strdup:
        call    ft_strlen
        inc     rax                   ; + 1 for '\0'
        push    rdi                   ; save the source (rdi is caller-saved)
        mov     rdi, rax
        call    malloc wrt ..plt
        cmp     rax, 0
        jz      .error
        pop     rsi                   ; source → 2nd argument
        mov     rdi, rax              ; new buffer → 1st argument
        call    ft_strcpy             ; returns the buffer in rax
        ret
```

## The bug in the error path

Reading it again for this post, I found a bug that the tests never hit. On `malloc` failure, the code
jumps to `.error` **without popping** the `rdi` it pushed. The error path then pushes and pops its own
value, sets `rax = 0`, and executes `ret`, which pops the saved string pointer and jumps to it as if
it were a return address. A `NULL` from `malloc` turns into a crash instead of a `NULL` return.

The fix is one instruction (`pop rdi` or `add rsp, 8` before handling the error). It's also unnecessary
to set `errno = ENOMEM` by hand, because `malloc` already did. It's a good example of what assembly
takes away: no compiler checks that every path leaves the stack as it found it. An error path that
runs once in a million calls has to be just as correct as the happy path.

libasm is short, but after it, a disassembly listing stops being noise. Function prologues, `call`
through the PLT, and `movzx` before a comparison all read like familiar sentences.
