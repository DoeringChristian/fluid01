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

layout(set = 1, binding = 0) uniform texture2D t_tex_vpf;
layout(set = 1, binding = 1) uniform sampler s_tex_vpf;
layout(set = 2, binding = 0) uniform texture2D t_tex_color;
layout(set = 2, binding = 1) uniform sampler s_tex_color;
layout(set = 3, binding = 0) uniform texture2D t_tex_float;
layout(set = 3, binding = 1) uniform sampler s_tex_float;

void main(){
    o_color = vec4(0.0, 0.0, 0.0, 1.0);

    vec4 tex_vpf = texture(sampler2D(t_tex_vpf, s_tex_vpf), f_uv * 2. - vec2(0., 0.));
    vec4 tex_color = texture(sampler2D(t_tex_color, s_tex_color), f_uv * 2. - vec2(1., 0.));
    vec4 tex_float = texture(sampler2D(t_tex_float, s_tex_float), f_uv * 2. - vec2(0., 1.));
    vec4 tex_cf = texture(sampler2D(t_tex_color, s_tex_color), f_uv * 2. - vec2(1., 0.)) + texture(sampler2D(t_tex_float, s_tex_float), f_uv * 2. - vec2(1., 0.));
    float tex_f = texture(sampler2D(t_tex_vpf, s_tex_vpf), f_uv * 2. - vec2(1., 1.)).w;

    if(f_uv.x < 0.5 && f_uv.y < 0.5){
        o_color = tex_vpf;
    }
    else if(f_uv.x > 0.5 && f_uv.y < 0.5){
        o_color = tex_cf;
    }
    else if(f_uv.x < 0.5 && f_uv.y > 0.5){
        o_color = tex_float;
    }
    else if(f_uv.x > 0.5 && f_uv.y > 0.5){
        o_color = vec4(vec3(tex_f), 1.0);
    }
}
#endif
