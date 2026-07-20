# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone.
it's written in rust.

## Bugs

## Future improvements

1. **Dynamic World Simulation**: Implement block-update mechanics where the
   world evolves over time (e.g., trees grow, ice melts). CLOUDS!
1. **Inventory system**: A player should be able to collect stuff by destroying
   blocks.
1. **Interactions with entities**: Clicking on a tree should destroy the tree
   (with physics?!) and provide the player with "wood". This requires an
   inventory system.
1. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that
   navigate the voxel terrain and interact with the world.
1. **More Block Types**: Expand the block palette with new materials and
   properties to allow for richer building and terrain variation.
1. **Player Object Rendering**: Render a 3D model/mesh for the player character
   instead of just relying on the camera's perspective, allowing third-person
   views and visible avatars.
1. **Expand Configuration File**: We now have a `config.toml` that handles
     `chunk_load_radius` and `seed`. In the future, we should expand this to
     include:
     - Display settings: fullscreen mode, VSync.
     - Gameplay settings: mouse sensitivity, keybindings.
     - Graphics settings: shadow quality, anti-aliasing.
1. **Climate**: Have the temperature/moisture maps get feedback from the
   generated terrain (e.g. rain shadows, altitude cooling). T+M also create
   weather.

## API Boundary Type Policies

To keep type casting to a minimum, we stick to the following coordinate and API
type policies:

1. **Continuous World Space**: Use `glam::Vec3` (`f32`) for all continuous
   coordinates (player position, velocity, camera direction, light directions).
2. **Discrete Block Space**: Use `glam::IVec3` (`i32`) for all discrete grid
   positions (block locations, raycasting hit/normal vectors). This avoids
   underflow bugs when using offsets (e.g., neighbor offset calculations).
3. **Chunk Column Coordinates**: Use `glam::UVec2` (`u32`) for indexing chunk
   columns in the grid.
4. **Voxel Grid Division**: Use `x.div_euclid(16)` to calculate chunk indices,
   and `x.rem_euclid(16) as usize` for relative local coordinates inside a chunk
   column. Range validation checks on signed values are performed *before*
   casting to unsigned to prevent underflow.
5. **Terrain boundaries**: Encapsulate `f64` noise calculation entirely inside
   `WorldTerrain`. All public functions accept `glam::Vec2` (`f32`) world
   coordinates, and `TerrainData` stores its fields (`height`, `moisture`,
   `temperature`) as `f32`.

## Tech Debt
