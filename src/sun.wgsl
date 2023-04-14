struct CameraUniform {
  view_proj: mat4x4<f32>,
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
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
  let scale = 2.0;
  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + lights.lights[0].position, 1.0);
  out.color = vec4<f32>(lights.lights[0].color.xyz, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color;
}
