pub fn unit_cube_array() -> [f32; 180] {
    // positions
    [
        // front
        -0.5f32, -0.5, 0.5, 0.0, 0.0,
         0.5, -0.5,  0.5, 1.0, 0.0,
         0.5,  0.5,  0.5, 1.0, 1.0,
         0.5,  0.5,  0.5, 1.0, 1.0,
        -0.5,  0.5,  0.5, 0.0, 1.0,
        -0.5, -0.5,  0.5, 0.0, 0.0,

        // back
         0.5, -0.5, -0.5, 0.0, 0.0,
        -0.5, -0.5, -0.5, 1.0, 0.0,
        -0.5,  0.5, -0.5, 1.0, 1.0,
        -0.5,  0.5, -0.5, 1.0, 1.0,
         0.5,  0.5, -0.5, 0.0, 1.0,
         0.5, -0.5, -0.5, 0.0, 0.0,

        // left
        -0.5, -0.5, -0.5,  0.0, 0.0,
        -0.5, -0.5,  0.5,  1.0, 0.0,
        -0.5,  0.5,  0.5,  1.0, 1.0,
        -0.5,  0.5,  0.5,  1.0, 1.0,
        -0.5,  0.5, -0.5,  0.0, 1.0,
        -0.5, -0.5, -0.5,  0.0, 0.0,

        // right
        0.5, -0.5,  0.5,  0.0, 0.0,
        0.5, -0.5, -0.5,  1.0, 0.0,
        0.5,  0.5, -0.5,  1.0, 1.0,
        0.5,  0.5, -0.5,  1.0, 1.0,
        0.5,  0.5,  0.5,  0.0, 1.0,
        0.5, -0.5,  0.5,  0.0, 0.0,

        // down
        -0.5, -0.5, -0.5, 0.0, 0.0,
         0.5, -0.5, -0.5, 1.0, 0.0,
         0.5, -0.5,  0.5, 1.0, 1.0,
         0.5, -0.5,  0.5, 1.0, 1.0,
        -0.5, -0.5,  0.5, 0.0, 1.0,
        -0.5, -0.5, -0.5, 0.0, 0.0,

        // up
        -0.5,  0.5,  0.5, 0.0, 0.0,
         0.5,  0.5,  0.5, 1.0, 0.0,
         0.5,  0.5, -0.5, 1.0, 1.0,
         0.5,  0.5, -0.5, 1.0, 1.0,
        -0.5,  0.5, -0.5, 0.0, 1.0,
        -0.5,  0.5,  0.5, 0.0, 0.0,
    ]
}