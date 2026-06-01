use crate::chunks::Chunk;
use crate::vertex::Vertex;
use glam::Vec3;
use noise::{Fbm, NoiseFn, Perlin};

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

    pub fn build(chunk: &Chunk, terrain: &Fbm<Perlin>) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let blocks = chunk.blocks();
        let start = chunk.start();

        let is_solid = |wx: i32, wy: i32, wz: i32| -> bool {
            let cx = wx - start.x as i32;
            let cy = wy - start.y as i32;
            let cz = wz - start.z as i32;

            if (0..16).contains(&cx) && (0..16).contains(&cy) && (0..16).contains(&cz) {
                blocks[cx as usize][cy as usize][cz as usize].is_active()
            } else {
                if wy < 32 {
                    true
                } else {
                    let point: [f64; 2] = [wx as f64 / 384.0, wz as f64 / 384.0];
                    let height = ((terrain.get(point) + 1.0) * 32.0) as i32;
                    wy < height
                }
            }
        };

        let vertex_ao = |side1: bool, side2: bool, corner: bool| -> f32 {
            let num_solid = if side1 && side2 {
                3
            } else {
                side1 as u8 + side2 as u8 + corner as u8
            };
            match num_solid {
                0 => 1.0,
                1 => 0.85,
                2 => 0.7,
                _ => 0.5,
            }
        };

        for (x, slice_x) in blocks.iter().enumerate().take(16) {
            for (y, slice_y) in slice_x.iter().enumerate().take(16) {
                for (z, block) in slice_y.iter().enumerate().take(16) {
                    if !block.is_active() {
                        continue;
                    }

                    let color = block.color();
                    let color_arr = [
                        color.r as f32,
                        color.g as f32,
                        color.b as f32,
                        color.a as f32,
                    ];
                    let pos = start + Vec3::new(x as f32, y as f32, z as f32);

                    let mut add_face = |normal: [f32; 3], vts: &[[f32; 3]; 4], aos: [f32; 4]| {
                        let idx = vertices.len() as u32;
                        for (i, v) in vts.iter().enumerate() {
                            vertices.push(Vertex::new(
                                [pos.x + v[0], pos.y + v[1], pos.z + v[2]],
                                normal,
                                color_arr,
                                aos[i],
                            ));
                        }

                        // Flip quad depending on AO to prevent anisotropy
                        if aos[0] + aos[2] > aos[1] + aos[3] {
                            indices.extend_from_slice(&[
                                idx,
                                idx + 1,
                                idx + 2,
                                idx + 2,
                                idx + 3,
                                idx,
                            ]);
                        } else {
                            indices.extend_from_slice(&[
                                idx + 1,
                                idx + 2,
                                idx + 3,
                                idx + 3,
                                idx,
                                idx + 1,
                            ]);
                        }
                    };

                    let wx = start.x as i32 + x as i32;
                    let wy = start.y as i32 + y as i32;
                    let wz = start.z as i32 + z as i32;

                    // X+ (Right)
                    if !is_solid(wx + 1, wy, wz) {
                        let a00 = is_solid(wx + 1, wy - 1, wz - 1);
                        let a01 = is_solid(wx + 1, wy - 1, wz);
                        let a02 = is_solid(wx + 1, wy - 1, wz + 1);
                        let a10 = is_solid(wx + 1, wy, wz - 1);
                        let a12 = is_solid(wx + 1, wy, wz + 1);
                        let a20 = is_solid(wx + 1, wy + 1, wz - 1);
                        let a21 = is_solid(wx + 1, wy + 1, wz);
                        let a22 = is_solid(wx + 1, wy + 1, wz + 1);

                        let ao0 = vertex_ao(a10, a01, a00); // [1, 0, 0]
                        let ao1 = vertex_ao(a21, a10, a20); // [1, 1, 0]
                        let ao2 = vertex_ao(a12, a21, a22); // [1, 1, 1]
                        let ao3 = vertex_ao(a01, a12, a02); // [1, 0, 1]

                        add_face(
                            [1.0, 0.0, 0.0],
                            &[
                                [1.0, 0.0, 0.0],
                                [1.0, 1.0, 0.0],
                                [1.0, 1.0, 1.0],
                                [1.0, 0.0, 1.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                    // X- (Left)
                    if !is_solid(wx - 1, wy, wz) {
                        let a00 = is_solid(wx - 1, wy - 1, wz - 1);
                        let a01 = is_solid(wx - 1, wy - 1, wz);
                        let a02 = is_solid(wx - 1, wy - 1, wz + 1);
                        let a10 = is_solid(wx - 1, wy, wz - 1);
                        let a12 = is_solid(wx - 1, wy, wz + 1);
                        let a20 = is_solid(wx - 1, wy + 1, wz - 1);
                        let a21 = is_solid(wx - 1, wy + 1, wz);
                        let a22 = is_solid(wx - 1, wy + 1, wz + 1);

                        let ao0 = vertex_ao(a01, a12, a02); // [0, 0, 1]
                        let ao1 = vertex_ao(a12, a21, a22); // [0, 1, 1]
                        let ao2 = vertex_ao(a21, a10, a20); // [0, 1, 0]
                        let ao3 = vertex_ao(a10, a01, a00); // [0, 0, 0]

                        add_face(
                            [-1.0, 0.0, 0.0],
                            &[
                                [0.0, 0.0, 1.0],
                                [0.0, 1.0, 1.0],
                                [0.0, 1.0, 0.0],
                                [0.0, 0.0, 0.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                    // Y+ (Top)
                    if !is_solid(wx, wy + 1, wz) {
                        let a00 = is_solid(wx - 1, wy + 1, wz - 1);
                        let a01 = is_solid(wx - 1, wy + 1, wz);
                        let a02 = is_solid(wx - 1, wy + 1, wz + 1);
                        let a10 = is_solid(wx, wy + 1, wz - 1);
                        let a12 = is_solid(wx, wy + 1, wz + 1);
                        let a20 = is_solid(wx + 1, wy + 1, wz - 1);
                        let a21 = is_solid(wx + 1, wy + 1, wz);
                        let a22 = is_solid(wx + 1, wy + 1, wz + 1);

                        let ao0 = vertex_ao(a12, a01, a02); // [0, 1, 1]
                        let ao1 = vertex_ao(a21, a12, a22); // [1, 1, 1]
                        let ao2 = vertex_ao(a10, a21, a20); // [1, 1, 0]
                        let ao3 = vertex_ao(a01, a10, a00); // [0, 1, 0]

                        add_face(
                            [0.0, 1.0, 0.0],
                            &[
                                [0.0, 1.0, 1.0],
                                [1.0, 1.0, 1.0],
                                [1.0, 1.0, 0.0],
                                [0.0, 1.0, 0.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                    // Y- (Bottom)
                    if !is_solid(wx, wy - 1, wz) {
                        let a00 = is_solid(wx - 1, wy - 1, wz - 1);
                        let a01 = is_solid(wx - 1, wy - 1, wz);
                        let a02 = is_solid(wx - 1, wy - 1, wz + 1);
                        let a10 = is_solid(wx, wy - 1, wz - 1);
                        let a12 = is_solid(wx, wy - 1, wz + 1);
                        let a20 = is_solid(wx + 1, wy - 1, wz - 1);
                        let a21 = is_solid(wx + 1, wy - 1, wz);
                        let a22 = is_solid(wx + 1, wy - 1, wz + 1);

                        let ao0 = vertex_ao(a01, a10, a00); // [0, 0, 0]
                        let ao1 = vertex_ao(a10, a21, a20); // [1, 0, 0]
                        let ao2 = vertex_ao(a21, a12, a22); // [1, 0, 1]
                        let ao3 = vertex_ao(a12, a01, a02); // [0, 0, 1]

                        add_face(
                            [0.0, -1.0, 0.0],
                            &[
                                [0.0, 0.0, 0.0],
                                [1.0, 0.0, 0.0],
                                [1.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                    // Z+ (Front)
                    if !is_solid(wx, wy, wz + 1) {
                        let a00 = is_solid(wx - 1, wy - 1, wz + 1);
                        let a01 = is_solid(wx - 1, wy, wz + 1);
                        let a02 = is_solid(wx - 1, wy + 1, wz + 1);
                        let a10 = is_solid(wx, wy - 1, wz + 1);
                        let a12 = is_solid(wx, wy + 1, wz + 1);
                        let a20 = is_solid(wx + 1, wy - 1, wz + 1);
                        let a21 = is_solid(wx + 1, wy, wz + 1);
                        let a22 = is_solid(wx + 1, wy + 1, wz + 1);

                        let ao0 = vertex_ao(a01, a10, a00); // [0, 0, 1]
                        let ao1 = vertex_ao(a10, a21, a20); // [1, 0, 1]
                        let ao2 = vertex_ao(a21, a12, a22); // [1, 1, 1]
                        let ao3 = vertex_ao(a12, a01, a02); // [0, 1, 1]

                        add_face(
                            [0.0, 0.0, 1.0],
                            &[
                                [0.0, 0.0, 1.0],
                                [1.0, 0.0, 1.0],
                                [1.0, 1.0, 1.0],
                                [0.0, 1.0, 1.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                    // Z- (Back)
                    if !is_solid(wx, wy, wz - 1) {
                        let a00 = is_solid(wx - 1, wy - 1, wz - 1);
                        let a01 = is_solid(wx - 1, wy, wz - 1);
                        let a02 = is_solid(wx - 1, wy + 1, wz - 1);
                        let a10 = is_solid(wx, wy - 1, wz - 1);
                        let a12 = is_solid(wx, wy + 1, wz - 1);
                        let a20 = is_solid(wx + 1, wy - 1, wz - 1);
                        let a21 = is_solid(wx + 1, wy, wz - 1);
                        let a22 = is_solid(wx + 1, wy + 1, wz - 1);

                        let ao0 = vertex_ao(a10, a21, a20); // [1, 0, 0]
                        let ao1 = vertex_ao(a01, a10, a00); // [0, 0, 0]
                        let ao2 = vertex_ao(a12, a01, a02); // [0, 1, 0]
                        let ao3 = vertex_ao(a21, a12, a22); // [1, 1, 0]

                        add_face(
                            [0.0, 0.0, -1.0],
                            &[
                                [1.0, 0.0, 0.0],
                                [0.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0],
                                [1.0, 1.0, 0.0],
                            ],
                            [ao0, ao1, ao2, ao3],
                        );
                    }
                }
            }
        }

        Self { vertices, indices }
    }
}
