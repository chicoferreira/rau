# Area Lights

Three rectangular lights in a dark room, shaded with Linearly Transformed
Cosines. Each panel is integrated over its whole rectangle, so the soft shadow
edges and the long reflections on the floor come out of the maths rather than
out of a pile of point lights.

The geometry is all procedural: five quads for the room, one per panel, built
from the vertex index. The only assets are the two lookup tables.

## Credits

See [CREDITS.md](CREDITS.md).

## The lookup tables

Two 64x64 RGBA 32-bit float images:

- **ltc1** — the four non-zero entries of the inverse LTC matrix.
- **ltc2** — GGX norm, Fresnel, unused, and the horizon-clipped sphere.

Both are indexed by roughness across and viewing angle down. They are stored as
float EXR because the values go negative and past 1.0, which PNG cannot hold.
