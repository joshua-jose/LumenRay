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

vec3 fxaa(vec2 uv, vec2 iResolution){
    float FXAA_SPAN_MAX = 8.0;
    float FXAA_REDUCE_MUL = 1.0/FXAA_SPAN_MAX;
    float FXAA_REDUCE_MIN = 1.0/128.0;

    vec3 rgbNW=texture(tex,uv+(vec2(-1.0,-1.0)/iResolution)).xyz;
    vec3 rgbNE=texture(tex,uv+(vec2(1.0,-1.0)/iResolution)).xyz;
    vec3 rgbSW=texture(tex,uv+(vec2(-1.0,1.0)/iResolution)).xyz;
    vec3 rgbSE=texture(tex,uv+(vec2(1.0,1.0)/iResolution)).xyz;
    vec3 rgbM=texture(tex,uv).xyz;

    vec3 luma=vec3(0.299, 0.587, 0.114);
    float lumaNW = dot(rgbNW, luma);
    float lumaNE = dot(rgbNE, luma);
    float lumaSW = dot(rgbSW, luma);
    float lumaSE = dot(rgbSE, luma);
    float lumaM  = dot(rgbM,  luma);

    float lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    float lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    vec2 dir;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y =  ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    float dirReduce = max(
        (lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * FXAA_REDUCE_MUL),
        FXAA_REDUCE_MIN);

    float rcpDirMin = 1.0/(min(abs(dir.x), abs(dir.y)) + dirReduce);

    dir = min(vec2( FXAA_SPAN_MAX,  FXAA_SPAN_MAX),
          max(vec2(-FXAA_SPAN_MAX, -FXAA_SPAN_MAX),
          dir * rcpDirMin)) / iResolution;

    vec3 rgbA = (1.0/2.0) * (
        texture(tex, uv + dir * (1.0/3.0 - 0.5)).xyz +
        texture(tex, uv + dir * (2.0/3.0 - 0.5)).xyz);
    vec3 rgbB = rgbA * (1.0/2.0) + (1.0/4.0) * (
        texture(tex, uv + dir * (0.0/3.0 - 0.5)).xyz +
        texture(tex, uv + dir * (3.0/3.0 - 0.5)).xyz);
    float lumaB = dot(rgbB, luma);

    if((lumaB < lumaMin) || (lumaB > lumaMax)){
        return rgbA;
    }else{
        return rgbB;
    }
}

void main() {
    vec2 iResolution = textureSize(tex,0);

    vec2 uv = gl_FragCoord.xy / iResolution;
    uv.y = 1.0-uv.y;  // flip

    vec3 colour = ACESFilm(texture(tex, uv).xyz);
    //vec3 colour = ACESFilm(fxaa(uv,iResolution));

    fragColor = vec4(colour,1.0);
}