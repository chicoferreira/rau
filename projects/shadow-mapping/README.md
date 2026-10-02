# Shadow Mapping

![Shadow Mapping](thumbnail.png)

Boxes casting shadows from a spot light with classic two-pass shadow mapping. The shadow pass renders the boxes from the light into a depth texture, and the scene pass compares each fragment against it to decide whether it is in shadow. A second viewport shows what the light sees.

[Open in browser](https://rau.chicoferreira.dev/?action=new&owner=chicoferreira&repo=rau&ref=main&path=projects/shadow-mapping&name=Shadow%20Mapping)
