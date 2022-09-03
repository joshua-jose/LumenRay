vec3 aces_tonemap(vec3 color) {
    mat3 m1 = mat3(0.59719, 0.07600, 0.02840, 0.35458, 0.90834, 0.13383,
                   0.04823, 0.01566, 0.83777);
    mat3 m2 = mat3(1.60475, -0.10208, -0.00327, -0.53108, 1.10813, -0.07276,
                   -0.07367, -0.00605, 1.07602);
    vec3 v = m1 * color;
    vec3 a = v * (v + 0.0245786) - 0.000090537;
    vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;

    // vec3 col = clamp(m2 * (a / b), 0.0, 1.0);
    vec3 col = m2 * (a / b);

    return col;
    // return toSRGB(col);	// gamma corrected
}

vec3 ACESFilm(vec3 x) {
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return (x * (a * x + b)) / (x * (c * x + d) + e);
}

vec3 toSRGB(vec3 col) {
    vec3 a = 12.92 * col;
    vec3 b = 1.055 * pow(col, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), col);
    return mix(a, b, c);
}

const int bayer[8][8] = {
    {0, 32, 8, 40, 2, 34, 10, 42},     /* 8x8 Bayer ordered dithering  */
    {48, 16, 56, 24, 50, 18, 58, 26},  /* pattern.  Each input pixel   */
    {12, 44, 4, 36, 14, 46, 6, 38},    /* is scaled to the 0..63 range */
    {60, 28, 52, 20, 62, 30, 54, 22},  /* before looking in this table */
    {3, 35, 11, 43, 1, 33, 9, 41},     /* to determine the action.     */
    {51, 19, 59, 27, 49, 17, 57, 25},  /*                              */
    {15, 47, 7, 39, 13, 45, 5, 37},    /*                              */
    {63, 31, 55, 23, 61, 29, 53, 21}}; /*                              */

vec3 dither(uvec2 pix_coord, float colour_depth) {
    const float DITHER_SCALE = 2.0;
    // go from 0..64 to 0..1 , then to -0.5..0.5
    int dither_amount = bayer[pix_coord.x % 8][pix_coord.y % 8];
    return vec3(DITHER_SCALE * (dither_amount - 32) / 64.0) / colour_depth;
}

// required descriptors:
/*
uniform sampler samp;
uniform texture2D textures[];
*/
vec3 sample_texture(Material mat, vec2 uv) {
    return texture(sampler2D(textures[mat.tex_id], samp), uv * mat.tex_scale)
        .xyz;
}