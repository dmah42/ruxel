struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // 6 vertices for a quad covering top 42%
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 0.16),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 0.16),
        vec2<f32>(1.0, 0.16)
    );
    
    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.85); // 85% black background
}
