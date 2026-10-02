# Ray Tracing

![Ray Tracing](thumbnail.png)

Ported from [*Ray Tracing in One Weekend*](https://raytracing.github.io/books/RayTracingInOneWeekend.html), path tracing the book's final scene with compute shaders. A generate pass builds the scene whenever the scene settings change, the trace pass adds new samples to accumulation textures every frame, and a reset pass clears them whenever the camera or the settings change, so the image refines progressively while it stays still.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/ray-tracing&name=Ray%20Tracing)

## Credits

The path tracer in `trace.wgsl` and the scene in `generate.wgsl` come from [*Ray Tracing in One Weekend*](https://raytracing.github.io/books/RayTracingInOneWeekend.html) by Peter Shirley, Trevor David Black and Steve Hollasch. Check the [Ray Tracing in One Weekend repository](https://github.com/RayTracing/raytracing.github.io) for licensing details.
