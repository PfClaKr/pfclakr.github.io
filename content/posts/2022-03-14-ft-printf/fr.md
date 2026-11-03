+++
title = "ft_printf : les fonctions variadiques sans magie"
description = "Recoder printf pour %c %s %p %d %i %u %x %X et %% : parcourir la chaîne de format, récupérer les arguments avec va_arg et compter chaque octet écrit."
tags = ["42", "c"]
+++

Après libft vient **ft_printf** : réécrire `printf` pour les conversions `%c`, `%s`, `%p`, `%d`,
`%i`, `%u`, `%x`, `%X` et `%%`, et renvoyer le nombre de caractères écrits, comme le vrai. Le
formatage n'est pas le sujet. Le sujet, c'est comprendre comment une fonction C peut recevoir
*n'importe quel nombre* d'arguments ([source](https://github.com/PfClaKr/Ecole42Paris_ftprintf)).

## Les arguments variadiques

Le C ne sait pas combien d'arguments ont été passés après `format`. `<stdarg.h>` fournit un curseur
sur ces arguments, et seule la chaîne de format dit de combien l'avancer et quel type lire.

```c
int ft_printf(const char *format, ...)
{
    va_list ap;
    int     len = 0;
    int     i = 0;

    va_start(ap, format);          // commence juste après le dernier paramètre nommé
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

`ft_type` est un simple aiguillage sur la lettre de conversion. Chaque branche appelle `va_arg` avec
le type que l'appelant *a promis* :

```c
if (format == 'c')
    len += ft_putchar(va_arg(ap, int));          // un char est promu en int
else if (format == 'p')
    len += ft_putpointer(va_arg(ap, unsigned long));
else if (format == 'x' || format == 'X')
    len += ft_puthex(va_arg(ap, unsigned int), format);
```

Deux détails s'y cachent :

- **Les promotions par défaut.** Un `char` passé via `...` arrive en `int`, un `float` en `double`.
  `va_arg(ap, char)` est un comportement indéfini.
- **Rien n'est vérifié.** Si le format dit `%s` et que l'appelant a passé un `int`, `va_arg` le
  réinterprète sans broncher comme un pointeur. C'est pour ça que les compilateurs avertissent sur
  les chaînes de format, et que `printf(saisie_utilisateur)` est une faille
  (voir [RainFall](/fr/posts/rainfall/)).

## Afficher des nombres

Tous les nombres suivent la même idée récursive : afficher d'abord les chiffres de poids fort, puis
le dernier. L'hexadécimal change juste la chaîne de base.

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

`%p`, c'est la même chose sur un `unsigned long` avec le préfixe `0x`. Les cas particuliers sont
exactement ce que les tests visent :

| Entrée | Sortie |
|---|---|
| `%s` avec `NULL` | `(null)` |
| `%d` avec `INT_MIN` | `-2147483648`, qu'on ne peut pas rendre positif dans un `int` |
| `%p` avec `NULL` | `0x0` sur macOS, `(nil)` avec la glibc |
| `%u` avec `-1` | `4294967295` |

## La valeur de retour

C'est la valeur de retour qui rend l'exercice plus délicat qu'il n'y paraît : chaque fonction doit
dire combien d'octets elle a écrits, et les comptes doivent tomber juste à travers la récursion.
J'ai gardé une fonction `*_len` par type qui calcule la longueur à l'avance, plus simple à rendre
correcte que de faire circuler un compteur dans la récursion.

C'est un petit projet, mais les fonctions variadiques reviennent dans chaque fonction de log qu'on
écrira, et savoir que `va_arg` vous fait aveuglément confiance change la façon de lire un `printf`.
