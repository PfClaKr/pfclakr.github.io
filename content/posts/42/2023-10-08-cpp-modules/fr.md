+++
title = "CPP Modules 00–09 : apprendre le C++98 à la dure"
description = "Dix modules, des classes à la STL : la forme canonique orthodoxe, les nombres à virgule fixe, les destructeurs virtuels, les quatre casts, et un tri par fusion-insertion de Ford-Johnson construit sur un itérateur maison."
tags = ["42", "cpp"]
+++

Après un an de C, 42 passe au C++ avec dix petits modules, **CPP 00 à 09**. Tout est en C++98 avec
`-Wall -Wextra -Werror -std=c++98` : pas d'`auto`, pas de pointeurs intelligents, pas de sémantique de
déplacement. Ça ressemble à un handicap, mais ça oblige à comprendre ce que le C++ moderne automatise
([source](https://github.com/PfClaKr/Ecole42Paris_CppModule)).

| Module | Sujet |
|---|---|
| 00 | namespaces, classes, `iostream` |
| 01 | allocation mémoire, références, pointeurs sur membres |
| 02 | surcharge d'opérateurs, forme canonique orthodoxe, virgule fixe |
| 03 | héritage |
| 04 | polymorphisme, classes abstraites, interfaces |
| 05 | exceptions |
| 06 | les casts du C++ |
| 07 | templates |
| 08 | conteneurs templates, itérateurs, algorithmes |
| 09 | la STL en pratique |

Voici ce qui m'est resté.

## La forme canonique orthodoxe

À partir du module 02, chaque classe doit déclarer quatre membres : un constructeur par défaut, un
constructeur de copie, un opérateur d'affectation par copie et un destructeur. En C++98, les versions
générées par le compilateur copient les pointeurs **superficiellement**, donc une classe qui possède de
la mémoire sur le tas est libérée deux fois dès qu'on la copie. Écrire les quatre à la main oblige à
décider, à chaque fois, ce que copier une ressource veut dire.

L'opérateur d'affectation a son propre piège : `a = a`. S'il libère son buffer avant de copier depuis
`ref`, et que `ref` est lui-même, il vient de détruire sa propre source. D'où la garde bien connue :

```cpp
Fixed &Fixed::operator=(const Fixed &ref)
{
    if (this != &ref)
        this->raw = ref.getRawBits();
    return *this;
}
```

## Les nombres à virgule fixe

Le module 02 construit une classe `Fixed` : un `int` dont les 8 bits de poids faible sont la partie
fractionnaire. Convertir, c'est décaler :

```cpp
Fixed::Fixed(const int a)   : raw(a << frac) {}                    // 5   → 1280
Fixed::Fixed(const float a) : raw(roundf(a * (1 << frac))) {}      // 1.5 → 384
float Fixed::toFloat() const { return (float)raw / (1 << frac); }
```

L'addition est une addition entière. La multiplication est plus subtile, parce que les échelles se
multiplient aussi : il faut redécaler le produit brut, `(a * b) >> 8`. Ma version prend le chemin facile
et passe par `toFloat()` aller-retour, ce qui est correct mais renonce à l'intérêt de la virgule fixe. C'est la première fois que la surcharge d'opérateurs
paraît utile plutôt que maligne : une fois `+ - * / < ==` et `<<` définis, `Fixed` se lit comme un type
natif.

## Les destructeurs virtuels

Le module 04 est celui où le polymorphisme mord. Supprimez un `Dog` via un `Animal *` : si
`~Animal()` n'est pas `virtual`, seule la partie `Animal` est détruite. Le `Brain` du chien fuit, sans
bruit. La règle est simple : si une classe a une fonction virtuelle, son destructeur doit l'être aussi.

Le même module demande une **copie profonde** de `Dog`, `Brain` compris, pour que copier un chien ne
donne pas deux chiens avec un seul cerveau. C'est là que la forme canonique rapporte.

## Quatre casts au lieu d'un

Le C a un seul cast, qui fait tout. Le C++ le découpe selon l'intention :

- `static_cast` : les conversions que le compilateur peut vérifier (`double` vers `int`, montée et
  descente dans une hiérarchie dont on est sûr) ;
- `reinterpret_cast` : « traite ces bits comme autre chose », utilisé au module 06 pour transformer un
  pointeur en `uintptr_t` et retour ;
- `const_cast` : retirer `const`, presque toujours le signe d'un problème de conception ;
- `dynamic_cast` : une descente **vérifiée** grâce aux informations de type à l'exécution.

Le module 06 demande d'identifier le vrai type derrière un `Base *` sans `typeid`. Avec des pointeurs,
un `dynamic_cast` raté renvoie `NULL`. Avec des références, il n'existe pas de référence nulle, donc il
lance une exception :

```cpp
void identify(Base &p)
{
    try { (void)dynamic_cast<A &>(p); std::cout << "A\n"; return; } catch (...) {}
    try { (void)dynamic_cast<B &>(p); std::cout << "B\n"; return; } catch (...) {}
    try { (void)dynamic_cast<C &>(p); std::cout << "C\n"; return; } catch (...) {}
}
```

## Module 09 : trois programmes

Le dernier module récompense tout ce qui précède, avec trois exercices qui utilisent chacun un
conteneur différent (on ne peut pas en réutiliser un).

**Bitcoin exchange** (`std::map`). À partir d'un CSV de prix journaliers et d'un fichier de lignes
`date | montant`, afficher la valeur de chaque montant. Si la date n'est pas dans la base, prendre la
date **antérieure** la plus proche. Une map triée en fait un seul appel : `lower_bound(date)` renvoie
la première entrée qui n'est pas avant la date, donc reculer d'un cran donne le jour précédent.

**RPN** (`std::stack`). Évaluer `"8 9 * 9 - 9 - 9 - 4 - 1 +"` : empiler les nombres, et sur un
opérateur dépiler deux valeurs, l'appliquer, empiler le résultat. L'intérêt est dans la gestion
d'erreurs : opérandes manquants, division par zéro, valeurs restantes à la fin.

**PmergeMe**. Trier une suite avec l'algorithme de **fusion-insertion de Ford-Johnson**, deux fois,
avec `std::vector` puis `std::deque`, et chronométrer les deux. Ford-Johnson minimise les
*comparaisons*, pas le temps :

1. Former des paires et comparer chaque paire. Les plus grands forment la « chaîne principale ».
2. Trier la chaîne principale récursivement.
3. Y insérer les plus petits par recherche dichotomique, dans un ordre donné par les **nombres de
   Jacobsthal** (1, 3, 5, 11, 21, …), choisis pour que chaque recherche porte sur une plage dont la taille
   est juste sous une puissance de deux, là où la dichotomie ne gaspille aucune comparaison.

Le difficile, c'est de le faire sans tout recopier à chaque niveau de récursion. Ma version utilise un
petit template `GroupIterator` qui fait passer une plage pour une suite de *groupes* de `size` éléments,
comparés par leur dernier élément. Échanger deux « éléments » échange des groupes entiers, donc la
récursion se fait en place, et le même code marche pour les deux conteneurs :

```cpp
template <typename Iterator>
typename std::iterator_traits<Iterator>::reference
GroupIterator<Iterator>::operator*() const
{
    return this->iter[this->size - 1];   // un groupe est représenté par son dernier élément
}
```
