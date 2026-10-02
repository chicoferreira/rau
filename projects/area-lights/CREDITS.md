# Credits

## Shaders

`room.wgsl` is ported from the [Area Lights](https://learnopengl.com/Guest-Articles/2022/Area-Lights) guest article of [LearnOpenGL](https://learnopengl.com) by Alexander Christensen, licensed under [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/).

Its `integrate_edge_vec` and `ltc_evaluate` routines come from the [reference code](https://github.com/selfshadow/ltc_code) of [Real-Time Polygonal-Light Shading with Linearly Transformed Cosines](https://eheitzresearch.wordpress.com/415-2/) by Eric Heitz, Jonathan Dupuy, Stephen Hill and David Neubelt (ACM Transactions on Graphics 35(4), SIGGRAPH 2016), licensed under the [BSD 3-Clause License](https://github.com/selfshadow/ltc_code/blob/master/LICENSE).

## Textures

`ltc1.exr` and `ltc2.exr` are converted from `ltc_matrix.hpp` in the [LearnOpenGL repository](https://github.com/JoeyDeVries/LearnOpenGL), and hold the lookup tables from the same paper's [reference code](https://github.com/selfshadow/ltc_code), licensed under the [BSD 3-Clause License](https://github.com/selfshadow/ltc_code/blob/master/LICENSE).
