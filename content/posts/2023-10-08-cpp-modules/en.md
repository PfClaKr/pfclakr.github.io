+++
title = "CPP Modules 00–09: learning C++98 the hard way"
description = "Ten modules from classes to the STL: the orthodox canonical form, fixed-point numbers, virtual destructors, the four casts, and a Ford-Johnson merge-insert sort built on a custom iterator."
tags = ["42", "cpp"]
+++

After a year of C, 42 switches to C++ with ten short modules, **CPP 00 to 09**. They're all in C++98
with `-Wall -Wextra -Werror -std=c++98`: no `auto`, no smart pointers, no move semantics. That sounds
like a handicap, but it forces you to understand what modern C++ automates
([source](https://github.com/PfClaKr/Ecole42Paris_CppModule)).

| Module | Topic |
|---|---|
| 00 | namespaces, classes, `iostream` |
| 01 | memory allocation, references, pointers to members |
| 02 | operator overloading, the orthodox canonical form, fixed-point numbers |
| 03 | inheritance |
| 04 | polymorphism, abstract classes, interfaces |
| 05 | exceptions |
| 06 | the C++ casts |
| 07 | templates |
| 08 | templated containers, iterators, algorithms |
| 09 | the STL in practice |

Here are the parts that stuck.

## The orthodox canonical form

From module 02 on, every class must declare four members: a default constructor, a copy constructor,
a copy assignment operator, and a destructor. In C++98, the compiler-generated versions copy pointers
**shallowly**, so a class that owns heap memory gets freed twice the moment it's copied. Writing all
four by hand makes you decide, every time, what copying a resource means.

The assignment operator has its own trap: `a = a`. If it frees its buffer before copying from `ref`,
and `ref` is itself, it has just destroyed its own source. Hence the familiar guard:

```cpp
Fixed &Fixed::operator=(const Fixed &ref)
{
    if (this != &ref)
        this->raw = ref.getRawBits();
    return *this;
}
```

## Fixed-point numbers

Module 02 builds a `Fixed` class: an `int` where the low 8 bits are the fraction. Converting is
shifting:

```cpp
Fixed::Fixed(const int a)   : raw(a << frac) {}                    // 5   → 1280
Fixed::Fixed(const float a) : raw(roundf(a * (1 << frac))) {}      // 1.5 → 384
float Fixed::toFloat() const { return (float)raw / (1 << frac); }
```

Addition is plain integer addition. Multiplication is subtler, because the scales multiply too: the
raw product has to be shifted back, `(a * b) >> 8`. My version takes the easy route and goes through
`toFloat()` and back, which is correct but gives up the point of fixed-point arithmetic. It's the first time operator overloading feels useful rather than clever: once
`+ - * / < ==` and `<<` are defined, `Fixed` reads like a built-in type.

## Virtual destructors

Module 04 is where polymorphism bites. Delete a `Dog` through an `Animal *`, and if `~Animal()` isn't
`virtual`, only the `Animal` part is destroyed. The `Dog`'s `Brain` leaks, silently. The rule is
simple: if a class has any virtual function, its destructor must be virtual too.

The same module asks for a **deep copy** of `Dog`, including its `Brain`, so that copying a dog doesn't
give two dogs one brain. That's the canonical form paying off.

## Four casts instead of one

C has one cast, and it does everything. C++ splits it by intent:

- `static_cast`: conversions the compiler can check (`double` to `int`, up and down a class hierarchy
  you're sure about);
- `reinterpret_cast`: "treat these bits as something else", used in module 06 to turn a pointer into
  a `uintptr_t` and back;
- `const_cast`: removing `const`, almost always a design smell;
- `dynamic_cast`: a **checked** downcast using runtime type information.

Module 06 asks you to identify the real type behind a `Base *` without `typeid`. With pointers, a failed
`dynamic_cast` returns `NULL`. With references there is no null reference, so it throws instead:

```cpp
void identify(Base &p)
{
    try { (void)dynamic_cast<A &>(p); std::cout << "A\n"; return; } catch (...) {}
    try { (void)dynamic_cast<B &>(p); std::cout << "B\n"; return; } catch (...) {}
    try { (void)dynamic_cast<C &>(p); std::cout << "C\n"; return; } catch (...) {}
}
```

## Module 09: three programs

The last module rewards everything before it, with three exercises that each use a different container
(and you may not reuse one).

**Bitcoin exchange** (`std::map`). Given a CSV of daily prices and an input file of `date | amount`
lines, print each amount's value. If the date isn't in the database, use the closest **earlier** date.
A sorted map makes that one call: `lower_bound(date)` returns the first entry not before the date, so
stepping back once gives the previous day.

**RPN** (`std::stack`). Evaluate `"8 9 * 9 - 9 - 9 - 4 - 1 +"`: push numbers, and on an operator pop
two, apply it, and push the result. The interesting part is error handling: too few operands, division
by zero, leftover values at the end.

**PmergeMe**. Sort a sequence with the **Ford-Johnson merge-insertion** algorithm, twice, once with
`std::vector` and once with `std::deque`, and time both. Ford-Johnson minimizes *comparisons*, not
time:

1. Pair up the elements and compare each pair. The larger ones are the "main chain".
2. Sort the main chain recursively.
3. Insert the smaller elements into it with binary search, in an order given by the **Jacobsthal
   numbers** (1, 3, 5, 11, 21, …), chosen so that each binary search runs on a range whose size is just
   under a power of two, where binary search wastes no comparison.

The hard part is doing this without copying everything at each level of recursion. My version uses a
small `GroupIterator` template that makes a range look like a sequence of *groups* of `size` elements,
compared by their last element. Swapping two "elements" swaps whole groups, so the recursion happens
in place, and the same code works for both containers:

```cpp
template <typename Iterator>
typename std::iterator_traits<Iterator>::reference
GroupIterator<Iterator>::operator*() const
{
    return this->iter[this->size - 1];   // a group is represented by its last element
}
```

## Looking back

C++98 is verbose, and that's the point. After writing every copy constructor and destructor by hand,
modern C++ reads differently: each `std::unique_ptr` is a destructor you no longer have to remember,
and each `= default` is a canonical form you no longer have to write. Knowing what they replace is what
these modules teach.
