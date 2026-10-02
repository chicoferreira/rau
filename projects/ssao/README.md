# SSAO

![SSAO](thumbnail.png)

Ported from the [SSAO](https://learnopengl.com/Advanced-Lighting/SSAO) chapter of LearnOpenGL. Screen-space ambient occlusion on a backpack in a room, using deferred shading. A G-buffer pass writes positions and normals, an SSAO pass estimates the occlusion from them, a blur pass removes its noise, and a lighting pass combines everything with a point light.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/ssao&name=SSAO)

## Credits

All shaders and models come from the [SSAO](https://learnopengl.com/Advanced-Lighting/SSAO) chapter of [LearnOpenGL](https://learnopengl.com) by Joey de Vries, with the backpack model being [Survival Guitar Backpack](https://sketchfab.com/3d-models/survival-guitar-backpack-low-poly-799f8c4511f84fab8c3f12887f7e6b36) by Berk Gedik. Check [LearnOpenGL's licensing](https://learnopengl.com/About) for licensing details.
