# Fur Shell

![Fur Shell](thumbnail.png)

Based on [Real-Time Fur over Arbitrary Surfaces](https://hhoppe.com/fur.pdf). Shell-based fur on the Stanford Bunny. The pipeline draws the model 48 times in one instanced draw call, and the vertex shader pushes each instance a little further out along the normals. The fragment shader discards most of each shell, leaving patches that line up across the shells as strands of fur.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/fur-shell&name=Fur%20Shell)

## Credits

### Shaders

`fur.wgsl` implements the shells from [Real-Time Fur over Arbitrary Surfaces](https://hhoppe.com/fur.pdf) by Jerome Lengyel, Emil Praun, Adam Finkelstein and Hugues Hoppe (Symposium on Interactive 3D Graphics, 2001).

### Models

`bunny.obj` is the Stanford Bunny from the [Stanford 3D Scanning Repository](https://graphics.stanford.edu/data/3Dscanrep/) by the Stanford University Computer Graphics Laboratory, under the repository's usage terms: free for research use and redistribution with credit, but not for commercial use without permission.
