#version 450

layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform sampler2D tex;

in vec4 gl_FragCoord;

vec3 aces_tonemap(vec3 color){	
	mat3 m1 = mat3(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777
	);
	mat3 m2 = mat3(
        1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602
	);
	vec3 v = m1 * color;    
	vec3 a = v * (v + 0.0245786) - 0.000090537;
	vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;

    //vec3 col = clamp(m2 * (a / b), 0.0, 1.0);
    vec3 col = m2 * (a / b);

    return col;
	//return toSRGB(col);	// gamma corrected
}

vec3 ACESFilm(vec3 x){
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return (x*(a*x+b))/(x*(c*x+d)+e);
}

vec3 toSRGB(vec3 col)
{
    vec3 a = 12.92 * col;
    vec3 b = 1.055 * pow(col, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), col);
    return mix(a, b, c);
}

void main() {
    vec2 iResolution = textureSize(tex,0);

    vec2 uv = gl_FragCoord.xy / iResolution;
    uv.y = 1.0-uv.y;  // flip

    vec3 colour = ACESFilm(texture(tex, uv).xyz);

    fragColor = vec4(colour,1.0);
}