#version 460
#if VERTEX_SHADER
// #############################################################################
// VertexShader:
// #############################################################################

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
// #############################################################################
// FragmentShader:
// #############################################################################

layout(location = 0) in vec2 f_pos;
layout(location = 1) in vec2 f_uv;
layout(location = 2) in vec2 r;

layout(location = 0) out vec4 vo;
layout(location = 1) out vec4 o_color;
layout(location = 2) out vec4 o_float;

layout(set = 0, binding = 0) uniform GlobalData{
    vec2 size;
    float time;
} global_data;

layout(set = 1, binding = 0) uniform texture2D t_tex_vpf;
layout(set = 1, binding = 1) uniform sampler s_tex_vpf;
layout(set = 2, binding = 0) uniform texture2D t_tex_color;
layout(set = 2, binding = 1) uniform sampler s_tex_color;
layout(set = 3, binding = 0) uniform texture2D t_tex_float;
layout(set = 3, binding = 1) uniform sampler s_tex_float;

#define VMAXX 5.0
#define VMAXY 5.0
#define VMAX sqrt(VMAXX * VMAXX + VMAXY * VMAXY)

vec4 v(vec2 pos){
    return texture(sampler2D(t_tex_vpf, s_tex_vpf), pos/global_data.size);
}

vec4 tex(vec2 pos, texture2D t, sampler s){
    return texture(sampler2D(t, s), pos/global_data.size);
}

vec3 to_ymc(vec3 rgb){
    return vec3(1., 1., 1.) - rgb;
}

vec3 to_rgb(vec3 ymc){
    return vec3(1., 1., 1.) - ymc;
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
    //K *= vo.w;

    // mass conservation
    //           ((\nabla p) \cdot u)       + (p \cdot (\nabla u))
    vo.z -= dt * (dx.z * vo.x + dy.z * vo.y + vo.z * div );

    // -----------------------------------------------------------------------------
    // Semi-Langrangian Advection:
    //
    // semi-Langrangian advection for velocity field (shift the field allong the field)
    vo.xy = v(r - dt * vo.xy).xy;


    // -----------------------------------------------------------------------------
    // Viscosity/Diffusion:
    // for velocity field:
    vo.xy += dt * vec2(nu, nu) * lapl.xy;


    // -----------------------------------------------------------------------------
    // Nullify Divergence:
    vo.xy -= K * vec2(dx.z, dy.z);

    // Move fluidity along field:
    vo.w = v(r - dt * vo.xy).w;

    // -----------------------------------------------------------------------------
    // External Sources:
    // pen source: 
    vec2 m = pen(global_data.time);
    vo.xyw += dt * exp(-(dot(r-m, r-m))/50.) * vec3(m - pen(global_data.time-0.1), 1.);
    //vo.xyw += dt * exp(-(dot(r-m, r-m))/50.) * vec3(m - pen(global_data.time-0.1), 1.);
    //vo.z += exp(-(dot(r-m, r-m))/50.);

    vo.xyzw = clamp(vo.xyzw, vec4(-VMAXX, -VMAXY, 0.5, 0.), vec4(VMAXX, VMAXY, 3., 5.));
    
    // How much of the dried color is picked up.
    // (The faster the liquid the more it picks up)
    float pickup = length(vo.xy)/VMAX;
    // How much particulate is falling out.
    // (At slower velocities more particulate falls out)
    float fallout = (1. - vo.w / VMAX);
    vec4 brush_color = vec4(1.0, 0.0, 0.0, 0.1);

    // TODO: Alpha of float as ammount of particulate.
    // Add picked up particulate to floating particulate and remove fallout.

    o_float = tex(r - dt * vo.xy, t_tex_float, s_tex_float) * (1 - fallout) + tex(r, t_tex_color, s_tex_color) * pickup + brush_color * exp(-(dot(r-m, r-m)/50.));

    // TODO: Dry particulate over time so it is harder to pick up. (use vo.w as water level/fluidity)
    // Add Fallen out particulate to dried color and remove picked up particulate.
    //o_color.rgb = tex(r, t_tex_color, s_tex_color).rgb * (1. - pickup) + tex(r - dt * vo.xy, t_tex_float, s_tex_float).rgb * fallout;
    o_color.rgb = tex(r, t_tex_color, s_tex_color).rgb * (1 - pickup) + tex(r - dt * vo.xy, t_tex_float, s_tex_float).rgb * fallout;

    vo.w *= 0.999;
}
#endif
