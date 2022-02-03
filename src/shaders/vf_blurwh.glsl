#version 460
#if VERTEX_SHADER

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;

layout(location = 0) out vec2 f_pos;
layout(location = 1) out vec2 f_uv;
layout(location = 2) out vec2 r;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
    float time;
} global_data;

void main(){
    f_pos = i_pos;
    f_uv = i_uv;
    r = i_uv * global_data.size;

    gl_Position = vec4(i_pos, 0.0, 1.0);
}

#endif
#if FRAGMENT_SHADER

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;
layout(location = 2) in vec2 r;

layout(location = 0) out vec4 o;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
    float time;
} global_data;

layout(set = 1, binding = 0) uniform texture2D t_tex_vpf;
layout(set = 1, binding = 1) uniform sampler s_tex_vpf;

#define tex_vpf t_tex_vpf, s_tex_vpf

float coeff[] = {
    0.19859610213125314,
    0.17571363439579307,
    0.12170274650962626,
    0.06598396774984912,
    0.028001560233780885,
    0.009300040045324049
};

vec4 tex(vec2 pos, texture2D t, sampler s){
    return textureLod(sampler2D(t, s), pos/global_data.size, 0);
}

vec4 blur5(vec2 r, vec2 dir, texture2D t, sampler s){
    vec4 res = tex(r, t, s) * coeff[0];
    for(int i = 1; i < 5; i++){
        res += tex(r + dir * i, t, s) * coeff[i];
        res += tex(r - dir * i, t, s) * coeff[i];
    }
    return res;
}

void main(){
    o.xyz = tex(r, tex_vpf).xyz;
    o.w = blur5(r, vec2(1, 0), tex_vpf).w;
}

#endif
