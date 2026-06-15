# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone. 
it's written in rust.

## TODO
1. **Specular Lighting**: Add glossy highlights to the shader, specifically targeting certain block types like water or ice to make them visually distinct and shiny.
2. **Dynamic World Simulation**: Implement block-update mechanics where the world evolves over time (e.g., trees grow, ice melts, water flows).
3. **Save/Load World to Disk**: Implement chunk serialization so the procedurally generated terrain and any user modifications persist between sessions.
4. **Trees & Flora (Decorators)**: Add a decorator pass during chunk generation to place structures like trees or tall grass.
5. **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the surface.
6. **Creatures/Entities**: Add mobile, AI-driven entities (mobs/animals) that navigate the voxel terrain and interact with the world.
7. **More Block Types**: Expand the block palette with new materials and properties to allow for richer building and terrain variation.
8. **Player Object Rendering**: Render a 3D model/mesh for the player character instead of just relying on the camera's perspective, allowing third-person views and visible avatars.
9. **Shadow Mapping**: Implement directional shadows cast by the sun and moon across the voxel terrain to add depth and realism to the lighting.

## Tech Debt
- `RenderState` currently acts as a god-object that owns the `Scene` and `Camera`, meaning game logic (like block placement in the `interact` method) lives inside the renderer. This should be refactored so the main `Ruxel` application (or a dedicated game state struct) owns the world state and handles input/interaction, passing references down to the renderer.
