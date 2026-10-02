# HDR Skybox

![HDR Skybox](thumbnail.png)

Ported from the [HDR](https://sotrh.github.io/learn-wgpu/intermediate/tutorial13-hdr/) chapter of Learn WGPU. A lit, normal-mapped cube in front of an HDR skybox. A compute pass converts the equirectangular HDR image into a cube map whenever its inputs change, the scene is rendered into a floating-point texture, and a final pass tonemaps it for display.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/hdr-skybox&name=HDR%20Skybox)

## Credits

All shaders, models and textures come from the [HDR](https://sotrh.github.io/learn-wgpu/intermediate/tutorial13-hdr/) chapter of [Learn WGPU](https://github.com/sotrh/learn-wgpu) by Benjamin Hansen. Check the [Learn WGPU repository](https://github.com/sotrh/learn-wgpu) for licensing details.
