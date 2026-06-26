# Trees & Flora (Decorators) Deep Dive - Iteration 11

This document represents the uncompromisingly detailed architectural blueprint for decorators in `ruxel`. All previous details have been retained and synthesized to ensure no depth is lost.

The choice between **Path A (Voxels)** and **Path B (Entities)** dictates the entire pipeline. Both paths share the same inventory loop: destroying the tree yields items (e.g., "Wood"), which the player can place as standard voxel blocks to build structures.

**DECISION LOCKED IN:** We have chosen **Path B (Entities)** as the primary architectural direction for decorators
---

# PATH A (deprecated): The Pure Voxel Architecture

Decorators are specific arrangements of integer Block IDs (e.g., `OakWood`, `Leaves`) injected directly into the `Chunk`'s 3D data array (`chunk.blocks[x][y][z]`). The engine treats them identically to dirt or stone.

## A.1 Placement: Deterministic Candidates (LOCKED)

Because trees are blocks within chunks, a tree generated on the edge of Chunk A will have blocks that overhang into Chunk B.

* **Mechanism:** We evaluate a global, deterministic noise function for the entire world space to find potential "candidate tree origins". When Chunk B is executing its chunk generation thread, it does not just look at its own data; it queries the candidate origins in a wide radius (e.g., up to 16 blocks into neighboring chunks). If it calculates that a tree origin in Chunk A mathematically overlaps into Chunk B's space, Chunk B injects those leaf blocks into its own block array.
* **Pros:** Computationally decoupled. Chunks do not need to wait for their neighbors to generate; they simply compute the overlap deterministically based on the global seed. Perfectly parallelizable.
* **Cons:** Requires querying the noise function over a larger area than just the current chunk, introducing slight CPU overhead during the generation phase.

## A.2 Distribution Logic (Integer Grid)

Distributions must resolve to absolute integer block coordinates `(x, y, z)`. We cannot use uniform random distribution, as a desert needs sparse cacti while a forest needs dense oaks.

* **Adaptive Poisson Disk / Blue Noise:** Poisson Disk sampling mathematically guarantees a minimum distance `r` between points, resulting in an organic distribution without ugly clumps (which simple noise thresholding causes). To make this biome-specific, we sample the `WorldTerrain` biome weights at that specific `(x, z)` coordinate to define `r`. For example, in a Plains biome, the required distance `r` for tall grass is extremely small (yielding high density), but `r` for trees is very large (sparse). In a Forest biome, `r` for trees becomes small.
* **Layered Noise Maps:** We evaluate independent noise functions for different categories of flora. Layer A determines tree placement. Layer B determines grass. Layer C determines flowers. The output of these noise layers is multiplied by the specific biome weight for that flora type, ensuring flowers do not spawn in deserts even if the Layer C noise spikes at that coordinate.

## A.3 Geometry & Rendering Pipeline

* **Geometry:** Absolutely everything is a 3D cube. There are no 2D sprites or planes. A tree trunk is a column of solid cubes. A flower is a specific 3D configuration of mini-voxels or a full 1x1x1 block painted to look like a flower.
* **Rendering (The Chunk Mesher):** Decorators are integrated into the chunk's greedy meshing algorithm. However, because leaves have transparent pixels, they require a split render pipeline to prevent greedy meshing from improperly culling faces behind them. Reviewing `render_state.rs`, the engine already has a `transparent_pipeline` designed for water using `wgpu::BlendState::ALPHA_BLENDING` and depth testing.
    1. *Opaque Pass:* (Wood, Stone, Dirt). Appended to `opaque_index_buffer`. Greedy meshed, depth-tested.
    2. *Transparent Pass:* (Leaves, Glass). Appended to the existing `transparent_index_buffer`. Rendered back-to-front seamlessly alongside water in the second render pass, with alpha blending enabled.

## A.4 Procedural Texture Generation (No Artists)

Since we are using 3D cubes, we must procedurally generate textures for all 6 faces on engine startup, writing them to a dynamic texture atlas to solve the lack of artist assets.

* **Bark:** We map the UV coordinates to a vertical Perlin noise function. By stretching the noise heavily along the Y-axis, we simulate vertical bark grain. Oak maps the noise output to a palette of medium browns. Birch maps the base to white, and applies a harsh threshold to high-frequency noise to generate black horizontal stripes.
* **Rings:** For the top and bottom faces of wood blocks, we use mathematical sine waves based on the radial distance from the UV center: `color = sin(distance_from_center * frequency)`. We perturb the `distance_from_center` with a slight low-frequency noise to make the rings organic rather than perfect concentric circles.
* **Leaves:** We generate a base color and overlay Cellular/Voronoi noise. We apply a strict alpha threshold: pixels where the noise value is below `0.4` are set to `rgba(0,0,0,0)` (100% transparent), creating physical gaps in the leaf block. The base color parameter shifts dynamically based on biomes (dark green in dense forests) or global `Season` variables (shifting to yellow/orange during Autumn).

