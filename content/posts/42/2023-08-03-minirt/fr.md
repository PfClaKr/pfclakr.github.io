+++
title = "miniRT : un ray tracer à partir des principes de base"
description = "Un ray tracer CPU en C, réalisé en binôme : rayons de caméra, équations du second degré pour sphères et cylindres, ombrage de Phong avec rayons d'ombre, et coordonnées UV pour damier, textures image et normal mapping (125/100)."
tags = ["42", "c", "graphics"]
+++

**miniRT** est le projet de ray tracing de 42 : lire un fichier de scène, lancer un rayon à travers
chaque pixel, et dessiner ce qu'il touche avec éclairage et ombres. Pas d'OpenGL, pas de GPU,
seulement du C, `math.h`, et la MiniLibX pour afficher des pixels dans une fenêtre. On l'a réalisé en
binôme avec tout le bonus, **125/100** ([source](https://github.com/PfClaKr/Ecole42Paris_miniRT)).

## Le fichier de scène

```text
A   0.2             255,255,255       # lumière ambiante : ratio, couleur
C   0,0,30   0,0,-1   70              # caméra : position, direction, FOV
L   0,3,0    1        255,0,0         # lumière ponctuelle : position, intensité, couleur
sp  -10,0,0  5        155,53,152      # sphère : centre, diamètre, couleur
pl  0,-3,0   0,1,0.2  55,35,172       # plan : point, normale, couleur
cy  10,0,0   1,1,1    5  5  142,153,45  # cylindre : centre, axe, diamètre, hauteur, couleur
```

Le parsing est une grosse partie du projet. `A`, `C` et `L` doivent apparaître exactement une fois,
chaque vecteur doit être normalisé ou normalisable, les ratios dans [0, 1], les couleurs dans
[0, 255]. Le dépôt contient un dossier de scènes volontairement cassées, et chacune doit être rejetée
avec une erreur claire au lieu de planter.

## Un rayon par pixel

À partir de la position, de la direction et du champ de vision de la caméra, on construit un écran
virtuel devant elle : son coin inférieur gauche, plus un vecteur de pas « droite » et un vecteur
« haut » par pixel. Le rayon du pixel *(i, j)* est alors :

```c
ray.pos    = cam->pos;
ray.orivec = vunit(cam->left_bottom + i * cam->right_normal
                                    + j * cam->up_normal - cam->pos);
```

Pour chaque rayon, on teste tous les objets et on garde l'impact le **plus proche**. L'astuce :
réduire `tmax` à chaque impact, pour que les objets plus lointains soient rejetés d'office :

```c
while (objects)
{
    hit += hit_sphere/plane/cylinder(objects->data, ray, rec);
    if (hit > 0)
        rec->tmax = rec->t;     // n'accepter que des impacts plus proches
    objects = objects->next;
}
```

## Les intersections sont des équations du second degré

Un point du rayon s'écrit `P(t) = O + tD`. Une sphère, c'est l'ensemble des points à distance *r* de
son centre *C*. En substituant l'un dans l'autre, on obtient une équation du second degré en *t* :

```text
(D·D) t² + 2 D·(O − C) t + |O − C|² − r² = 0
```

Pas de racine réelle, pas d'impact. Sinon, on prend la plus petite racine dans `[tmin, tmax]`.
Utiliser `half_b = D · (O − C)` au lieu de `b` économise quelques multiplications par rayon :

```c
oc = vminus(ray.pos, sp->pos);
a = vlength2(ray.orivec);
half_b = vdot(oc, ray.orivec);
discriminant = half_b * half_b - a * (vlength2(oc) - sp->radius2);
```

Un cylindre infini suit la même idée, mais la distance se mesure à son **axe**, ce que le produit
vectoriel calcule élégamment (`|(P − C) × axe|` est la distance de *P* à l'axe). Puis deux
vérifications en plus : l'impact doit être à moins d'une demi-hauteur du centre le long de l'axe, et
les deux bouchons sont des disques, testés comme des intersections rayon–plan limitées au rayon. Le
plan est le plus simple de tous : une division, `t = ((P₀ − O) · n) / (D · n)`.

## Lumière et ombre

Une fois le point d'impact *P* et sa normale *N* connus, la couleur vient d'un modèle de type Phong :

- **ambiante** : une fraction constante de la couleur de l'objet, pour que rien ne soit totalement noir ;
- **diffuse** : proportionnelle à `max(N · L, 0)`, le cosinus entre la normale et la direction de la
  lumière ;
- **spéculaire** (bonus) : un reflet proportionnel à `max(R · V, 0)^brillance`, où *R* est la direction
  de la lumière réfléchie autour de *N* et *V* pointe vers la caméra.

Avant d'ajouter la contribution d'une lumière, on lance un **rayon d'ombre** de *P* vers la lumière.
S'il touche quelque chose avant la lumière, celle-ci ne contribue pas :

```c
ray.pos = rec->p;
ray.orivec = vunit(vminus(light->pos, rec->p));
tmp.tmin = EPSILON;
tmp.tmax = vlength(vminus(light->pos, rec->p));
if (hit_object(objects, ray, &tmp))
    return (vec3(0, 0, 0));     // dans l'ombre
```

`tmin = EPSILON` compte. Sans lui, l'erreur d'arrondi fait toucher au rayon d'ombre la surface même
d'où il part, et chaque objet se couvre de points noirs (« shadow acne »).

## Les textures (bonus)

Pour plaquer une texture, chaque point d'impact a besoin de coordonnées 2D *(u, v)* dans [0, 1) :

- **sphère** : angles sphériques, `u = φ / 2π`, `v = θ / π` ;
- **plan** : le point projeté sur deux vecteurs tangents, ramené dans [0, 1) avec `fmod` ;
- **cylindre et cône** : l'angle autour de l'axe pour *u*, la hauteur le long de l'axe pour *v*.

À partir de *(u, v)*, un **damier** vaut simplement `(floor(u·k) + floor(v·k)) % 2`. Une **texture
image** lit le pixel correspondant dans un fichier XPM. Une **normal map** stocke une direction dans
chaque pixel : les valeurs RGB passent de [0, 255] à [−1, 1] puis sont tournées dans le repère de la
surface (tangente, bitangente, normale), pour qu'une sphère lisse s'éclaire comme si elle avait du
relief. C'est comme ça que la scène de la Terre a ses montagnes sans un seul polygone de plus.

## Ce qu'on améliorerait

- L'atténuation de la lumière est une courbe linéaire réglée à la main. Physiquement, l'intensité
  décroît avec le carré de la distance, et les scènes à plusieurs lumières seraient plus cohérentes.
- Chaque rayon teste chaque objet. Une hiérarchie de volumes englobants accélérerait beaucoup les
  grosses scènes, mais en 512×512 avec quelques dizaines d'objets, ce n'était pas nécessaire.

Le meilleur côté de miniRT, c'est que chaque bug se voit. Un signe faux, c'est une sphère à l'envers ;
un epsilon manquant, de l'acné ; une mauvaise normale, une bande noire. L'algèbre linéaire n'a jamais
été aussi facile à déboguer.
