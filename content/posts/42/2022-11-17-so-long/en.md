+++
title = "so_long: a tiny 2D game and a map validator"
description = "A tile-based game in MiniLibX: parsing .ber maps, rejecting the invalid ones, and a flood fill that proves every collectible and the exit can be reached."
tags = ["42", "c", "graphics"]
+++

**so_long** is the first 42 project with a window. You write a small top-down game with
[MiniLibX](https://github.com/42Paris/minilibx-linux), 42's minimal X11/Cocoa wrapper: the player
walks around with WASD, picks up every collectible, then reaches the exit. The move count is printed
in the terminal ([source](https://github.com/PfClaKr/Ecole42Paris_Solong)).

The game itself is maybe a third of the work. The rest is making sure the map you're given is
playable.

## The map format

A map is a `.ber` text file, one character per tile:

```text
111111111111111111111111111111111111
11E000000010000111100C0000001100C011
1000010000100000101C010000C001010001
111000000000101000000000000001000001
1000010100100000101C000000C000000001
101001001010101000100100000001011111
1P00010000C1111000100000C11100000C11
111111111111111111111111111111111111
```

`1` is a wall, `0` floor, `C` a collectible, `E` the exit, `P` the player. The subject lists what
makes a map invalid, and the program has to print `Error` and quit cleanly for each case:

- the map is not a rectangle;
- it isn't closed by walls on all four sides;
- there isn't exactly one `P`, exactly one `E`, and at least one `C`;
- it contains any other character;
- **some collectible or the exit can't be reached.**

The first four are simple loops over the grid. The last one needs a search.

## Proving the map is solvable

The check is a recursive flood fill from the player. It walks through floor and collectibles, marks
each visited tile so it's never visited twice, and treats walls and the exit as blocked. The exit is
blocked because, in my game, you can't walk over it until everything is collected.

```c
int path_algo(t_map *m, char **map, int x, int y)
{
    if (x < 0 || y < 0 || x >= m->row || y >= m->col)
        return (0);
    if (x == m->c_row[i] && y == m->c_col[i])   // reached the current target
        return (1);
    if (map[x][y] != 'P' && map[x][y] != '0' && map[x][y] != 'C')
        return (0);                              // wall, exit, or already visited
    map[x][y] = '2';                             // mark as visited
    return (path_algo(m, map, x + 1, y) || path_algo(m, map, x - 1, y)
         || path_algo(m, map, x, y + 1) || path_algo(m, map, x, y - 1));
}
```

Because the fill writes into the grid, it runs on a **copy** of the map. My version runs one search
per target (each collectible plus the exit), each time on a fresh copy. It's correct but wasteful. A
single fill from `P`, followed by a check that no `C` remains unvisited and that a visited tile
touches `E`, does the same job in one pass. That's what I'd write now.

## The game loop

MiniLibX is event-driven: you register a key hook and hand control to `mlx_loop`.

```c
mlx_key_hook(win.win_ptr, key_input, &info);          // key released
mlx_hook(win.win_ptr, 17, 0, win_exit, &win);          // window close button
mlx_loop(win.mlx_ptr);
```

On each key press, the handler looks at the target tile: a wall does nothing, floor moves the player,
a collectible moves and decrements the counter, and the exit ends the game only if the counter is
zero. Then it redraws the grid with five 32×32 XPM sprites.
