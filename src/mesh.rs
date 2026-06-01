use crate::chunks::Chunk;
use crate::vertex::Vertex;
use glam::Vec3;

#[derive(Debug)]
pub struct ChunkMesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn build(chunk: &Chunk) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let blocks = chunk.blocks();
        let start = chunk.start();

        // For Phase 1, we do simple non-greedy meshing, but without instances!
        // We just iterate blocks and add their exposed faces. This is a stepping stone to greedy meshing,
        // and guarantees we can get the pipeline working first.
        // Actually, let's just do a basic mesher first that only generates faces if the adjacent block is inactive.
        
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let block = &blocks[x][y][z];
                    if !block.is_active() {
                        continue;
                    }

                    let color = block.color();
                    let color_arr = [color.r as f32, color.g as f32, color.b as f32, color.a as f32];
                    let pos = start + Vec3::new(x as f32, y as f32, z as f32);
                    
                    // Simple face generation helper
                    let mut add_face = |normal: [f32; 3], vts: &[[f32; 3]; 4]| {
                        let idx = vertices.len() as u32;
                        for v in vts {
                            vertices.push(Vertex::new(
                                [pos.x + v[0], pos.y + v[1], pos.z + v[2]],
                                normal,
                                color_arr,
                                0.0, // AO 0.0 for Phase 1
                            ));
                        }
                        // two triangles for the quad
                        indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 3, idx]);
                    };

                    // Check neighbors (this assumes boundary blocks are exposed, which is fine for Phase 1)
                    // X+ (Right)
                    if x == 15 || !blocks[x + 1][y][z].is_active() {
                        add_face([1.0, 0.0, 0.0], &[
                            [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]
                        ]);
                    }
                    // X- (Left)
                    if x == 0 || !blocks[x - 1][y][z].is_active() {
                        add_face([-1.0, 0.0, 0.0], &[
                            [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]
                        ]);
                    }
                    // Y+ (Top)
                    if y == 15 || !blocks[x][y + 1][z].is_active() {
                        add_face([0.0, 1.0, 0.0], &[
                            [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]
                        ]);
                    }
                    // Y- (Bottom)
                    if y == 0 || !blocks[x][y - 1][z].is_active() {
                        add_face([0.0, -1.0, 0.0], &[
                            [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]
                        ]);
                    }
                    // Z+ (Front/Back)
                    if z == 15 || !blocks[x][y][z + 1].is_active() {
                        add_face([0.0, 0.0, 1.0], &[
                            [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]
                        ]);
                    }
                    // Z- (Front/Back)
                    if z == 0 || !blocks[x][y][z - 1].is_active() {
                        add_face([0.0, 0.0, -1.0], &[
                            [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]
                        ]);
                    }
                }
            }
        }

        Self { vertices, indices }
    }
}
