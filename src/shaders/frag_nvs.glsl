#version 460

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;

layout(location = 0) out vec4 vo;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
    float time;
} global_data;

layout(set = 1, binding = 0) uniform texture2D t_tex;
layout(set = 1, binding = 1) uniform sampler s_tex;

vec4 v(vec2 pos){
    return texture(sampler2D(t_tex, s_tex), pos/global_data.size);
}

vec2 pen(float t){
    return vec2(cos(t) * 200, sin(t) * 200) * cos(t * 3.1415926) + vec2(300, 300);
}


void main(){
    float K = 0.2;
    float nu = 0.5;
    float dt = 0.15;

    if(f_uv.y > 0.5){
        //K = 0.01;
    }

    if(global_data.time > 10.0){
        K = 0.0;
        nu = 0.0;
    }

    vec2 r = f_uv * global_data.size;

    vo = v(r);

    vec4 vpx = v(r + vec2(1., 0.)); 
    vec4 vnx = v(r + vec2(-1., 0.));
    vec4 vpy = v(r + vec2(0., 1.)); 
    vec4 vny = v(r + vec2(0., -1.));

    //     | vpy  |
    // ----|------|-----
    // vnx | v(r) | vpx
    // ----|------|-----
    //     | vny  | 
    //

    vec4 dx = (vpx - vnx)/2.0;
    vec4 dy = (vpy - vny)/2.0;

    vec4 lapl = (vpx + vnx + vpy + vny - 4.*vo);

    float div = dx.x + dy.y;

    // mass conservation
    //           ((\nabla p) \cdot u)       + (p \cdot (\nabla u))
    vo.z -= dt * (dx.z * vo.x + dy.z * vo.y + vo.z * div );

    // semi-Langrangian advection (shift the field allong the field)
    vo.xy = v(r - dt * vo.xy).xy;

    // viscosity/diffusion
    vo.xy += dt * vec2(nu, nu) * lapl.xy;

    // nullify divergence
    vo.xy -= K * vec2(dx.z, dy.z);

    // external source
    vec2 m = pen(global_data.time);
    vo.xy += dt * exp(-(dot(r-m, r-m))/50.) * vec2(m - pen(global_data.time-0.1));

    if(f_uv.y > 0.5){
    }

    vo.xyzw = clamp(vo.xyzw, vec4(-5, -5, 0.5, 0), vec4(5, 5, 3, 5));
}
