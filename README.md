# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone. 
it's written in rust.

## TODO
* player object instead of moving the camera
    * hacked to make it look like a player.
* save/load world to disk
* more block types
* specular lighting
    * maybe just for some block types
* shadow mapping
* creatures
* dynamic world?

## Future Work

* **Frustum Culling & View Distance**: Check if a chunk's bounding box is within
  the camera's view before drawing it to increase render distance without
  sacrificing performance.
* **Targeted Block Highlight**: Add visual feedback (like a wireframe cube) to
  the targeted block from the DDA raycaster.
* **Trees & Flora (Decorators)**: Add a decorator pass during chunk generation
  to place structures like trees or tall grass.
* **3D Noise / Caves**: Introduce 3D noise to carve out cave networks below the
  surface.
