// Vertex shader
struct CameraUniform {
  view_proj: mat4x4<f32>,
  inv_view_proj: mat4x4<f32>,
  view_pos: vec4<f32>,
  water_level: f32,
  fog_start_sq: f32,
  fog_end_sq: f32,
  padding: f32,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec4<f32>,
}
struct LightUniforms {
  lights: array<LightUniform, 2>,
}
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
@group(1) @binding(0) var<uniform> sky: SkyUniform;
@group(1) @binding(1) var<uniform> lights: LightUniforms;

struct MainShadowUniform {
  sun_view_proj: mat4x4<f32>,
  moon_view_proj: mat4x4<f32>,
}
@group(2) @binding(0) var sun_shadow_map: texture_depth_2d;
@group(2) @binding(1) var moon_shadow_map: texture_depth_2d;
@group(2) @binding(2) var shadow_sampler: sampler_comparison;
@group(2) @binding(3) var<uniform> shadow_cameras: MainShadowUniform;

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

fn get_sky_color(view_dir: vec3<f32>) -> vec3<f32> {
    let A = vec3<f32>(sky.a.x, sky.a.y, sky.a.z);
    let B = vec3<f32>(sky.b.x, sky.b.y, sky.b.z);
    let C = vec3<f32>(sky.c.x, sky.c.y, sky.c.z);
    let D = vec3<f32>(sky.d.x, sky.d.y, sky.d.z);
    let E = vec3<f32>(sky.e.x, sky.e.y, sky.e.z);
    let Z = vec3<f32>(sky.z.x, sky.z.y, sky.z.z);
    let sun_dir = normalize(sky.sun_dir.xyz);
    
    let f = perez(A, B, C, D, E, Z, sun_dir, view_dir);
    let f0 = perez(A, B, C, D, E, Z, sun_dir, vec3<f32>(0.0, 1.0, 0.0));
    let Yxy = Z * f / f0;
    var color = YxyToRGB(Yxy);

    color = vec3<f32>(1.0) - exp(-color * 1.5);
    
    let is_night = clamp(-sun_dir.y * 3.0, 0.0, 1.0);
    if (is_night > 0.0) {
        let moon_dir = normalize(lights.lights[1].position - camera.view_pos.xyz);
        let moon_height = max(0.0, moon_dir.y);
        let night_color = vec3<f32>(0.01, 0.02, 0.05) * (1.0 + moon_height * 0.5);
        color = mix(color, night_color, is_night);
    }
    return color;
}

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) material: u32,
  @location(2) color: vec4<f32>,
  @location(3) normal_and_ao: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) world_normal: vec3<f32>,
  @location(2) world_position: vec3<f32>,
  @location(3) ao: f32,
  @location(4) @interpolate(flat) material: u32,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.color = model.color;
  out.world_normal = model.normal_and_ao.xyz;
  out.world_position = model.position;
  out.material = model.material;
  
  // AO is mapped from 0..127 to 0.0..1.0 by the Snorm format
  // Negative values shouldn't happen, but we max with 0 just in case
  out.ao = max(model.normal_and_ao.w, 0.0);

  out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
  return out;
}

fn light_color(light: LightUniform, pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
  let dir = normalize(light.position - pos);

  // Fade out the light as it dips below the horizon line
  let horizon_fade = smoothstep(-0.1, 0.1, dir.y);

  let diffuse_strength = max(dot(normal, dir), 0.0) * horizon_fade;
  return light.color.xyz * diffuse_strength * light.color.w;
}

fn specular_color(light: LightUniform, pos: vec3<f32>, normal: vec3<f32>, view_dir: vec3<f32>, shininess: f32) -> vec3<f32> {
    let light_dir = normalize(light.position - pos);
    if (dot(normal, light_dir) <= 0.0) {
        return vec3<f32>(0.0);
    }
    
    let h = normalize(light_dir + view_dir);
    let horizon_fade = smoothstep(-0.1, 0.1, light_dir.y);
    
    let spec_factor = pow(max(dot(normal, h), 0.0), shininess);
    return light.color.xyz * spec_factor * light.color.w * horizon_fade;
}

fn hash(p: vec3<f32>) -> f32 {
    let p2 = fract(p * 0.1031);
    let p3 = p2 + dot(p2, p2.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn smooth_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 0.0)), hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

