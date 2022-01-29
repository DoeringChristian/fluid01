#version 460

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;

layout(location = 0) out vec4 o_color;

void main(){
    o_color = vec4(1.0, 0.0, 0.0, 1.0);
}

