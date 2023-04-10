// Vertex shader
struct CameraUniform {
  view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct LightUniform {
  position: vec3<f32>,
  color: vec3<f32>,
}
@group(1) @binding(0) var<uniform> sun: LightUniform;
@group(1) @binding(1) var<uniform> moon: LightUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
};

struct InstanceInput {
  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>,

  @location(9) normal_matrix_0: vec3<f32>,
  @location(10) normal_matrix_1: vec3<f32>,
  @location(11) normal_matrix_2: vec3<f32>,

  @location(12) color: vec4<f32>,
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) world_normal: vec3<f32>,
  @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );

  let normal_matrix = mat3x3<f32>(
    instance.normal_matrix_0,
    instance.normal_matrix_1,
    instance.normal_matrix_2,
  );

  var out: VertexOutput;
  out.color = instance.color;
  out.world_normal = normal_matrix * model.normal;
  var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
  out.world_position = world_position.xyz;
  out.clip_position = camera.view_proj * world_position;
  return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let ambient_strength = 0.1;
  let ambient_color = moon.color * sun.color * ambient_strength;

  let sun_dir = normalize(sun.position - in.world_position);

  let sun_diffuse_strength = max(dot(in.world_normal, sun_dir), 0.0);
  let sun_diffuse_color = sun.color * sun_diffuse_strength;

  let moon_dir = normalize(moon.position - in.world_position);

  let moon_diffuse_strength = max(dot(in.world_normal, moon_dir), 0.0);
  let moon_diffuse_color = moon.color * moon_diffuse_strength;

  let result = (ambient_color + sun_diffuse_color + moon_diffuse_color) * in.color.xyz;

  return vec4<f32>(result, in.color.w);
}
