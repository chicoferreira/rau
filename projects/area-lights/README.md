# Area Lights

Three rectangular lights in a dark room, shaded with Linearly Transformed
Cosines. Each panel is integrated over its whole rectangle, so the soft shadow
edges and the long reflections on the floor come out of the maths rather than
out of a pile of point lights.

The geometry is all procedural: five quads for the room, one per panel, built
from the vertex index. The only assets are the two lookup tables.

## Credits

The technique, the lookup tables and the `integrate_edge_vec` / `ltc_evaluate`
routines come from:

> **Real-Time Polygonal-Light Shading with Linearly Transformed Cosines.**
> Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt.
> ACM Transactions on Graphics (Proceedings of ACM SIGGRAPH 2016) 35(4), 2016.
> <https://eheitzresearch.wordpress.com/415-2/>

Reference code: <https://github.com/selfshadow/ltc_code>, © 2017 Heitz, Dupuy,
Hill and Neubelt (BSD-3-clause, citation required). The WGSL follows the GLSL in
LearnOpenGL's [Area Lights](https://learnopengl.com/Guest-Articles/2022/Area-Lights)
guest article by Alexander Christensen (CC BY-NC 4.0).

The room is ours. The paper's own figures use Crytek Sponza and an unpublished
Unity scene; neither is reused here.

## The lookup tables

Two 64x64 RGBA 32-bit float images:

- **ltc1** — the four non-zero entries of the inverse LTC matrix.
- **ltc2** — GGX norm, Fresnel, unused, and the horizon-clipped sphere.

Both are indexed by roughness across and viewing angle down. They were converted
from `ltc_matrix.hpp` in the LearnOpenGL repository with a one-off Python script.
Float EXR format because the values go negative and past 1.0, which PNG cannot hold.