fn get_texture_noise(pos: vec3<f32>, material: u32) -> f32 {
    if material == 5u {
        // Water: Swirly
        let n1 = smooth_noise(pos * 4.0);
        return smooth_noise(pos * 8.0 + vec3<f32>(n1 * 5.0));
    } else if material == 4u {
        // Ice: Smooth
        return 0.5;
    } else if material == 2u {
        // Grass: Streaky (stretch the Y coordinate)
        let streaky_pos = vec3<f32>(pos.x * 16.0, pos.y * 2.0, pos.z * 16.0);
        return smooth_noise(streaky_pos);
    } else if material == 1u {
        // Sand: Grainy (high frequency, no interpolation for pixelated look)
        return hash(floor(pos * 32.0));
    }
    // Default (Rock/Dirt/Wood): slightly chunky
    return smooth_noise(pos * 16.0);
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let ambient_strength = 0.1;
  let ambient_color = sky.color.xyz * ambient_strength;

  var total_diffuse = ambient_color;

  let lights = lights.lights;
  
  // Sun shadow mapping
  let sun_shadow_pos = shadow_cameras.sun_view_proj * vec4<f32>(in.world_position, 1.0);
  let sun_ndc = sun_shadow_pos.xyz / sun_shadow_pos.w;
  let sun_shadow_uv = vec2<f32>(sun_ndc.x * 0.5 + 0.5, sun_ndc.y * -0.5 + 0.5);
  var sun_shadow_factor = 1.0;
  
  if (sun_shadow_uv.x >= 0.0 && sun_shadow_uv.x <= 1.0 && sun_shadow_uv.y >= 0.0 && sun_shadow_uv.y <= 1.0 && sun_ndc.z >= 0.0 && sun_ndc.z <= 1.0) {
      sun_shadow_factor = textureSampleCompare(sun_shadow_map, shadow_sampler, sun_shadow_uv, sun_ndc.z - 0.005);
  }

  // Moon shadow mapping
  let moon_shadow_pos = shadow_cameras.moon_view_proj * vec4<f32>(in.world_position, 1.0);
  let moon_ndc = moon_shadow_pos.xyz / moon_shadow_pos.w;
  let moon_shadow_uv = vec2<f32>(moon_ndc.x * 0.5 + 0.5, moon_ndc.y * -0.5 + 0.5);
  var moon_shadow_factor = 1.0;
  
  if (moon_shadow_uv.x >= 0.0 && moon_shadow_uv.x <= 1.0 && moon_shadow_uv.y >= 0.0 && moon_shadow_uv.y <= 1.0 && moon_ndc.z >= 0.0 && moon_ndc.z <= 1.0) {
      moon_shadow_factor = textureSampleCompare(moon_shadow_map, shadow_sampler, moon_shadow_uv, moon_ndc.z - 0.005);
  }

  total_diffuse += light_color(lights[0], in.world_position, in.world_normal) * sun_shadow_factor;
  total_diffuse += light_color(lights[1], in.world_position, in.world_normal) * moon_shadow_factor;

  let view_dir = normalize(camera.view_pos.xyz - in.world_position);
  var spec_strength = 0.0;
  var shininess = 1.0;
  
  if (in.material == 5u) { // Water
      spec_strength = 0.8;
      shininess = 16.0;
  } else if (in.material == 4u) { // Ice
      spec_strength = 1.2;
      shininess = 32.0;
  }

  var total_specular = vec3<f32>(0.0);
  if (spec_strength > 0.0) {
      total_specular += specular_color(lights[0], in.world_position, in.world_normal, view_dir, shininess) * sun_shadow_factor;
      total_specular += specular_color(lights[1], in.world_position, in.world_normal, view_dir, shininess) * moon_shadow_factor;
      total_specular *= spec_strength;
  }

  let noise_val = get_texture_noise(in.world_position, in.material);
  
  // Map noise from [0, 1] to something like [0.8, 1.1] to subtly perturb color
  let color_variation = mix(0.8, 1.1, noise_val);
  
  let base_color = in.color.xyz * color_variation;

  var result = total_diffuse * base_color * in.ao + total_specular;

  // Distance fog to blend chunks smoothly into the sky (using squared distance
  // to avoid slow sqrt)
  let d = camera.view_pos.xyz - in.world_position;
  let dist_sq = dot(d, d);
  let distance_fog_factor = smoothstep(camera.fog_start_sq, camera.fog_end_sq, dist_sq);
  if (distance_fog_factor > 0.0 || camera.view_pos.y < 32.0) {
    let view_dir_to_fragment = normalize(-d);
    let fog_sky_color = get_sky_color(view_dir_to_fragment);
    if (distance_fog_factor > 0.0) {
        result = mix(result, fog_sky_color, distance_fog_factor);
    }

    // Underwater fog
    if (camera.view_pos.y < 32.0) {
        let fog_color = fog_sky_color * vec3<f32>(0.2, 0.5, 1.0);
        let fog_factor = 1.0 - exp(-sqrt(dist_sq) * 0.05);
        result = mix(result, fog_color, fog_factor);
    }
  }

  return vec4<f32>(result, in.color.w);
}
