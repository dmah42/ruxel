struct ShadowUniform {
  view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> shadow_camera: ShadowUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) material: u32,
  @location(2) color: vec4<f32>,
  @location(3) normal_and_ao: vec4<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> @builtin(position) vec4<f32> {
  return shadow_camera.view_proj * vec4<f32>(model.position, 1.0);
}
