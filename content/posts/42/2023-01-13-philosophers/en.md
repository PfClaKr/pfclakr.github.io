+++
title = "Philosophers: threads, mutexes and not starving"
description = "The dining philosophers problem with pthreads: one mutex per fork, a monitor thread per philosopher, staggered starts against deadlock, and the data races that hide in a shared stop flag."
tags = ["42", "c", "concurrency"]
+++

**Philosophers** is 42's take on Dijkstra's dining philosophers. *N* philosophers sit around a table
with one fork between each pair. To eat, a philosopher needs both forks next to them. They eat, sleep,
think, and repeat. If one goes `time_to_die` milliseconds without starting a meal, they die and the
simulation stops ([source](https://github.com/PfClaKr/Ecole42Paris_Philosophers)).

```text
./philo 5 800 200 200 [7]
        │  │   │   │   └─ optional: stop once everyone has eaten 7 times
        │  │   │   └─ time_to_sleep (ms)
        │  │   └─ time_to_eat (ms)
        │  └─ time_to_die (ms)
        └─ number of philosophers
```

The output is a log like `200 Philo 3 is eating`, and the graders read it closely: no message may
appear after a death, and a death must be reported within 10 ms.

## The model

Each philosopher is a thread, and each fork is a `pthread_mutex_t`. Philosopher *i* uses forks *i*
and *(i + 1) mod N*:

```c
info->philo[i].fork_left = i;
info->philo[i].fork_right = (i + 1) % info->num_philo;
```

Eating means locking both mutexes, recording the meal time, waiting `time_to_eat`, then unlocking.

## Deadlock

If every philosopher grabs their right fork at the same moment, everyone holds one fork and waits
forever for the other. That's the classic deadlock, and it happens immediately with a naive version.

My fix is to **stagger the start**: even-numbered philosophers wait `time_to_eat` before their first
meal, so the odd ones get both forks first and the table settles into an alternating rhythm.

```c
if (philo->n % 2 == 0 && philo->info->num_philo != 1)
    usleep(1000 * philo->info->time_to_eat);
```

The more robust textbook fix is **lock ordering**: always take the lower-numbered fork first. Then
the last philosopher reaches for the same fork as the first one, a cycle of waiting can't form, and
deadlock is impossible no matter how the threads are scheduled. Staggering works in practice;
ordering is a proof.

One philosopher alone has only one fork. They pick it up, can never get a second, and must die on
time, which is a special case in the code.

## Watching for death

Each philosopher gets a second thread, a monitor, that checks how long ago it last started eating:

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

`start_time` is written by the philosopher and read by the monitor, so both go through
`philo_mutex`. Printing goes through another mutex, so two log lines can never interleave, and
`print_philo` refuses to print once `stop` is set.

## Time is not exact

`usleep(200000)` sleeps *at least* 200 ms, often a few ms more, and those few ms add up over a long
run until someone dies who shouldn't. The fix is to sleep in small steps and check the clock:

```c
start = get_time();
while (get_time() - start <= time_to_sleep && !info->stop)
    usleep(1000);
```

This also lets a sleeping philosopher notice `stop` quickly instead of finishing a long nap after
the simulation ended.

## What a race detector would say

The program behaves correctly in the tests, but `-fsanitize=thread` would complain. The shared
`stop` flag is a plain `int` read by every thread without a lock, and the meal counter is read by
`check_meals` while other threads increment it. On x86 these reads happen to work, but they're data
races, so the C standard makes no promise. Today I'd protect `stop` with the print mutex (or make
it `atomic_int`) and count finished philosophers under a lock.
