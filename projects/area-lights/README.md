# Area Lights

![Area Lights](thumbnail.png)

Ported from the [Area Lights](https://learnopengl.com/Guest-Articles/2022/Area-Lights) guest article of LearnOpenGL. Three rectangular lights in a dark room, shaded with Linearly Transformed Cosines. Each panel is integrated over its whole rectangle, so the soft shadow edges and the long reflections on the floor come out of the maths rather than out of a pile of point lights.

The geometry is all procedural: five quads for the room, one per panel, built from the vertex index. The only assets are the two lookup tables.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/area-lights&name=Area%20Lights)

## The lookup tables

Two 64x64 RGBA 32-bit float images:

- **ltc1** — the four non-zero entries of the inverse LTC matrix.
- **ltc2** — GGX norm, Fresnel, unused, and the horizon-clipped sphere.

Both are indexed by roughness across and viewing angle down. They are stored as float EXR because the values go negative and past 1.0, which PNG cannot hold.

## Credits

### Shaders

`room.wgsl` is ported from the [Area Lights](https://learnopengl.com/Guest-Articles/2022/Area-Lights) guest article of [LearnOpenGL](https://learnopengl.com) by Alexander Christensen, licensed under [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/).

Its `integrate_edge_vec` and `ltc_evaluate` routines come from the [reference code](https://github.com/selfshadow/ltc_code) of [Real-Time Polygonal-Light Shading with Linearly Transformed Cosines](https://eheitzresearch.wordpress.com/415-2/) by Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt (ACM Transactions on Graphics 35(4), SIGGRAPH 2016), licensed under the [BSD 3-Clause License](https://github.com/selfshadow/ltc_code/blob/master/LICENSE).

### Textures

`ltc1.exr` and `ltc2.exr` are converted from `ltc_matrix.hpp` in the [LearnOpenGL repository](https://github.com/JoeyDeVries/LearnOpenGL), and hold the lookup tables from the same paper's [reference code](https://github.com/selfshadow/ltc_code), licensed under the [BSD 3-Clause License](https://github.com/selfshadow/ltc_code/blob/master/LICENSE).
