struct CameraUniform {
  view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec3<f32>,
}
@group(1) @binding(0) var<uniform> _unused: LightUniform;
// left unbound
@group(1) @binding(1) var<uniform> moon: LightUniform;

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
  let scale = 1.0;
  var out: VertexOutput;
  out.clip_position = camera.view_proj * vec4<f32>(model.position * scale + moon.position, 1.0);
  out.color = vec4<f32>(moon.color, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color;
}
