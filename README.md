# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone.
it's written in rust.

## Bugs

## Future improvements

1. **Dynamic chunk range**: change the `CHUNK_LOAD_RADIUS` from a const to a
   value that can be changed based on computer performance capabilities. note
   the shader will also need to be updated somehow as the fog radius relies on
   the `CHUNK_LOAD_RADIUS` implicitly.
2. **Dynamic World Simulation**: Implement block-update mechanics where the
   world evolves over time (e.g., trees grow, ice melts, water flows).
3. **Simulate a better sky**: <https://nicoschertler.wordpress.com/2013/04/03/simulating-a-days-sky/>
4. **Save/Load World to Disk**: Implement chunk serialization so the
   procedurally generated terrain and any user modifications persist between
   sessions.
5. **More terrain types**: We currently only have mountains (with water). It
   would be nice to have some plains, some rolling hills, etc.
6. **Trees & Flora (Decorators)**: Add a decorator pass during chunk generation
   to place structures like trees or tall grass.
7. **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the
   surface.
8. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that
   navigate the voxel terrain and interact with the world.
9. **More Block Types**: Expand the block palette with new materials and
   properties to allow for richer building and terrain variation.
10. **Player Object Rendering**: Render a 3D model/mesh for the player character
   instead of just relying on the camera's perspective, allowing third-person
   views and visible avatars.
11. **Shadow Mapping**: Implement directional shadows cast by the sun and moon
    across the voxel terrain to add depth and realism to the lighting.

## Tech Debt

- `RenderState` currently acts as a god-object that owns the `Scene` and
  `Camera`, meaning game logic (like block placement in the `interact` method)
  lives inside the renderer. This should be refactored so the main `Ruxel`
  application (or a dedicated game state struct) owns the world state and
  handles input/interaction, passing references down to the renderer.
