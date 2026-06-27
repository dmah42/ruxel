# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone.
it's written in rust.

## Bugs

- oaks are a little on the small side

## Future improvements

1. **Dynamic World Simulation**: Implement block-update mechanics where the
   world evolves over time (e.g., trees grow, ice melts, water flows). CLOUDS!
1. **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the
   surface.
1. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that
   navigate the voxel terrain and interact with the world.
1. **More Block Types**: Expand the block palette with new materials and
   properties to allow for richer building and terrain variation.
1. **Better underwater fog**: Model the colour shift that happens as a player
   gets deeper into water.
1. **Player Object Rendering**: Render a 3D model/mesh for the player character
   instead of just relying on the camera's perspective, allowing third-person
   views and visible avatars.
1. **Shadow Mapping**: Implement directional shadows cast by the sun and moon
   across the voxel terrain to add depth and realism to the lighting.
1. **Expand Configuration File**: We now have a `config.toml` that handles
     `chunk_load_radius` and `seed`. In the future, we should expand this to
     include:
     - Display settings: window resolution, fullscreen mode, VSync.
     - Gameplay settings: mouse sensitivity, keybindings.
     - Graphics settings: shadow quality, anti-aliasing.
1. **Climate**: Have the temperature/moisture maps get feedback from the generated terrain (e.g. rain shadows, altitude cooling).

## Tech Debt
