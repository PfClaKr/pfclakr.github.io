+++
title = "miniRT: a ray tracer from first principles"
description = "A CPU ray tracer in C, built as a team of two: camera rays, solving quadratics for spheres and cylinders, Phong shading with shadow rays, and UV mapping for checkerboard, image and normal-map textures (125/100)."
tags = ["42", "c", "graphics"]
+++

**miniRT** is 42's ray tracing project: read a scene file, cast a ray through every pixel, and draw
what it hits with lighting and shadows. No OpenGL, no GPU, just C, `math.h`, and MiniLibX to put pixels
in a window. We built it as a team of two and finished with the full bonus, **125/100**
([source](https://github.com/PfClaKr/Ecole42Paris_miniRT)).

## The scene file

```text
A   0.2             255,255,255       # ambient light: ratio, color
C   0,0,30   0,0,-1   70              # camera: position, direction, FOV
L   0,3,0    1        255,0,0         # point light: position, brightness, color
sp  -10,0,0  5        155,53,152      # sphere: center, diameter, color
pl  0,-3,0   0,1,0.2  55,35,172       # plane: point, normal, color
cy  10,0,0   1,1,1    5  5  142,153,45  # cylinder: center, axis, diameter, height, color
```

Parsing is a big part of the project. `A`, `C` and `L` must appear exactly once, every vector must be
normalized or normalizable, ratios must be in [0, 1], colors in [0, 255]. The repository has a folder
of deliberately broken scenes, and each one must be rejected with a clear error instead of crashing.

## One ray per pixel

From the camera's position, direction and field of view, we build a virtual screen in front of it:
its lower-left corner, plus one "right" and one "up" step vector per pixel. The ray for pixel *(i, j)*
is then:

```c
ray.pos    = cam->pos;
ray.orivec = vunit(cam->left_bottom + i * cam->right_normal
                                    + j * cam->up_normal - cam->pos);
```

For each ray, we test every object and keep the **closest** hit. The trick is to shrink `tmax` each
time something is hit, so farther objects are rejected automatically:

```c
while (objects)
{
    hit += hit_sphere/plane/cylinder(objects->data, ray, rec);
    if (hit > 0)
        rec->tmax = rec->t;     // only accept closer hits from now on
    objects = objects->next;
}
```

## Intersections are quadratics

A point on the ray is `P(t) = O + tD`. A sphere is every point at distance *r* from its center *C*.
Substitute one into the other and you get a quadratic in *t*:

```text
(D·D) t² + 2 D·(O − C) t + |O − C|² − r² = 0
```

No real root means a miss. Otherwise, take the smaller root that lies in `[tmin, tmax]`. Using
`half_b = D · (O − C)` instead of `b` removes a few multiplications per ray:

```c
oc = vminus(ray.pos, sp->pos);
a = vlength2(ray.orivec);
half_b = vdot(oc, ray.orivec);
discriminant = half_b * half_b - a * (vlength2(oc) - sp->radius2);
```

A cylinder of infinite length is the same idea, but distance is measured to its **axis**, which the
cross product computes neatly (`|(P − C) × axis|` is the distance from *P* to the axis). Then two
extra checks: the hit must lie within half the height from the center along the axis, and the two caps
are discs, so they're tested as ray–plane intersections limited to the radius. The plane is the easiest
of all: one division, `t = ((P₀ − O) · n) / (D · n)`.

## Light and shadow

Once we know the hit point *P* and its normal *N*, the color comes from a Phong-style model:

- **ambient**: a constant fraction of the object's color, so nothing is pitch black;
- **diffuse**: proportional to `max(N · L, 0)`, the cosine between the normal and the direction to
  the light;
- **specular** (bonus): a highlight proportional to `max(R · V, 0)^shininess`, where *R* is the light
  direction reflected around *N* and *V* points to the camera.

Before adding a light's contribution, we cast a **shadow ray** from *P* toward the light. If it hits
anything closer than the light, that light contributes nothing:

```c
ray.pos = rec->p;
ray.orivec = vunit(vminus(light->pos, rec->p));
tmp.tmin = EPSILON;
tmp.tmax = vlength(vminus(light->pos, rec->p));
if (hit_object(objects, ray, &tmp))
    return (vec3(0, 0, 0));     // in shadow
```

`tmin = EPSILON` matters. Without it, floating-point error makes the shadow ray hit the very surface
it starts from, and every object gets covered in black speckles ("shadow acne").

## Textures (bonus)

To put a texture on a surface, every hit point needs 2D coordinates *(u, v)* in [0, 1):

- **sphere**: spherical angles, `u = φ / 2π`, `v = θ / π`;
- **plane**: the hit point projected onto two tangent vectors, wrapped with `fmod`;
- **cylinder and cone**: the angle around the axis for *u*, the height along it for *v*.

From *(u, v)*, a **checkerboard** is just `(floor(u·k) + floor(v·k)) % 2`. An **image texture** looks
up the matching pixel in an XPM file. A **normal map** stores a direction in each pixel: RGB values
are mapped from [0, 255] to [−1, 1] and rotated into the surface's frame (tangent, bitangent, normal),
so a smooth sphere shades as if it had relief. That's how the earth scene gets its mountains without a
single extra polygon.

## What we'd improve

- The light falloff is a hand-tuned linear curve. Physically, intensity drops with the square of the
  distance, and scenes with several lights would look more consistent with it.
- Every ray tests every object. A bounding volume hierarchy would make large scenes much faster, but
  at 512×512 with a few dozen objects, it wasn't needed.

The best part of miniRT is that every bug is visible. A wrong sign is an inside-out sphere, a missing
epsilon is acne, and a bad normal is a black stripe. Linear algebra has never been this easy to debug.
