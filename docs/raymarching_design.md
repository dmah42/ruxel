# Raymarching Renderer Design

This document outlines the detailed architectural design and implementation plan for migrating `ruxel` from its greedy-meshed rasterization pipeline to a voxel raymarching engine. This serves as the technical foundation for future features such as Signed Distance Fields (SDFs), dynamic world simulations, and advanced lighting.

## Git Workflow

Because this is a massive engine rewrite that touches rendering, chunk management, and shaders, all work should be isolated.

- **Branch**: `feature/raymarching-renderer`.
- **Reason**: This ensures `main` remains stable while iterating on the complex Digital Differential Analyzer (DDA) algorithms and memory structures.

## Core Architecture Decisions

### 1. Voxel Data Structure: 3D Texture Ring Buffer

We store the active 3D voxel grid on the GPU using a single large `wgpu::Texture` with `TextureDimension::D3`. This acts as a toroidal "ring buffer" centered on the player's current chunk.

- **Why**: Voxel engines thrive on fast inner loops. A 3D texture allows for extremely fast hardware texture fetches, native 3D boundary clamping, and mathematically simple DDA traversal.
- **Memory Footprint**: At a `load_radius` of 4 (9x9 chunks), the active area is 144x128x144 blocks. At 1 byte per block ID, it uses ~2.6 MB. Even at a massive radius of 32 (65x65 chunks), it uses ~138 MB. The memory cost is negligible on both modern integrated and dedicated hardware compared to the massive performance gain of a single texture fetch per ray step.
- **Updates**: When the player crosses a chunk boundary, the ring buffer's logical center shifts. New chunks are loaded and their data is written into the 3D texture using `queue.write_texture`, overwriting the oldest chunks that fell out of range.

### 2. Transparency Strategy: Pure DDA

We handle transparent blocks (e.g., water, ice) entirely within the raymarching fragment shader without falling back to rasterized meshes.

- **Why**: When a ray hits a water block, it calculates the water color, then *continues marching* through the water until it hits a solid block or exits the world. This is physically accurate, allows for true refraction, thick volumetric glass, and interacting SDFs underwater. It maintains architectural purity by using one unified rendering pipeline for all voxel logic.
- **Trade-offs**: We accept the potential for GPU thread divergence and performance penalties when looking through deep bodies of water. The simplicity of a single unified shader loop outweighs the complexity of a hybrid rasterization approach.

## Component Implementation Details

### `src/render_state.rs` (Render Pipeline)

The core rendering pipeline is overhauled to support full-screen quad raymarching over the 3D texture.

- **Voxel Volume Texture**: Introduce a `wgpu::Texture` with `TextureDimension::D3`. Its physical dimensions are fixed to `((load_radius * 2 + 1) * 16, 128, (load_radius * 2 + 1) * 16)`.
- **Texture Bindings**: Create a new bind group to pass the 3D texture and a nearest-neighbor sampler (`wgpu::Sampler` with `FilterMode::Nearest`). Also pass a uniform containing the player's world `chunk_position` offset, allowing the shader to map world-space coordinates to the toroidal ring buffer.
- **Texture Updates**: In the `update()` loop, monitor the chunk cache. When chunks are modified (block broken/placed) or freshly loaded, use `queue.write_texture` to upload the 16x128x16 byte array directly into the corresponding sub-region of the 3D texture.
- **Pipeline Replacement**: Remove the old `render_pipeline` and `transparent_pipeline`. Create a single `raymarch_pipeline`.
- **Integration with Existing Pipelines**:
  - The Raymarcher draws *before* the Sky pipeline, replacing the old landscape rasterizer pass.
  - The Raymarch fragment shader calculates the exact intersection distance, converts it to a standard non-linear depth value, and explicitly outputs it to `gl_FragDepth`.
  - If a ray exits the voxel volume without hitting a block, the shader calls `discard`.
  - **Sky, Sun, Moon**: Because the raymarcher writes accurate depth and discards on empty sky pixels, the existing `sun_render_pipeline`, `moon_render_pipeline`, and `sky_render_pipeline` continue to function seamlessly via standard depth testing and alpha blending. No changes to their shaders are required.
  - **Wireframe**: The existing block selection wireframe draws on top of everything. Because our raymarcher writes `gl_FragDepth`, the wireframe will correctly depth-test against and overlay the raymarched blocks without any code changes.

