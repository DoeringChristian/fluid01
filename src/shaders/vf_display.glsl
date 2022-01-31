#version 460
#if VERTEX_SHADER

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;

layout(location = 0) out vec2 f_pos;
layout(location = 1) out vec2 f_uv;

void main(){
    f_pos = i_pos;
    f_uv = i_uv;

    gl_Position = vec4(i_pos, 0.0, 1.0);
}
#endif
#if FRAGMENT_SHADER

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;

layout(location = 0) out vec4 o_color;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
} global_data;

layout(set = 1, binding = 0) uniform texture2D t_tex;
layout(set = 1, binding = 1) uniform sampler s_tex;

void main(){
    o_color = texture(sampler2D(t_tex, s_tex), f_uv);
    //o_color = vec4(vec3(texture(sampler2D(t_tex, s_tex), f_uv).w), 1.0);
}
#endif
