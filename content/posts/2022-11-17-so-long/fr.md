+++
title = "so_long : un petit jeu 2D et un validateur de cartes"
description = "Un jeu en tuiles avec la MiniLibX : lire des cartes .ber, rejeter celles qui sont invalides, et un remplissage par diffusion qui prouve que chaque objet et la sortie sont atteignables."
tags = ["42", "c", "graphics"]
+++

**so_long** est le premier projet de 42 avec une fenêtre. On écrit un petit jeu vu de dessus avec la
[MiniLibX](https://github.com/42Paris/minilibx-linux), l'enveloppe minimale de 42 autour de X11 et
Cocoa : le joueur se déplace avec WASD, ramasse tous les objets, puis rejoint la sortie. Le nombre de
déplacements s'affiche dans le terminal ([source](https://github.com/PfClaKr/Ecole42Paris_Solong)).

Le jeu lui-même représente peut-être un tiers du travail. Le reste consiste à vérifier que la carte
fournie est jouable.

## Le format de carte

Une carte est un fichier texte `.ber`, un caractère par tuile :

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

`1` est un mur, `0` le sol, `C` un objet, `E` la sortie, `P` le joueur. Le sujet liste ce qui rend
une carte invalide, et le programme doit afficher `Error` et quitter proprement dans chaque cas :

- la carte n'est pas rectangulaire ;
- elle n'est pas fermée par des murs sur les quatre côtés ;
- il n'y a pas exactement un `P`, exactement un `E` et au moins un `C` ;
- elle contient un autre caractère ;
- **un objet ou la sortie est inatteignable.**

Les quatre premiers sont de simples boucles sur la grille. Le dernier demande une recherche.

## Prouver que la carte est faisable

La vérification est un remplissage récursif depuis le joueur. Il traverse le sol et les objets,
marque chaque case visitée pour ne jamais y repasser, et considère les murs et la sortie comme
bloquants. La sortie est bloquante parce que, dans mon jeu, on ne peut pas marcher dessus tant que
tout n'est pas ramassé.

```c
int path_algo(t_map *m, char **map, int x, int y)
{
    if (x < 0 || y < 0 || x >= m->row || y >= m->col)
        return (0);
    if (x == m->c_row[i] && y == m->c_col[i])   // cible courante atteinte
        return (1);
    if (map[x][y] != 'P' && map[x][y] != '0' && map[x][y] != 'C')
        return (0);                              // mur, sortie ou déjà visité
    map[x][y] = '2';                             // marquer comme visité
    return (path_algo(m, map, x + 1, y) || path_algo(m, map, x - 1, y)
         || path_algo(m, map, x, y + 1) || path_algo(m, map, x, y - 1));
}
```

Comme le remplissage écrit dans la grille, il tourne sur une **copie** de la carte. Ma version lance
une recherche par cible (chaque objet plus la sortie), à chaque fois sur une copie neuve. C'est
correct mais coûteux. Un seul remplissage depuis `P`, suivi d'une vérification qu'aucun `C` n'est
resté non visité et qu'une case visitée touche `E`, fait le même travail en une passe. C'est ce que
j'écrirais aujourd'hui.

## La boucle de jeu

La MiniLibX est pilotée par les événements : on enregistre un hook clavier et on rend la main à
`mlx_loop`.

```c
mlx_key_hook(win.win_ptr, key_input, &info);       // touche relâchée
mlx_hook(win.win_ptr, 17, 0, win_exit, &win);          // bouton de fermeture
mlx_loop(win.mlx_ptr);
```

À chaque touche, le handler regarde la case visée : un mur ne fait rien, le sol déplace le joueur,
un objet le déplace et décrémente le compteur, et la sortie ne termine la partie que si le compteur
est à zéro. Puis il redessine la grille avec cinq sprites XPM de 32×32.

## Ce que j'en retiens

- **Valider l'entrée avant de lui faire confiance.** L'essentiel du code, et de l'évaluation, porte
  sur les mauvaises cartes : fichier vide, saut de ligne manquant, carte d'une seule tuile de large.
- **Chaque chemin de sortie doit libérer.** La fenêtre peut se fermer par ESC, par la croix, par une
  victoire ou par une erreur au milieu du chargement, et chacun doit libérer la carte et les images.
- **La recherche dans un graphe surgit là où on ne l'attend pas.** Sous les sprites, ce projet de
  « jeu » est surtout un exercice de parcours en profondeur.
