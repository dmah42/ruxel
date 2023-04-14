# ruxel

ruxel is a voxel rendering engine but will probably end up as a minecraft clone. 
it's written in rust.

## TODO
* player object instead of moving the camera
    * hacked to make it look like a player.
* collision
    * basics are in place but the player can climb too easily maybe
* better landscape generation
    * Fbm<Perlin> is working well and is fast.
* chunks: basics are in place but i need to:
    * _optimisation_: make a chunk an instance rather than each block to reduce render calls
* save/load world to disk
* more block types
* specular lighting
    * maybe just for some block types
* shadow mapping
* transparent blocks
    * basics are there, but all the water should be a separate render pass to work between chunks.
* textures?
* creatures
* dynamic world?
