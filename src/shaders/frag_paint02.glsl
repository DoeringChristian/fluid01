#version 460

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;

layout(location = 0) out vec4 vo;
layout(location = 1) out vec4 o_uv;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
    float time;
} global_data;

layout(set = 1, binding = 0) uniform texture2D t_tex;
layout(set = 1, binding = 1) uniform sampler s_tex;
layout(set = 2, binding = 0) uniform texture2D t_tex_uv;
layout(set = 2, binding = 1) uniform sampler s_tex_uv;

vec4 v(vec2 pos){
    return texture(sampler2D(t_tex, s_tex), pos/global_data.size);
}

vec4 tex(vec2 pos, sampler s, texture2D t){
    return texture(sampler2D(t, s), pos/global_data.size);
}

vec2 pen(float t){
    return vec2(cos(t/4.) * 200, sin(t/4.) * 200) * cos(t/10.) + vec2(300, 300);
}


void main(){
    // x,y: velocity field,
    // z: preasure field,
    // w: fluidity

    float dt = 0.15;
    float K = 0.2;
    float nu = 0.5;
    float kappa = 0.5;

    if(f_uv.y > 0.5){
        //K = 0.01;
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

    // -----------------------------------------------------------------------------
    // Addjust the Viscosity and diffusion:
    //nu *= vo.w * 0.5;
    K *= vo.w;

    // mass conservation
    //           ((\nabla p) \cdot u)       + (p \cdot (\nabla u))
    vo.z -= dt * (dx.z * vo.x + dy.z * vo.y + vo.z * div );

    // -----------------------------------------------------------------------------
    // Semi-Langrangian Advection:
    //
    // for uv map.
    o_uv = tex(r - dt * vo.xy, s_tex_uv, t_tex_uv).xyzw;

    // semi-Langrangian advection for velocity field (shift the field allong the field)
    vo.xyw = v(r - dt * vo.xy).xyw;


    // -----------------------------------------------------------------------------
    // Viscosity/Diffusion:
    // for velocity field:
    vo.xyw += dt * vec3(nu, nu, kappa) * lapl.xyw;


    // -----------------------------------------------------------------------------
    // Nullify Divergence:
    vo.xy -= K * vec2(dx.z, dy.z);

    // -----------------------------------------------------------------------------
    // External Sources:
    // pen source: 
    vec2 m = pen(global_data.time);
    vo.xyw += dt * exp(-(dot(r-m, r-m))/50.) * vec3(m - pen(global_data.time-0.1), 1.);

    if(f_uv.y > 0.5){
    }

    vo.w -= dt * 0.0005;

    vo.xyzw = clamp(vo.xyzw, vec4(-5., -5., 0.5, 0.), vec4(5., 5., 3., 5.));
}
