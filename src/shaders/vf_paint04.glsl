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
#define HMIN 0.5

#define tex_vpf t_tex_vpf, s_tex_vpf
#define tex_color t_tex_color, s_tex_color
#define tex_float t_tex_float, s_tex_float

vec4 v(vec2 pos){
    return texture(sampler2D(t_tex_vpf, s_tex_vpf), pos/global_data.size);
}

vec4 tex(vec2 pos, texture2D t, sampler s){
    return textureLod(sampler2D(t, s), pos/global_data.size, 0);
}

vec3 to_ymc(vec3 rgb){
    return vec3(1., 1., 1.) - rgb;
}

vec3 to_rgb(vec3 ymc){
    return vec3(1., 1., 1.) - ymc;
}

vec2 pen(float t){
    //return vec2(300, 300);
    //return vec2(cos(t/4.) * 200, sin(t/4.) * 200) * cos(t/10.) + vec2(300, 300);
    return vec2(cos(t/4.) * 50, sin(t/4.) * 50) * cos(t/10.) + vec2(300, 300);
}

float gaus(float x){
    return exp(-(x * x));
}

float gaus2(vec2 r){
    return exp(-dot(r, r));
}

vec4 blur(vec2 r, vec2 scale, float cutoff, texture2D t, sampler s){
    float g = gaus2(vec2(0, 0));
    vec4 res = tex(r, t, s);
    for(int i = 1; g > cutoff; i++){
        for(int j = 1; g > cutoff; j++){
            g = gaus2(vec2(i, j) * scale);
            res += tex(r + vec2(i, j), t, s) * g;
            res += tex(r + vec2(i, -j), t, s) * g;
            res += tex(r + vec2(-i, j), t, s) * g;
            res += tex(r + vec2(-i, -j), t, s) * g;
        }
        g = gaus2(vec2(i, 0) * scale);
    }
    return res;
}

void main(){
    /*
    sampler2D tex_vpf = sampler2D(t_tex_vpf, s_tex_vpf);
    sampler2D tex_color = sampler2D(t_tex_color, s_tex_color);
    sampler2D tex_float = sampler2D(t_tex_float, s_tex_float);
    */

    // x,y: velocity field,
    // z: preasure field,
    // w: fluidity

    float dt = 0.15;
    //float K = 0.2;
    float K = 0.3;
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

    // mass conservation. z is equivalent to the column hight.
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

    // DEBUG:
    o_color.r = dx.z;
    o_color.g = dy.z;

    // -----------------------------------------------------------------------------
    // External Sources:
    // pen source: 
    vec2 m = pen(global_data.time);
    //vo.xyw += dt * exp(-(dot(r-m, r-m))/50.) * vec3(m - pen(global_data.time-0.1), 2.);
    if(global_data.time < 15.){
        vo.z += exp(-(dot(r-m, r-m))/50.) * 2.;
    }
    //vo.xy += dt * exp(-(dot(r-m, r-m))/50.) * vec2(m - pen(global_data.time-0.1));
    //vo.z += exp(-(dot(r-m, r-m))/50.);

    vo.xyz = clamp(vo.xyz, vec3(-VMAXX, -VMAXY, HMIN), vec3(VMAXX, VMAXY, 3.));

    // w is the wet area mask
    if(vo.z >= HMIN + 0.5 || vo.w >= 0.5)
        vo.w = 1.00;
    else
        vo.w = 0.0;
    if(vo.z < HMIN){
        vo.w = 0.0;
        vo.z = HMIN;
    }
    /*
    if(vo.z <= HMIN + 0.2)
        vo.w = 0.0;
    else
        vo.w = 1.0;
        */
    //vo.w = vo.z < HMIN + 0e-5? 0.0: 1.0;

    // apply boundary condiation to dry areas of paper.
    // Von Neumann Boundary Condition.
    if(vo.w < 0.5){
        vo.xy = vec2(0., 0.);
    }

    
    
    // Evapuration:
    float evap_nu = 0.01;
    //float evap_nu = 0.0;
    vo.z = vo.z - evap_nu * (1 - v(r).w)*vo.w;

    // DEBUG:
    o_color.b = (1 - v(r).w)*vo.w;
    

    vec4 brush_color = vec4(1.0, 0.0, 0.0, 0.1);

    vec4 fl = tex(r, tex_float);
    vec4 float_px = tex(r + vec2(1., 0.), tex_float); 
    vec4 float_nx = tex(r + vec2(-1., 0.), tex_float);
    vec4 float_py = tex(r + vec2(0., 1.), tex_float); 
    vec4 float_ny = tex(r + vec2(0., -1.), tex_float);

    vec4 float_dx = (float_px - float_nx)/2.0;
    vec4 float_dy = (float_py - float_ny)/2.0;

    vec4 float_lapl = (float_px + float_nx + float_py + float_ny - 4.*fl);

    float float_div = float_dx.x + float_dy.y;

    // Adjust the diffusion coefficient of the pigment according to the height of the fluid.
    //float float_nu = (vo.z - HMIN) / 2.5 * 2.;
    float float_nu = 0.0;
    // Diffusion coefficient should never be over 1.
    if(vo.z > 0.5001){
        float_nu = 0.001;
    }
    //float_nu = 1.0;
    

    // advection: for some reason no pigment is carried away to the edges of the liquid.
    // Wtf... why do I need this multiplicand (3.)? Implies that pigment moves faster than liquid.
    vec2 vo_s = vo.xy;
    o_float = tex(r - dt * vo_s, tex_float);
    //o_float = tex(r - dt * vo.xy, tex_float);
    // diffusion
    o_float += dt * float_nu * float_lapl;

    if(global_data.time < 10.)
        o_float += brush_color * dt * exp(-(dot(r-m, r-m))/50.) * 0.002;
    
    /* DEBUG:
    if(length(r - vec2(300, 300)) < 10 && global_data.time < 5){
        o_float = vec4(10, 0, 0, 1);
    }
    */
}
#endif