## A.5 Physics & Interaction: The Minecraft Standard (LOCKED)

* **Destructibility:** Because the tree is an array of blocks, the player can destroy a single 1x1x1 block in the center of the trunk, or carve a staircase into a massive tree. It is infinitely, locally destructible.
* **The Identity Problem (Metadata):** A wood block spawned by the decorator algorithm is structurally identical to a wood block placed by a player building a cabin. If we need the engine to differentiate them, we must expand `chunk.blocks` from a `u16` ID to a `u32` (16 bits ID, 16 bits Metadata), including an `is_natural: bool` flag set to `true` during generation and `false` when placed by a player.
* **Floating Blocks (No Gravity):** Standard blocks (Wood, Leaves) have zero gravity physics. If a player destroys the bottom block of a tree, the rest of the tree remains floating in mid-air exactly where it was generated. This is computationally free and highly consistent.
* **Leaf Decay:** To prevent forever-floating leaves when a tree is chopped down, we implement a random tick system. Every tick, a small percentage of leaf blocks are evaluated. The leaf performs a Breadth-First Search (up to a radius of 4 blocks) looking for a `Wood` block. If no `Wood` is found, the leaf block destroys itself.

---

# PATH B (LOCKED): The Entity / Mesh Architecture

In this path, decorators are independent 3D objects (meshes) managed by a Scene Graph or Entity Component System. They sit *on top of* or *inside* the voxel terrain.

## B.1 Placement & Cave Spawning

There is no "Chunk Boundary Problem" here because entities do not inject data into chunk arrays.

* **Global Spawning:** When the player enters a region, an Entity Spawner uses spatial hashing (e.g., `hash(grid_x, grid_z, world_seed)`) to deterministically seed the RNG for that region. This guarantees a Tree Entity always spawns at the exact same continuous world coordinate (e.g., `x: 14.52, z: -100.89`) every time you revisit the area. The entity's mesh simply exists in 3D space, managed by an Octree for frustum culling.
* **3D Biomes & Cave Spawning:** If we want underground mushrooms or crystals, we must adapt the placement algorithm to 3D. The Spawner evaluates the 3D voxel density noise (or chunk data) alongside a 3D Biome Map. If a 3D coordinate evaluates to an "Underground" biome, the Spawner hunts for transitions from Solid to Air. When a transition is found, we calculate the surface normal. A mushroom entity is spawned and rotated to align with the floor normal, while a stalactite aligns to a ceiling normal.
* **Clipping Prevention:** To prevent a large cave tree from clipping through tight cavern walls, the Spawner performs a bounding-box volume check against the 3D voxel grid before spawning. If the mesh volume intersects solid rock, the spawn is aborted or scaled down.

## B.2 Distribution Logic: Continuous Float Coordinates (LOCKED)

*This option has been locked in per your feedback. We will NOT snap entities to the integer grid in Path B.*

Density is driven by biome weights identically to Path A, but the output coordinate applies to an Entity Transform using continuous float space. Outputting continuous floats allows for sub-voxel micro-translations and arbitrary rotation (e.g., leaning trees, randomized yaw). This looks highly organic, though it creates a visual distinction between the fluid placement of nature and the rigid, integer-locked placement of player voxel buildings.

## B.3 Geometry & Rendering

* **Geometry Generation (L-Systems):** Excellent for generating line segments that are easily lofted into cylinders/polygons. These output standard vertex buffers natively compatible with our existing `wgpu` rasterization pipeline.
* **Rendering:** Trees are rendered via Instanced Rendering or separate draw calls outside the chunk mesher.

## B.4 Procedural Asset Pipeline

* **Procedural Meshes:** We do not generate 2D cube textures. The startup routine generates 3D vertex data (positions, normals, UVs) algorithmically based on L-system rules, allowing for smooth, non-grid-aligned branches.
* **Procedural Texturing:** Leveraging Inigo Quilez's methodologies, we generate noise textures algorithmically on the CPU/GPU. These are applied to the generated meshes using Triplanar Mapping (projecting 2D noise from the X, Y, and Z axes), which eliminates the need to calculate complex UV unwrapping on procedurally generated branch geometry.
* **Vertex Coloring:** We can supplement textures with procedural vertex coloring (brown for trunk vertices, green for foliage vertices), shifting the leaf vertex colors dynamically based on the global `Season` uniform passed into the shader.

## B.5 Physics & Interaction (Raycasting explained)

Harvesting yields inventory items (e.g., "Wood Block" items). The player can place these blocks to build a cabin, but the player's cabin will look blocky (Path A-style voxels), while the natural trees look smooth (Path B meshes).

* **Destructibility:** Upon harvesting, the *entire entity* is destroyed instantly. You cannot carve a hole through the leaves or chop "half" of the trunk because it is a single unified mesh.
* **Interaction (Raycasting):** The CPU performs standard bounding-box or AABB (Axis-Aligned Bounding Box) intersection tests against the player's look-vector to determine if they hit a procedural tree mesh.
* **Physics:** Entities natively support rigid body physics in game engines. When chopped, the Tree Entity can become a dynamic rigid body that tips over and falls to the ground smoothly before despawning and converting into inventory item drops, without any of the immense CPU load required to calculate falling physics for 100 individual voxel blocks.

