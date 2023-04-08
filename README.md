# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone. 
it's written in rust.

## TODO
* player object instead of moving the camera
* collision
* better landscape generation
    * perlin is fine for now but maybe something more interesting if the complexplanet example can be made to work
* chunks: basics are in place but i need to:
    * queue up the loading of chunks based on player position
    * keep track of which chunks are ready to render
    * update instance buffer based on these renderable chunks
    * _optimisation_: make a chunk an instance rather than each block to reduce render calls
* save/load world to disk
* more block types
* specular lighting
* shadow mapping
* transparent blocks
    * basics are there, but all the water should be a separate render pass to work between chunks.
* textures?
* creatures
* dynamic world?