### `src/mesh.rs` (CPU Meshing)

Since the GPU directly traverses raw voxel data, CPU polygon generation is obsolete.

- **Action**: Delete the greedy meshing algorithm entirely. `ChunkMesh`, `opaque_indices`, and `transparent_indices` are removed.

### `src/chunks.rs` (Chunk Management)

Minimal structural changes are needed here, primarily formatting data for the GPU.

- **Data Casting**: Ensure the inner block arrays (`[[[Block; 16]; 16]; 16]`) can be safely cast as contiguous byte slices (`bytemuck::cast_slice`). `Block` needs to be represented as a simple `u8` (or `u32` if we add metadata later) so it can be swiftly uploaded via `wgpu::Queue::write_texture`.

### `src/shader.wgsl` (The Raymarcher)

The shader is rewritten from a standard triangle rasterizer to a full DDA raymarcher.

- **Vertex Shader (`vs_main`)**: Ignore incoming vertex buffers. Use the built-in `vertex_index` to generate a single full-screen triangle. Calculate the world-space `ray_direction` from the screen UVs using the inverse projection and inverse view matrices. Pass the `ray_origin` (camera position) and `ray_direction` to the fragment shader.
- **Fragment Shader (`fs_main`)**:
  - **AABB Intersect**: First, mathematically intersect the ray with the bounding box of the active loaded voxel volume to find the entry and exit points. If it misses, `discard`.
  - **DDA Traversal**: Implement the Amanatides & Woo 3D DDA algorithm. Step voxel-by-voxel along the ray.
  - **Texture Fetch**: At each step, convert the world voxel coordinate to the local 3D texture coordinate (using a modulo operation to handle the ring buffer wrapping) and fetch the block ID.
  - **Pure DDA Transparency**: If a transparent block is hit (like water), accumulate its color and continue the DDA loop. If a solid block is hit, apply its color and break the loop. If the ray exits the volume, blend the accumulated transparent colors with the sky color.
  - **Normals**: Calculate the surface normal mathematically based on the DDA step mask (which axis the ray just crossed).
  - **Lighting & Noise**: Apply the existing lighting functions, specular highlights, and `get_texture_noise()`.
  - **Depth Writing**: Convert the total intersection distance into a normalized depth value and write it to `gl_FragDepth`.
  - **Fog & Underwater Fog**: The raymarcher knows the exact `world_position` of the hit block. We will port the exact same fog logic currently in the shader to blend the final output into the sky:

      ```wgsl
      let dist_sq = dot(camera.view_pos.xyz - world_position, camera.view_pos.xyz - world_position);
      // Distance fog logic blending to get_sky_color()
      // Underwater fog (view_pos.y < 32.0) applying blue tint based on dist_sq
      ```

## Verification Plan

### Automated Tests

- Run `make test_release` to verify compilation and cargo tests pass, ensuring no structural breaks in `chunks.rs` or `render_state.rs`.

### Manual Verification

1. **Opaque Terrain Check**: Verify opaque terrain renders without holes and visually matches the old meshing renderer.
2. **Chunk Boundary Check**: Walk across chunk boundaries to ensure the 3D texture ring buffer wraps and updates seamlessly without artifacts.
3. **Real-time Update Check**: Place and destroy blocks to verify real-time 3D texture updates via `write_texture`.
4. **Transparency Check**: Verify water and ice render correctly via pure DDA accumulation.
5. **Sky/Sun/Moon Check**: Verify the sun, moon, and sky gradient render correctly in the background where the ray hits nothing.
6. **Distance Fog Check**: Verify distance fog correctly obscures chunks at the edge of the load radius.
7. **Underwater Fog Check**: Verify the screen tints blue and visibility decreases dynamically when the camera dips below water level (`y < 32.0`).
8. **Wireframe Check**: Verify the block selection wireframe draws correctly on the raymarched blocks.
