+++
title = "libft : écrire ma propre bibliothèque standard C"
description = "Le premier projet du tronc commun de 42 : recoder des fonctions de chaînes et de mémoire de la libc, plus une API de listes chaînées, et les détails qui les rendent délicates."
tags = ["42", "c"]
+++

**libft** est le premier projet du tronc commun de 42. On réécrit une partie de la bibliothèque
standard C, on ajoute quelques fonctions absentes de la libc, et on emballe le tout dans une
bibliothèque statique, `libft.a`. Ensuite, on est censé la réutiliser dans les projets suivants à la
place de la vraie libc. La mienne réapparaît dans `push_swap`, `so_long` et `minishell`.

Le [dépôt](https://github.com/PfClaKr/Ecole42Paris_libft) contient 43 fonctions réparties en trois
groupes.

| Groupe | Fonctions |
|---|---|
| réécritures de la libc | `memset`, `bzero`, `memcpy`, `memmove`, `memchr`, `memcmp`, `strlen`, `strlcpy`, `strlcat`, `strchr`, `strrchr`, `strncmp`, `strnstr`, `atoi`, `calloc`, `strdup`, `isalpha`… |
| fonctions de chaînes en plus | `substr`, `strjoin`, `strtrim`, `split`, `itoa`, `strmapi`, `striteri`, `putstr_fd`… |
| listes chaînées (bonus) | `lstnew`, `lstadd_front`, `lstadd_back`, `lstsize`, `lstlast`, `lstdelone`, `lstclear`, `lstiter`, `lstmap` |

## Les plus délicates

La plupart des fonctions se résument à une boucle, mais quelques-unes apprennent vraiment quelque
chose.

**`memmove` contre `memcpy`.** Les deux copient `n` octets, mais `memmove` doit fonctionner quand
les deux zones se chevauchent. L’astuce est le sens de copie : si la destination est avant la
source, on copie vers l’avant ; sinon on copie vers l’arrière, pour ne jamais écraser des octets pas
encore lus.

```c
if (dest < src)
    while (n--)
        *new_dest++ = *new_src++;      // du début vers la fin
else
{
    new_dest = (unsigned char *)dest + (n - 1);
    new_src = (unsigned char *)src + (n - 1);
    while (n--)
        *new_dest-- = *new_src--;      // de la fin vers le début
}
```

**`strlcpy` et `strlcat`.** Elles viennent de BSD et non du C standard. Elles terminent toujours la
chaîne par un NUL et renvoient la longueur de la chaîne qu’elles ont *essayé* de créer : l’appelant
détecte la troncature avec `if (ret >= size)`.

**`split`.** Elle renvoie un tableau de chaînes nouvellement allouées, donc une allocation peut
échouer en plein milieu. Dans ce cas, tout ce qui a déjà été alloué doit être libéré avant de
renvoyer `NULL` :

```c
new_s[i] = ft_split_strdup(s, c);
if (!new_s[i])
    return (ft_split_free(new_s));   // ne pas perdre les mots déjà copiés
```

**`calloc`.** `nmemb * size` peut déborder et allouer silencieusement un tout petit buffer : cette
multiplication mérite une vérification.

## Construire une bibliothèque statique

Une bibliothèque statique n’est qu’une archive de fichiers objets :

```makefile
NAME = libft.a
AR   = ar rc

$(NAME): $(OBJS)
	$(AR) $(NAME) $(OBJS)
```

À l’édition de liens (`-L. -lft`), seuls les fichiers objets réellement utilisés par le programme
sont copiés.

## Ce qu’on en retient

- **Les cas limites sont la spécification.** `n == 0`, buffers qui se chevauchent, chaînes vides,
  `INT_MIN` dans `itoa` : l’évaluation par les pairs et les testeurs visent directement ces cas.
- **Chaque `malloc` a besoin d’un propriétaire.** Qui le libère, et que se passe-t-il s’il échoue
  en plein milieu ?
- **Une bibliothèque est un contrat.** Les projets suivants comptent sur ces fonctions pour se
  comporter exactement comme leurs pages de manuel.
