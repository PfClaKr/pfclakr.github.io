+++
title = "Philosophers : threads, mutex et ne pas mourir de faim"
description = "Le dîner des philosophes avec pthreads : un mutex par fourchette, un thread moniteur par philosophe, des départs décalés contre l'interblocage, et les data races cachées dans un drapeau d'arrêt partagé."
tags = ["42", "c", "concurrency"]
+++

**Philosophers** est la version 42 du dîner des philosophes de Dijkstra. *N* philosophes sont assis
autour d'une table, avec une fourchette entre chaque paire. Pour manger, un philosophe a besoin des
deux fourchettes voisines. Ils mangent, dorment, pensent, et recommencent. Si l'un d'eux passe
`time_to_die` millisecondes sans commencer un repas, il meurt et la simulation s'arrête
([source](https://github.com/PfClaKr/Ecole42Paris_Philosophers)).

```text
./philo 5 800 200 200 [7]
        │  │   │   │   └─ optionnel : s'arrêter quand chacun a mangé 7 fois
        │  │   │   └─ time_to_sleep (ms)
        │  │   └─ time_to_eat (ms)
        │  └─ time_to_die (ms)
        └─ nombre de philosophes
```

La sortie est un journal du type `200 Philo 3 is eating`, et les évaluateurs le lisent de près : aucun
message après une mort, et une mort doit être annoncée en moins de 10 ms.

## Le modèle

Chaque philosophe est un thread, chaque fourchette un `pthread_mutex_t`. Le philosophe *i* utilise les
fourchettes *i* et *(i + 1) mod N* :

```c
info->philo[i].fork_left = i;
info->philo[i].fork_right = (i + 1) % info->num_philo;
```

Manger, c'est verrouiller les deux mutex, noter l'heure du repas, attendre `time_to_eat`, puis
déverrouiller.

## L'interblocage

Si chaque philosophe prend sa fourchette droite au même instant, tout le monde en tient une et attend
l'autre pour toujours. C'est l'interblocage classique, et il arrive immédiatement avec une version
naïve.

Ma solution est de **décaler le départ** : les philosophes pairs attendent `time_to_eat` avant leur
premier repas, donc les impairs prennent d'abord les deux fourchettes et la table trouve un rythme
alterné.

```c
if (philo->n % 2 == 0 && philo->info->num_philo != 1)
    usleep(1000 * philo->info->time_to_eat);
```

La solution de manuel, plus robuste, est **l'ordre de verrouillage** : toujours prendre d'abord la
fourchette de plus petit numéro. Le dernier philosophe vise alors la même fourchette que le premier,
aucun cycle d'attente ne peut se former, et l'interblocage est impossible quel que soit
l'ordonnancement. Le décalage marche en pratique ; l'ordre est une preuve.

Un philosophe seul n'a qu'une fourchette. Il la prend, ne peut jamais en avoir une deuxième, et doit
mourir à l'heure : c'est un cas particulier dans le code.

## Surveiller la mort

Chaque philosophe a un second thread, un moniteur, qui vérifie depuis combien de temps il a commencé
son dernier repas :

```c
pthread_mutex_lock(&philo->philo_mutex);
if (get_time() - philo->start_time >= philo->info->time_to_die)
{
    print_philo(philo, 5);   // "died"
    philo->info->stop = 1;
}
pthread_mutex_unlock(&philo->philo_mutex);
usleep(100);
```

`start_time` est écrit par le philosophe et lu par le moniteur, donc les deux passent par
`philo_mutex`. L'affichage passe par un autre mutex, pour que deux lignes ne se mélangent jamais, et
`print_philo` refuse d'afficher dès que `stop` est levé.

## Le temps n'est pas exact

`usleep(200000)` dort *au moins* 200 ms, souvent quelques ms de plus, et ces quelques ms s'accumulent
sur une longue exécution jusqu'à faire mourir quelqu'un qui n'aurait pas dû. La solution est de dormir
par petits pas en regardant l'horloge :

```c
start = get_time();
while (get_time() - start <= time_to_sleep && !info->stop)
    usleep(1000);
```

Ça permet aussi à un philosophe endormi de remarquer `stop` vite, au lieu de finir une longue sieste
après la fin de la simulation.

## Ce que dirait un détecteur de races

Le programme se comporte correctement aux tests, mais `-fsanitize=thread` se plaindrait. Le drapeau
partagé `stop` est un simple `int` lu par tous les threads sans verrou, et le compteur de repas est lu
par `check_meals` pendant que d'autres threads l'incrémentent. Sur x86, ces lectures se trouvent
marcher, mais ce sont des data races, et la norme C ne promet rien. Aujourd'hui, je protégerais `stop`
avec le mutex d'affichage (ou j'en ferais un `atomic_int`) et je compterais les philosophes rassasiés
sous verrou.
