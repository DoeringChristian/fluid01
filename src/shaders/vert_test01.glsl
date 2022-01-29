#version 460

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;

layout(location = 0) out vec2 f_pos;
layout(location = 1) out vec2 f_uv;

void main(){
    f_pos = i_pos;
    f_uv = i_uv;

    gl_Position = vec4(i_pos, 0.0, 1.0);
}
