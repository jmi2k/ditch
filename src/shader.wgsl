struct PushConstants {
    camera: mat4x4f,
    viewport: vec2f,
    time: f32,
};

var<push_constant> constants: PushConstants;

const ZNEAR: f32 = 1e-1;
const ZFAR: f32 = 1e4;

struct V2F {
    @builtin(position) xyz: vec4f,
    @location(0) uv: vec2f,
    @location(1) shadow: f32,
    @location(2) light: u32,
};

@vertex
fn vertex(
    @location(0) xyz: vec3f,
    @location(1) uv: vec2f,
    @location(2) shadow: f32,
    @location(3) light: u32,
) -> V2F {
    //return v2f;
    return V2F(constants.camera * vec4f(xyz, 1.0), uv, shadow, light);
}

@group(0) @binding(0)
var atlas: texture_2d<f32>;
@group(0) @binding(1)
var samp: sampler;

const E: f32 = 2.71828182845904523536028747135266250;

@fragment
fn fragment(v: V2F) -> @location(0) vec4f {
    let fov = 90.;
    let fog = 128.;
    let density = 0.5;
    let base_fog = 0.01;

    let z0 = 2. * v.xyz.z - 1.;
    let z1 = 2. * ZNEAR * ZFAR / (ZFAR + ZNEAR - z0 * (ZFAR - ZNEAR)) / fog;
    let rgba = textureSample(atlas, samp, v.uv);
    if rgba.a == 0.0 { discard; }
    let light = log((E - 1.) * (f32(v.light) + 0.25) / 15.25 + 1.);
    let color = rgba * vec4f(v.shadow, v.shadow, v.shadow, 1.) * light;
    let center = constants.viewport / 2.0 - 0.5;
    let focal_length = (constants.viewport.y / 2.0) / tan(fov / 2.0);
    let diagonal = length(vec3(v.xyz.x - center.x,
                               v.xyz.y - center.y,
                               focal_length));
    let z2 = z1 * (diagonal / focal_length);
    let z3 = 1. - pow(2., -pow((z2 * density), 2.));
    let z = z3 * (1. - base_fog) + base_fog;

    //return vec4f(color.r, color.g, color.b, 1.);
    return mix(color, vec4f(0.527, 0.805, 0.918, 1.), clamp(z, 0., 1.));
}
