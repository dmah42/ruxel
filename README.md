# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone.
it's written in rust.

## Bugs

- we don't save the player position/direction.
- underwater fog is glowing almost white at night.. i think maybe a sky colour
  issue?

## Future improvements

1. **Switch Local Biome Noise to Simplex**: Currently, individual biomes like
   hills and mountains use `Perlin` noise for heightmaps. Switching these to
   `Simplex` noise would remove grid-like directional artifacts and result in
   more organic local terrain.
2. **Trees & Flora (Decorators)**: Add a decorator pass during chunk generation
   to place structures like trees or tall grass.
3. **Dynamic World Simulation**: Implement block-update mechanics where the
   world evolves over time (e.g., trees grow, ice melts, water flows). CLOUDS!
4. **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the
   surface.
5. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that
   navigate the voxel terrain and interact with the world.
6. **More terrain types**: We currently only have mountains (with water). It
   would be nice to have some plains, some rolling hills, etc.
7. **More Block Types**: Expand the block palette with new materials and
   properties to allow for richer building and terrain variation.
8. **Better underwater fog**: Model the colour shift that happens as a player
   gets deeper into water.
9. **Player Object Rendering**: Render a 3D model/mesh for the player character
   instead of just relying on the camera's perspective, allowing third-person
   views and visible avatars.
10. **Shadow Mapping**: Implement directional shadows cast by the sun and moon
   across the voxel terrain to add depth and realism to the lighting.
11. **Expand Configuration File**: We now have a `config.toml` that handles
      `chunk_load_radius` and `seed`. In the future, we should expand this to
      include:
      - Display settings: window resolution, fullscreen mode, VSync.
      - Gameplay settings: mouse sensitivity, keybindings.
      - Graphics settings: shadow quality, anti-aliasing.

## Tech Debt

1. **Decouple UI from RenderState**: Currently, `RenderState` owns the `Ui`
   struct and calculates game logic (like determining the player's current
   biome) during its update loop. To adhere to a cleaner architecture, ownership
   of the `Ui` should be lifted out into the main `Engine` struct in `lib.rs`.
   The game loop should calculate state and update the UI, then simply pass the
   `Ui` to `RenderState` to act as a "dumb" renderer.

2. **Add more tests**: We have tests. we should have more tests.
