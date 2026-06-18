# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone.
it's written in rust.

## Bugs

## Future improvements

1. **Push Sun and Moon Further Out**: Since expanding the chunk load radius,
    the sun and moon can be seen setting into the sea/world because their orbit
    radius is too small. We need to push them further away.
2. **Save/Load World to Disk**: Implement chunk serialization so the
   procedurally generated terrain and any user modifications persist between
   sessions.
3. **Trees & Flora (Decorators)**: Add a decorator pass during chunk generation
   to place structures like trees or tall grass.
4. **Dynamic World Simulation**: Implement block-update mechanics where the
   world evolves over time (e.g., trees grow, ice melts, water flows).
5. **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the
   surface.
6. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that
   navigate the voxel terrain and interact with the world.
7. **More terrain types**: We currently only have mountains (with water). It
   would be nice to have some plains, some rolling hills, etc.
8. **More Block Types**: Expand the block palette with new materials and
    properties to allow for richer building and terrain variation.
9. **Player Object Rendering**: Render a 3D model/mesh for the player character
    instead of just relying on the camera's perspective, allowing third-person
   views and visible avatars.
10. **Shadow Mapping**: Implement directional shadows cast by the sun and moon
    across the voxel terrain to add depth and realism to the lighting.
11. **Expand Configuration File**: We now have a `config.toml` that handles
    `chunk_load_radius` and `seed`. In the future, we should expand this to
    include:
    - Display settings: window resolution, fullscreen mode, VSync.
    - Gameplay settings: FOV (field of view), mouse sensitivity, keybindings.
    - Graphics settings: shadow quality, anti-aliasing.

## Tech Debt
