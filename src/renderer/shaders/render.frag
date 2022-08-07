#version 450
layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

in vec4 gl_FragCoord;

vec2 iResolution = vec2(800,600);

float rand(vec2 co){
    return fract(sin(dot(co, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 uv = gl_FragCoord.xy / iResolution.xy;
    uv.y = 1.0-uv.y;  // flip
    f_color = texture(tex, uv);
}