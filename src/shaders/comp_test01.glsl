#version 460
#if COMPUTE_SHADER

layout(set = 0, binding = 0) buffer InBuffer{
    int in_buffer[];
};
layout(set = 1, binding = 0) buffer OutBuffer{
    int out_buffer[];
};

void main(){
    uint i = gl_GlobalInvocationID.x;

    out_buffer[i] = in_buffer[i]+4;
}

#endif
