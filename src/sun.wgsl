struct CameraUniform {
  view_proj: mat4x4<f32>,
  inv_view_proj: mat4x4<f32>,
  view_pos: vec4<f32>,
  water_level: f32,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec4<f32>,
}
struct LightUniforms {
  lights: array<LightUniform, 2>,
}
@group(1) @binding(0) var<storage, read> _unused: vec3<f32>;
// left unbound
@group(1) @binding(1) var<uniform> lights: LightUniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
  let dist = length(lights.lights[0].position - camera.view_pos.xyz);
  let scale = dist / 50.0;
  var out: VertexOutput;
  let world_pos = model.position * scale + lights.lights[0].position;
  out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
  out.color = vec4<f32>(lights.lights[0].color.xyz, 1.0);
  out.world_position = world_pos;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  if (in.world_position.y < camera.water_level) {
    discard;
  }
  return in.color;
}
