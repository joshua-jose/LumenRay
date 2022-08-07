#version 450

layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform sampler2D tex;

in vec4 gl_FragCoord;

vec2 iResolution = vec2(800,600);

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

    vec3 col = clamp(m2 * (a / b), 0.0, 1.0);

    return col;
	//return pow(col, vec3(1.0 / 2.2));	// gamma corrected
}

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    uv.y = 1.0-uv.y;  // flip

    vec3 colour = aces_tonemap(texture(tex, uv).xyz);

    fragColor = vec4(colour,1.0);
}