---

# Phased Implementation Plan: Path B Decorators

This plan outlines the rigorous architectural roadmap for integrating **Path B (Entity/Mesh Architecture)** decorators into `ruxel`.

## Proposed Changes

### Phase 1: Spatial Management & Poisson Disk Spawning [COMPLETE]

Entities must be spawned deterministically based on continuous coordinates and managed via a spatial grid.

* **[NEW] `src/poisson.rs`:** Implement an Adaptive Poisson Disk Sampling algorithm. The algorithm evaluates the `WorldTerrain` biome weight at `(x, z)` to dynamically determine the minimum distance radius `r` between generated points.
* **[NEW] `src/entities.rs`:** Create an `EntityManager`. It divides the world into spatial cells (matching the 16x16 chunk grid). When a cell loads, it deterministically seeds the RNG (e.g., `hash(world_seed, cell_x, cell_z)`) and runs the Poisson sampler to generate tree origins.

### Phase 2: Biome-Specific Distribution & Taxonomy

The generation must strictly adhere to the biome environment and placement rules.

* **[MODIFY] `src/terrain.rs`:** Update `DesertTerrain` to occasionally generate deep depressions that dip below the global water level to naturally form rare Oases.
* **[PROCESS]**: develop one terrain at a time, testing between each one.
* **[MODIFY] `src/lsystem.rs`:** Expand the L-system rules into a taxonomy:
  * **Oak/Birch:** Standard branching, spawns primarily in **Hills**.
  * **Pine/Conifer:** Tall, triangular branching, spawns primarily in **Mountains**.
  * **Palm Trees:** Thin trunks, top-heavy leaves. Spawns in **Deserts**, but *only* if the terrain height is at or near the water level threshold (`y <= WATER_LEVEL + 3`), populating the rare Oases.
  * **Scrub/Bush:** Low iteration, dense leaves. Spawns heavily in **Plains**.
  * **Cacti:** L-system without leaves. Spawns in **Deserts**, extremely rarely (large Poisson radius `r`).
* **[MODIFY] `src/entities.rs`:** The `EntityManager` queries the terrain height map to snap the `y` coordinate. It then queries the biome type at `(x, z)` to decide *which* L-system rule to evaluate and generate.

### Phase 3: Batched Entity Rendering (Opaque & Transparent)

Generating individual `wgpu::Buffer`s for every tree is inefficient. We will use Batched Meshes to preserve unique procedural shapes.

* **Buffer Separation:** The entity mesh generation will split into two buffers per chunk: `opaque_buffer` (trunks, branches, cacti) and `transparent_buffer` (leaves).
* **Boundary Overlap Solution:** An entity belongs exclusively to the spatial cell where its origin `(x, z)` resides. When a cell is loaded, all entity vertices whose origins are in that cell are concatenated into the batched buffers. If a tree's branches physically overhang into an unloaded neighboring chunk, they will disappear when the origin chunk unloads. Because chunks unload outside the player's fog distance, this disappearance will be completely hidden by the fog, requiring no expanded load radius!
* **[MODIFY] `src/render_state.rs`:** Add an `EntityChunkBuffers` map. During the opaque render pass, iterate and draw the trunks. During the transparent render pass (where water is drawn), iterate and draw the leaves to ensure proper alpha blending.

### Phase 4: Raycasting Interaction

* **[MODIFY] `src/camera.rs` / `src/scene.rs`:** Implement an AABB (Axis-Aligned Bounding Box) intersection test for the camera's look vector. Each entity will store a simplified AABB for its trunk/mesh.
* **[NEW] Destruction Logic:** When the player clicks, the raycast detects the entity. The entire entity is removed from the `EntityManager` and the batched mesh buffer is re-generated for that cell. *(Future iterations will handle the physics of the tree tipping over and falling).*

## Verification Plan

### Automated Tests

* **Poisson Disk Tests:** Add unit tests in `src/poisson.rs` to assert that generated coordinates never violate the minimum distance `r` for given biomes.
* **Deterministic Spawning:** Add tests asserting that initializing the `EntityManager` for cell `(10, -5)` twice with the same seed yields the exact same entity coordinates and tree types.
* **Performance Benchmark:** Add a `cargo bench` load test measuring the time it takes to generate and batch entity meshes for a dense forest region of 10x10 chunks, ensuring generation times do not block the main thread.

### Manual Verification

1. **Placement Validation:** Use `/find_biome desert` to teleport to a desert. Visually confirm palm trees are exclusively near water (oases) and cacti are rare. Use `/find_biome mountains` to confirm pine-like structures.
2. **Destruction:** Look at a tree trunk and left-click. Confirm the tree vanishes.
3. **Rendering Integrity:** Look at overlapping leaves against the sky to ensure the transparent rendering pass handles the alpha cutouts correctly.
