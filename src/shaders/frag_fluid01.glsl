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



#if 1
/******** 3d simplex noise from https://www.shadertoy.com/view/XsX3zB ********/

/* discontinuous pseudorandom uniformly distributed in [-0.5, +0.5]^3 */
vec3 random3(vec3 c) {
	float j = 4096.0*sin(dot(c,vec3(17.0, 59.4, 15.0)));
	vec3 r;
	r.z = fract(512.0*j);
	j *= .125;
	r.x = fract(512.0*j);
	j *= .125;
	r.y = fract(512.0*j);
	return r-0.5;
}

/* skew constants for 3d simplex functions */
const float F3 =  0.3333333;
const float G3 =  0.1666667;

/* 3d simplex noise */
float simplex3d(vec3 p) {
	 /* 1. find current tetrahedron T and it's four vertices */
	 /* s, s+i1, s+i2, s+1.0 - absolute skewed (integer) coordinates of T vertices */
	 /* x, x1, x2, x3 - unskewed coordinates of p relative to each of T vertices*/
	 
	 /* calculate s and x */
	 vec3 s = floor(p + dot(p, vec3(F3)));
	 vec3 x = p - s + dot(s, vec3(G3));
	 
	 /* calculate i1 and i2 */
	 vec3 e = step(vec3(0.0), x - x.yzx);
	 vec3 i1 = e*(1.0 - e.zxy);
	 vec3 i2 = 1.0 - e.zxy*(1.0 - e);
	 	
	 /* x1, x2, x3 */
	 vec3 x1 = x - i1 + G3;
	 vec3 x2 = x - i2 + 2.0*G3;
	 vec3 x3 = x - 1.0 + 3.0*G3;
	 
	 /* 2. find four surflets and store them in d */
	 vec4 w, d;
	 
	 /* calculate surflet weights */
	 w.x = dot(x, x);
	 w.y = dot(x1, x1);
	 w.z = dot(x2, x2);
	 w.w = dot(x3, x3);
	 
	 /* w fades from 0.6 at the center of the surflet to 0.0 at the margin */
	 w = max(0.6 - w, 0.0);
	 
	 /* calculate surflet components */
	 d.x = dot(random3(s), x);
	 d.y = dot(random3(s + i1), x1);
	 d.z = dot(random3(s + i2), x2);
	 d.w = dot(random3(s + 1.0), x3);
	 
	 /* multiply d by w^4 */
	 w *= w;
	 w *= w;
	 d *= w;
	 
	 /* 3. return the sum of the four surflets */
	 return dot(d, vec4(52.0));
}

/*****************************************************************************/


vec2 pen(float t){
    return vec2(cos(t) * 200, sin(t) * 200) * cos(t * 3.1415926) + vec2(300, 300);
}
#endif







#define K 0.2
#define nu 0.5
#define kappa 0.1
#define dt 0.15
#define length2(p) dot(p, p)

void main(){
    vec2 p = f_uv * global_data.size;

    vo = v(p);

    vec4 vpx = v(p + vec2(1.0, 0.0));
    vec4 vnx = v(p + vec2(-1.0, 0.0));
    vec4 vpy = v(p + vec2(0.0, 1.0));
    vec4 vny = v(p + vec2(0.0, -1.0));

    vec4 lap = (vpx + vnx + vpy + vny - 4*v(p));

    vec4 dx = (vpx - vnx) / 2.0;
    vec4 dy = (vpy - vny) / 2.0;

    float div = dx.x + dy.y;

    vo.z -= dt*(dx.z * vo.x + dy.z * vo.y + div * vo.z);

    vo.xy = v(p - dt*vo.xy).xy;

    vo.xy += dt * vec2(nu, nu) * lap.xy;

    vo.xy -= K * vec2(dx.z, dy.z);

    vec2 m = pen(global_data.time);
    vo.xy += dt * exp(-length2(p - m)/50.0) * vec2(m - pen(global_data.time - 0.1));

    vo.xyzw = clamp(vo.xyzw, vec4(-5.0, -5.0, 0.5, 0.0), vec4(5.0, 5.0, 3.0, 5.0));
}
