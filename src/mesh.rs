use crate::vertex::Vertex;
use crate::{chunks::Chunk, terrain::MountainTerrain};
use glam::Vec3;
use noise::NoiseFn;

#[derive(Debug)]
pub struct ChunkMesh {
    vertices: Vec<Vertex>,
    opaque_indices: Vec<u32>,
    transparent_indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn opaque_indices(&self) -> &[u32] {
        &self.opaque_indices
    }

    pub fn transparent_indices(&self) -> &[u32] {
        &self.transparent_indices
    }

    pub fn build(chunk: &Chunk, terrain: &MountainTerrain) -> Self {
        let mut vertices = Vec::new();
        let mut opaque_indices = Vec::new();
        let mut transparent_indices = Vec::new();

        let blocks = chunk.blocks();
        let start = chunk.start();

        let is_opaque = |wx: i32, wy: i32, wz: i32| -> bool {
            let cx = wx - start.x as i32;
            let cy = wy - start.y as i32;
            let cz = wz - start.z as i32;

            if (0..16).contains(&cx) && (0..16).contains(&cy) && (0..16).contains(&cz) {
                let n = &blocks[cx as usize][cy as usize][cz as usize];
                n.is_active() && n.color().a == 1.0
            } else {
                let point: [f64; 2] = [wx as f64 / 384.0, wz as f64 / 384.0];
                let height = ((terrain.get(point) + 1.0) * 32.0) as i32;
                wy < height
            }
        };

        let is_transparent_block = |wx: i32, wy: i32, wz: i32| -> bool {
            let cx = wx - start.x as i32;
            let cy = wy - start.y as i32;
            let cz = wz - start.z as i32;

            if (0..16).contains(&cx) && (0..16).contains(&cy) && (0..16).contains(&cz) {
                let n = &blocks[cx as usize][cy as usize][cz as usize];
                n.is_active() && n.color().a < 1.0
            } else {
                let point: [f64; 2] = [wx as f64 / 384.0, wz as f64 / 384.0];
                let height = ((terrain.get(point) + 1.0) * 32.0) as i32;
                wy < 32 && wy >= height
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
                    let is_transparent = color.a < 1.0;
                    let pos = start + Vec3::new(x as f32, y as f32, z as f32);

                    let should_draw_face = |nx: i32, ny: i32, nz: i32| -> bool {
                        let neighbor_opaque = is_opaque(nx, ny, nz);
                        if is_transparent {
                            !neighbor_opaque && !is_transparent_block(nx, ny, nz)
                        } else {
                            !neighbor_opaque
                        }
                    };

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

                        let target_indices = if color_arr[3] < 1.0 {
                            &mut transparent_indices
                        } else {
                            &mut opaque_indices
                        };

                        // Flip quad depending on AO to prevent anisotropy
                        if aos[0] + aos[2] > aos[1] + aos[3] {
                            target_indices.extend_from_slice(&[
                                idx,
                                idx + 1,
                                idx + 2,
                                idx + 2,
                                idx + 3,
                                idx,
                            ]);
                        } else {
                            target_indices.extend_from_slice(&[
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
                    if should_draw_face(wx + 1, wy, wz) {
                        let a00 = is_opaque(wx + 1, wy - 1, wz - 1);
                        let a01 = is_opaque(wx + 1, wy - 1, wz);
                        let a02 = is_opaque(wx + 1, wy - 1, wz + 1);
                        let a10 = is_opaque(wx + 1, wy, wz - 1);
                        let a12 = is_opaque(wx + 1, wy, wz + 1);
                        let a20 = is_opaque(wx + 1, wy + 1, wz - 1);
                        let a21 = is_opaque(wx + 1, wy + 1, wz);
                        let a22 = is_opaque(wx + 1, wy + 1, wz + 1);

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
                    if should_draw_face(wx - 1, wy, wz) {
                        let a00 = is_opaque(wx - 1, wy - 1, wz - 1);
                        let a01 = is_opaque(wx - 1, wy - 1, wz);
                        let a02 = is_opaque(wx - 1, wy - 1, wz + 1);
                        let a10 = is_opaque(wx - 1, wy, wz - 1);
                        let a12 = is_opaque(wx - 1, wy, wz + 1);
                        let a20 = is_opaque(wx - 1, wy + 1, wz - 1);
                        let a21 = is_opaque(wx - 1, wy + 1, wz);
                        let a22 = is_opaque(wx - 1, wy + 1, wz + 1);

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
                    if should_draw_face(wx, wy + 1, wz) {
                        let a00 = is_opaque(wx - 1, wy + 1, wz - 1);
                        let a01 = is_opaque(wx - 1, wy + 1, wz);
                        let a02 = is_opaque(wx - 1, wy + 1, wz + 1);
                        let a10 = is_opaque(wx, wy + 1, wz - 1);
                        let a12 = is_opaque(wx, wy + 1, wz + 1);
                        let a20 = is_opaque(wx + 1, wy + 1, wz - 1);
                        let a21 = is_opaque(wx + 1, wy + 1, wz);
                        let a22 = is_opaque(wx + 1, wy + 1, wz + 1);

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
                    if should_draw_face(wx, wy - 1, wz) {
                        let a00 = is_opaque(wx - 1, wy - 1, wz - 1);
                        let a01 = is_opaque(wx - 1, wy - 1, wz);
                        let a02 = is_opaque(wx - 1, wy - 1, wz + 1);
                        let a10 = is_opaque(wx, wy - 1, wz - 1);
                        let a12 = is_opaque(wx, wy - 1, wz + 1);
                        let a20 = is_opaque(wx + 1, wy - 1, wz - 1);
                        let a21 = is_opaque(wx + 1, wy - 1, wz);
                        let a22 = is_opaque(wx + 1, wy - 1, wz + 1);

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
                    if should_draw_face(wx, wy, wz + 1) {
                        let a00 = is_opaque(wx - 1, wy - 1, wz + 1);
                        let a01 = is_opaque(wx - 1, wy, wz + 1);
                        let a02 = is_opaque(wx - 1, wy + 1, wz + 1);
                        let a10 = is_opaque(wx, wy - 1, wz + 1);
                        let a12 = is_opaque(wx, wy + 1, wz + 1);
                        let a20 = is_opaque(wx + 1, wy - 1, wz + 1);
                        let a21 = is_opaque(wx + 1, wy, wz + 1);
                        let a22 = is_opaque(wx + 1, wy + 1, wz + 1);

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
                    if should_draw_face(wx, wy, wz - 1) {
                        let a00 = is_opaque(wx - 1, wy - 1, wz - 1);
                        let a01 = is_opaque(wx - 1, wy, wz - 1);
                        let a02 = is_opaque(wx - 1, wy + 1, wz - 1);
                        let a10 = is_opaque(wx, wy - 1, wz - 1);
                        let a12 = is_opaque(wx, wy + 1, wz - 1);
                        let a20 = is_opaque(wx + 1, wy - 1, wz - 1);
                        let a21 = is_opaque(wx + 1, wy, wz - 1);
                        let a22 = is_opaque(wx + 1, wy + 1, wz - 1);

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

        Self {
            vertices,
            opaque_indices,
            transparent_indices,
        }
    }
}
