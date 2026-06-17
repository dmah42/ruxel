struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
    fog_start_sq: f32,
    fog_end_sq: f32,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct SkyUniform {
    color: vec4<f32>,
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    d: vec4<f32>,
    e: vec4<f32>,
    z: vec4<f32>,
    sun_dir: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> sky: SkyUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec4<f32>,
}
struct LightUniforms {
  lights: array<LightUniform, 2>,
}
@group(1) @binding(1)
var<uniform> lights: LightUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) clip_pos: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vertex_index & 1u) << 2u);
    let y = f32((vertex_index & 2u) << 1u);
    out.clip_pos = vec2<f32>(x - 1.0, 1.0 - y);
    // Draw at z=1.0 to ensure it is behind everything. 
    // Depth test will be LessEqual to allow it to render where no opaque objects are.
    out.position = vec4<f32>(out.clip_pos, 1.0, 1.0); 
    return out;
}

fn perez(A: vec3<f32>, B: vec3<f32>, C: vec3<f32>, D: vec3<f32>, E: vec3<f32>, Z: vec3<f32>, sunDir: vec3<f32>, viewDir: vec3<f32>) -> vec3<f32> {
    let theta = acos(max(0.001, viewDir.y));
    let gamma = acos(clamp(dot(viewDir, sunDir), -1.0, 1.0));
    
    let term1 = 1.0 + A * exp(B / cos(theta));
    let term2 = 1.0 + C * exp(D * gamma) + E * pow(cos(gamma), 2.0);
    return term1 * term2;
}

fn YxyToRGB(Yxy: vec3<f32>) -> vec3<f32> {
    var rgb: vec3<f32>;
    let z = max(Yxy.z, 0.0001);
    rgb.r = Yxy.x * ( 3.2406 * Yxy.y - 1.5372 * z - 0.4986 * (1.0 - Yxy.y - z)) / z;
    rgb.g = Yxy.x * (-0.9689 * Yxy.y + 1.8758 * z + 0.0415 * (1.0 - Yxy.y - z)) / z;
    rgb.b = Yxy.x * ( 0.0557 * Yxy.y - 0.2040 * z + 1.0570 * (1.0 - Yxy.y - z)) / z;
    return rgb;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let clip = vec4<f32>(in.clip_pos.x, in.clip_pos.y, 0.5, 1.0);
    var world_ray = camera.inv_view_proj * clip;
    world_ray = world_ray / world_ray.w;
    let view_dir = normalize(world_ray.xyz - camera.view_pos.xyz);
    
    let A = vec3<f32>(sky.a.x, sky.a.y, sky.a.z);
    let B = vec3<f32>(sky.b.x, sky.b.y, sky.b.z);
    let C = vec3<f32>(sky.c.x, sky.c.y, sky.c.z);
    let D = vec3<f32>(sky.d.x, sky.d.y, sky.d.z);
    let E = vec3<f32>(sky.e.x, sky.e.y, sky.e.z);
    let Z = vec3<f32>(sky.z.x, sky.z.y, sky.z.z);
    let sun_dir = normalize(sky.sun_dir.xyz);
    
    // Calculate sky color using Perez model
    let f = perez(A, B, C, D, E, Z, sun_dir, view_dir);
    let f0 = perez(A, B, C, D, E, Z, sun_dir, vec3<f32>(0.0, 1.0, 0.0));
    let Yxy = Z * f / f0;
    var color = YxyToRGB(Yxy);

    // Apply exposure/tonemapping
    color = vec3<f32>(1.0) - exp(-color * 1.5);
    
    // Add night sky fading and stars
    let is_night = clamp(-sun_dir.y * 3.0, 0.0, 1.0);
    if (is_night > 0.0) {
        let moon_dir = normalize(lights.lights[1].position - camera.view_pos.xyz);
        let moon_halo = max(0.0, dot(view_dir, moon_dir));
        let moon_glow = pow(moon_halo, 400.0) * 0.15;
        
        let moon_height = max(0.0, moon_dir.y);
        let base_night_color = vec3<f32>(0.01, 0.02, 0.05) * (1.0 + moon_height * 0.5);
        let night_color = base_night_color + vec3<f32>(0.05, 0.06, 0.08) * moon_glow;
        
        color = mix(color, night_color, is_night);
    }

    return vec4<f32>(color, 1.0);
}
