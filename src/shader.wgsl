// Vertex shader
struct CameraUniform {
  view_proj: mat4x4<f32>,
  view_pos: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec4<f32>,
}
struct LightUniforms {
  lights: array<LightUniform, 2>,
}
@group(1) @binding(0) var<storage, read> sky: vec3<f32>;
@group(1) @binding(1) var<uniform> lights: LightUniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) color: vec4<f32>,
  @location(3) ao: f32,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) world_normal: vec3<f32>,
  @location(2) world_position: vec3<f32>,
  @location(3) ao: f32,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.color = model.color;
  out.world_normal = model.normal;
  out.world_position = model.position;
  out.ao = model.ao;
  out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
  return out;
}

fn light_color(light: LightUniform, pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
  let dir = normalize(light.position - pos);

  let diffuse_strength = max(dot(normal, dir), 0.0);
  return light.color.xyz * diffuse_strength * light.color.w;
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

fn get_texture_noise(pos: vec3<f32>, color: vec4<f32>) -> f32 {
    if color.w < 1.0 {
        // Water: Swirly
        let n1 = smooth_noise(pos * 4.0);
        return smooth_noise(pos * 8.0 + vec3<f32>(n1 * 5.0));
    } else if color.g > color.r && color.g > color.b && color.g > 0.4 {
        // Grass: Streaky (stretch the Y coordinate)
        let streaky_pos = vec3<f32>(pos.x * 16.0, pos.y * 2.0, pos.z * 16.0);
        return smooth_noise(streaky_pos);
    } else if color.r > 0.6 && color.g > 0.6 && color.b < 0.5 {
        // Sand: Grainy (high frequency, no interpolation for pixelated look)
        return hash(floor(pos * 32.0));
    }
    // Default (Dirt/Stone/Wood): slightly chunky
    return smooth_noise(pos * 16.0);
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let ambient_strength = 0.1;
  let ambient_color = sky * ambient_strength;

  var total_light = ambient_color;

  let lights = lights.lights;
  total_light += light_color(lights[0], in.world_position, in.world_normal);
  total_light += light_color(lights[1], in.world_position, in.world_normal);

  let noise_val = get_texture_noise(in.world_position, in.color);
  
  // Map noise from [0, 1] to something like [0.8, 1.1] to subtly perturb color
  let color_variation = mix(0.8, 1.1, noise_val);
  
  let base_color = in.color.xyz * color_variation;

  var result = total_light * base_color * in.ao;

  // Distance fog to blend chunks smoothly into the sky (using squared distance
  // to avoid slow sqrt)
  let d = camera.view_pos.xyz - in.world_position;
  let dist_sq = dot(d, d);
  // NOTE: The value 48.0 corresponds to CHUNK_LOAD_RADIUS * 16.
  let distance_fog_factor = smoothstep(40.0 * 40.0, 48.0 * 48.0, dist_sq);
  result = mix(result, sky, distance_fog_factor);

  // Underwater fog
  if (camera.view_pos.y < 32.0) {
      let fog_color = vec3<f32>(0.0, 0.2, 0.6);
      let fog_factor = 1.0 - exp(-sqrt(dist_sq) * 0.05);
      result = mix(result, fog_color, fog_factor);
  }

  return vec4<f32>(result, in.color.w);
}
