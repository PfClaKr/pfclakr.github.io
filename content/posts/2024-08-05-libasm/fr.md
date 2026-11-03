+++
title = "libasm : retour à libft, en assembleur x86-64"
description = "strlen, strcpy, strcmp, strdup, read et write en NASM : la convention d'appel System V, les appels système bruts et errno, les appels indépendants de la position via la PLT, l'alignement de la pile, et un bug caché dans un chemin d'erreur."
tags = ["42", "assembly", "low-level"]
+++

**libasm** renvoie aux toutes premières fonctions de libft, `strlen`, `strcpy`, `strcmp`, `strdup`,
plus `read` et `write`, et demande de les écrire en assembleur 64 bits (syntaxe Intel, NASM). Elles
doivent être appelables depuis le C comme les vraies, ce qui veut dire respecter la convention d'appel
à la lettre ([source](https://github.com/PfClaKr/libasm-x86_64)).

## L'ABI System V AMD64

Sous Linux x86-64, un appel de fonction C suit l'ABI System V :

| | Registres |
|---|---|
| arguments entiers/pointeurs, dans l'ordre | `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` |
| valeur de retour | `rax` |
| l'appelé peut écraser | `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`–`r11` |
| l'appelé doit préserver | `rbx`, `rbp`, `r12`–`r15`, `rsp` |

`ft_strlen(const char *s)` reçoit donc `s` dans `rdi` et renvoie la longueur dans `rax`. Rien à
préserver : un compteur et une boucle suffisent.

```nasm
ft_strlen:
        xor     rcx, rcx              ; compteur = 0 (xor avec soi-même : le zéro idiomatique)
.loop:
        cmp     byte [rdi + rcx], 0   ; s[i] == '\0' ?
        jz      .done
        inc     rcx
        jmp     .loop
.done:
        mov     rax, rcx              ; la valeur de retour va dans rax
        ret
```

`strcmp` a un point subtil : le résultat doit venir des octets comparés comme **non signés**. `movzx`
étend chaque octet avec des zéros dans un registre 64 bits avant la soustraction, donc `'\x80'` est plus
grand que `'a'`, comme l'exige la page de manuel.

## Appels système et errno

`ft_read` et `ft_write` appellent directement le noyau. Le numéro d'appel système va dans `rax` (0 pour
`read`, 1 pour `write`), et les arguments sont déjà dans `rdi`, `rsi`, `rdx`, exactement là où
l'appelant C les a mis.

La partie intéressante, ce sont les erreurs. Le noyau ne touche pas à `errno`. Il renvoie `-errno` dans
`rax`, par exemple `-9` pour `EBADF`. Le rôle de l'enveloppe de la libc est de transformer ça en
`errno = 9; return -1;`, donc la nôtre doit faire pareil. `errno` est local au thread : on ne peut pas
écrire dans un symbole global. On appelle `__errno_location()`, qui renvoie un pointeur vers l'`errno`
du thread courant :

```nasm
ft_write:
        mov     rax, 1                ; numéro d'appel système : write
        syscall
        cmp     rax, 0
        jl      .error                ; négatif → -errno
        ret
.error:
        neg     rax                   ; -9 → 9
        push    rax                   ; le garder (et réaligner la pile, voir plus bas)
        call    __errno_location wrt ..plt
        pop     qword [rax]           ; *__errno_location() = 9
        mov     rax, -1
        ret
```

Deux détails dans ces quelques lignes :

- **`wrt ..plt`.** Les chaînes de compilation modernes produisent par défaut des exécutables
  indépendants de la position. Un appel direct à un symbole de la libc ne peut pas être résolu à
  l'édition de liens, donc l'appel passe par la Procedure Linkage Table, que l'éditeur de liens
  dynamique remplit au chargement.
- **L'alignement de la pile.** L'ABI exige que `rsp` soit aligné sur 16 octets à chaque `call`. À
  l'entrée de notre fonction, il est décalé de 8, parce que `call` a empilé l'adresse de retour. Le
  `push rax` corrige l'alignement *et* protège la valeur pendant l'appel, puisque `rax` n'est pas
  préservé. L'oublier marche la plupart du temps, puis plante dans la libc sur un `movaps`.

## strdup appelle du C

`ft_strdup` enchaîne tout : notre `ft_strlen`, le `malloc` de la libc, puis notre `ft_strcpy`.

```nasm
ft_strdup:
        call    ft_strlen
        inc     rax                   ; + 1 pour '\0'
        push    rdi                   ; garder la source (rdi n'est pas préservé)
        mov     rdi, rax
        call    malloc wrt ..plt
        cmp     rax, 0
        jz      .error
        pop     rsi                   ; source → 2e argument
        mov     rdi, rax              ; nouveau buffer → 1er argument
        call    ft_strcpy             ; renvoie le buffer dans rax
        ret
```

## Le bug du chemin d'erreur

En le relisant pour cet article, j'ai trouvé un bug que les tests n'ont jamais déclenché. Si `malloc`
échoue, le code saute à `.error` **sans dépiler** le `rdi` empilé. Le chemin d'erreur empile et dépile
sa propre valeur, met `rax = 0`, puis exécute `ret`, qui dépile le pointeur de chaîne sauvegardé et y
saute comme si c'était une adresse de retour. Un `NULL` de `malloc` devient un plantage au lieu d'un
retour `NULL`.

La correction tient en une instruction (`pop rdi` ou `add rsp, 8` avant de traiter l'erreur). Mettre
`errno = ENOMEM` à la main est aussi inutile, puisque `malloc` l'a déjà fait. C'est un bon exemple de ce
que l'assembleur retire : aucun compilateur ne vérifie que chaque chemin laisse la pile comme il l'a
trouvée. Un chemin d'erreur qui s'exécute une fois sur un million doit être aussi juste que le chemin
normal.

libasm est court, mais après, un listing désassemblé n'est plus du bruit. Les prologues de fonctions,
les `call` via la PLT et les `movzx` avant une comparaison se lisent comme des phrases familières.
