use crate::vertex::Vertex;
use glam::{Quat, Vec3};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::trees::TreeType;

pub struct EntityMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Copy)]
struct TurtleState {
    pos: Vec3,
    dir: Vec3,
    up: Vec3,
    right: Vec3,
    thickness: f32,
    length: f32,
}

pub fn generate_l_system_string(axiom: &str, iterations: usize) -> String {
    let mut string = String::from(axiom);
    for _ in 0..iterations {
        let mut next = String::new();
        for c in string.chars() {
            match c {
                'A' => next.push_str("TT[&&&B][////&&&B][\\\\&&&B]TT[//&&&B][//////&&&B][\\&&&B]A"),
                'B' => next.push_str("TT[++L]L"),
                'L' => next.push_str("TT[--B]&B"),
                'X' => next.push_str("F[+X][-X][^X][&X]X"),
                'O' => next.push_str("F[+^O]T[-^O]T[+&O]T[-&O]"),
                'P' => {
                    next.push_str("FFFF[Y][\\^Y][\\\\^Y][\\\\\\^Y][\\\\\\\\^Y][/^Y][//^Y][///^Y]")
                }
                'Y' => next.push('Y'), // Fronds don't recursively branch
                'T' => next.push('T'), // Non-expanding trunk segment
                'F' => next.push_str("FF"),
                _ => next.push(c),
            }
        }
        string = next;
    }
    string
}

struct BranchParams {
    start: Vec3,
    end: Vec3,
    up: Vec3,
    right: Vec3,
    thickness: f32,
    color: [u8; 4],
}

fn jitter_color(
    mut color: [u8; 4],
    rng: &mut StdRng,
    tree_jitter: i32,
    branch_jitter: i32,
) -> [u8; 4] {
    let r_jitter = tree_jitter + rng.gen_range(-branch_jitter..=branch_jitter);
    let g_jitter = tree_jitter + rng.gen_range(-branch_jitter..=branch_jitter);
    let b_jitter = tree_jitter + rng.gen_range(-branch_jitter..=branch_jitter);
    color[0] = (color[0] as i32 + r_jitter).clamp(0, 255) as u8;
    color[1] = (color[1] as i32 + g_jitter).clamp(0, 255) as u8;
    color[2] = (color[2] as i32 + b_jitter).clamp(0, 255) as u8;
    color
}

pub fn generate_l_system_tree(tree_type: TreeType, origin: Vec3) -> EntityMesh {
    let (axiom, iterations, angle, base_thickness, base_length) = match tree_type {
        TreeType::Palm => ("P", 2, std::f32::consts::PI / 4.0, 0.2, 1.5),
        TreeType::Bush => (
            "[+X][-X][^X][&X]",
            3,
            std::f32::consts::PI / 6.0,
            0.05,
            0.15,
        ),
        TreeType::Birch => ("X", 4, std::f32::consts::PI / 8.0, 0.15, 0.6),
        TreeType::Oak => ("O", 5, std::f32::consts::PI / 4.0, 0.8, 0.8),
        TreeType::Pine => ("A", 5, std::f32::consts::PI / 6.0, 0.5, 1.0),
    };

    let string = generate_l_system_string(axiom, iterations);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let mut rng = StdRng::seed_from_u64((origin.x.to_bits() as u64) ^ (origin.z.to_bits() as u64));
    let height_var = rng.gen_range(0.8..1.2);
    let frond_var = rng.gen_range(0.8..1.2);
    let tree_color_jitter = rng.gen_range(-15..=15);

    let mut state = TurtleState {
        pos: origin,
        dir: Vec3::Y,
        up: Vec3::Z,
        right: Vec3::X,
        thickness: base_thickness * rng.gen_range(0.8..1.2),
        length: base_length * height_var,
    };
    let mut stack = Vec::new();

    for c in string.chars() {
        match c {
            'F' | 'T' => {
                let end = state.pos + state.dir * state.length;

                // Color: brown for trunk/branches
                let base_color = match tree_type {
                    TreeType::Palm => [210, 180, 140, 255], // Pale tan
                    TreeType::Bush => [85, 107, 47, 255],   // Dark olive green
                    TreeType::Birch => [200, 200, 200, 255], // White/Grey
                    TreeType::Oak => [80, 50, 20, 255],     // Darker brown
                    TreeType::Pine => [90, 60, 40, 255],    // Pine brown
                };
                let color = jitter_color(base_color, &mut rng, tree_color_jitter, 5);

                add_branch(
                    &mut vertices,
                    &mut indices,
                    BranchParams {
                        start: state.pos,
                        end,
                        up: state.up,
                        right: state.right,
                        thickness: state.thickness,
                        color,
                    },
                );

                state.pos = end;
                // state.thickness *= 0.9;
            }
            '+' => {
                let rot = Quat::from_axis_angle(state.up, angle);
                state.dir = rot * state.dir;
                state.right = rot * state.right;
            }
            '-' => {
                let rot = Quat::from_axis_angle(state.up, -angle);
                state.dir = rot * state.dir;
                state.right = rot * state.right;
            }
            '^' => {
                let rot = Quat::from_axis_angle(state.right, angle);
                state.dir = rot * state.dir;
                state.up = rot * state.up;
            }
            '&' => {
                let rot = Quat::from_axis_angle(state.right, -angle);
                state.dir = rot * state.dir;
                state.up = rot * state.up;
            }
            '\\' => {
                let rot = Quat::from_axis_angle(state.dir, angle);
                state.up = rot * state.up;
                state.right = rot * state.right;
            }
            '/' => {
                let rot = Quat::from_axis_angle(state.dir, -angle);
                state.up = rot * state.up;
                state.right = rot * state.right;
            }
            '[' => stack.push(state),
            ']' => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
            }
            'X' | 'O' | 'B' | 'A' | 'L' => {
                // Draw a leaf at the end of the bud
                let base_color = match tree_type {
                    TreeType::Bush => [107, 142, 35, 255],  // Olive drab
                    TreeType::Birch => [173, 255, 47, 255], // Bright leaves
                    TreeType::Oak => [34, 110, 34, 255],    // Deep green
                    TreeType::Pine => [20, 70, 20, 255],    // Pine needle dark green
                    _ => [34, 139, 34, 255],                // Default green
                };
                let color = jitter_color(base_color, &mut rng, tree_color_jitter, 10);
                let leaf_pos = state.pos;
                let leaf_end = leaf_pos + state.dir * 0.5;
                add_branch(
                    &mut vertices,
                    &mut indices,
                    BranchParams {
                        start: leaf_pos,
                        end: leaf_end,
                        up: state.up,
                        right: state.right,
                        thickness: 0.8,
                        color,
                    },
                );
            }
            'Y' => {
                // Draw a long, drooping palm frond
                let base_color = [46, 139, 87, 255]; // sea green
                let color = jitter_color(base_color, &mut rng, tree_color_jitter, 8);
                let mut current_pos = state.pos;
                let mut current_dir = state.dir;
                let mut current_up = state.up;
                let mut current_right = state.right;

                let segments = 5;
                // Subtle variation per frond
                let segment_length = 1.2 * frond_var * rng.gen_range(0.9..1.1);

                for i in 0..segments {
                    let end = current_pos + current_dir * segment_length;
                    // Taper the thickness of the frond towards the tip
                    let thickness = 0.6 * (1.0 - (i as f32 / segments as f32) * 0.7);

                    add_branch(
                        &mut vertices,
                        &mut indices,
                        BranchParams {
                            start: current_pos,
                            end,
                            up: current_up,
                            right: current_right,
                            thickness,
                            color,
                        },
                    );
                    current_pos = end;

                    // Pitch down for the next segment (gravity/droop)
                    let mut droop_axis = current_dir.cross(Vec3::NEG_Y);
                    if droop_axis.length_squared() < 0.001 {
                        droop_axis = current_right;
                    } else {
                        droop_axis = droop_axis.normalize();
                    }

                    let pitch_rot = Quat::from_axis_angle(droop_axis, angle * 0.4);
                    current_dir = pitch_rot * current_dir;
                    current_up = pitch_rot * current_up;
                    current_right = pitch_rot * current_right;
                }
            }
            _ => {}
        }
    }

    EntityMesh { vertices, indices }
}

fn add_branch(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, params: BranchParams) {
    let t = params.thickness / 2.0;
    let base_idx = vertices.len() as u32;

    // We'll create a 4-sided branch (square cross-section)
    // 8 vertices: 4 at start, 4 at end

    let p0 = params.start - params.right * t - params.up * t;
    let p1 = params.start + params.right * t - params.up * t;
    let p2 = params.start + params.right * t + params.up * t;
    let p3 = params.start - params.right * t + params.up * t;

    let p4 = params.end - params.right * t - params.up * t;
    let p5 = params.end + params.right * t - params.up * t;
    let p6 = params.end + params.right * t + params.up * t;
    let p7 = params.end - params.right * t + params.up * t;

    let push_vertex = |vts: &mut Vec<Vertex>, p: Vec3, n: Vec3| {
        let nx = (n.x * 127.0) as i8;
        let ny = (n.y * 127.0) as i8;
        let nz = (n.z * 127.0) as i8;
        // material 0 is solid, normal_and_ao: last is AO, just use 127 (fully bright)
        vts.push(Vertex::new(
            [p.x, p.y, p.z],
            0,
            params.color,
            [nx, ny, nz, 127],
        ));
    };

    let dir = (params.end - params.start).normalize_or_zero();

    // Bottom face (-dir)
    let n_bottom = -dir;
    push_vertex(vertices, p0, n_bottom);
    push_vertex(vertices, p1, n_bottom);
    push_vertex(vertices, p2, n_bottom);
    push_vertex(vertices, p3, n_bottom);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 2,
        base_idx + 1,
        base_idx,
        base_idx + 3,
        base_idx + 2,
    ]);

    // Top face (+dir)
    let base_idx = vertices.len() as u32;
    let n_top = dir;
    push_vertex(vertices, p4, n_top);
    push_vertex(vertices, p5, n_top);
    push_vertex(vertices, p6, n_top);
    push_vertex(vertices, p7, n_top);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);

    // Right face (+right)
    let base_idx = vertices.len() as u32;
    let n_right = params.right;
    push_vertex(vertices, p1, n_right);
    push_vertex(vertices, p5, n_right);
    push_vertex(vertices, p6, n_right);
    push_vertex(vertices, p2, n_right);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);

    // Left face (-right)
    let base_idx = vertices.len() as u32;
    let n_left = -params.right;
    push_vertex(vertices, p0, n_left);
    push_vertex(vertices, p3, n_left);
    push_vertex(vertices, p7, n_left);
    push_vertex(vertices, p4, n_left);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);

    // Up face (+up)
    let base_idx = vertices.len() as u32;
    let n_up = params.up;
    push_vertex(vertices, p3, n_up);
    push_vertex(vertices, p2, n_up);
    push_vertex(vertices, p6, n_up);
    push_vertex(vertices, p7, n_up);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);

    // Down face (-up)
    let base_idx = vertices.len() as u32;
    let n_down = -params.up;
    push_vertex(vertices, p0, n_down);
    push_vertex(vertices, p4, n_down);
    push_vertex(vertices, p5, n_down);
    push_vertex(vertices, p1, n_down);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 1,
        base_idx + 2,
        base_idx,
        base_idx + 2,
        base_idx + 3,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn write_ply_header(file: &mut File, num_vertices: usize, num_faces: usize) {
        writeln!(file, "ply").unwrap();
        writeln!(file, "format ascii 1.0").unwrap();
        writeln!(file, "element vertex {}", num_vertices).unwrap();
        writeln!(file, "property float x").unwrap();
        writeln!(file, "property float y").unwrap();
        writeln!(file, "property float z").unwrap();
        writeln!(file, "property uchar red").unwrap();
        writeln!(file, "property uchar green").unwrap();
        writeln!(file, "property uchar blue").unwrap();
        writeln!(file, "property uchar alpha").unwrap();
        writeln!(file, "element face {}", num_faces).unwrap();
        writeln!(file, "property list uchar int vertex_index").unwrap();
        writeln!(file, "end_header").unwrap();
    }

    fn write_vertices(file: &mut File, vertices: &Vec<Vertex>) {
        for v in vertices {
            writeln!(
                file,
                "{} {} {} {} {} {} {}",
                v.position()[0],
                v.position()[1],
                v.position()[2],
                v.color()[0],
                v.color()[1],
                v.color()[2],
                v.color()[3]
            )
            .unwrap();
        }
    }

    fn write_faces(file: &mut File, indices: &Vec<u32>) {
        for chunk in indices.chunks(3) {
            writeln!(file, "3 {} {} {}", chunk[0], chunk[1], chunk[2]).unwrap();
        }
    }

    #[test]
    fn test_lsystem_string_expansion() {
        let axiom = "X";
        let iter_1 = generate_l_system_string(axiom, 1);
        assert_eq!(iter_1, "F[+X][-X][^X][&X]X");

        let iter_2 = generate_l_system_string(axiom, 2);
        // F becomes FF
        // X becomes F[+X][-X][^X][&X]X
        assert!(iter_2.starts_with("FF[+F[+X]"));
    }

    #[test]
    fn test_generate_palm_tree() {
        let mesh = generate_l_system_tree(TreeType::Palm, Vec3::ZERO);
        let mut file = File::create("test_outputs/lsystem_palm.ply").unwrap();

        let num_faces = mesh.indices.len() / 3;

        write_ply_header(&mut file, mesh.vertices.len(), num_faces);
        write_vertices(&mut file, &mesh.vertices);
        write_faces(&mut file, &mesh.indices);
    }

    #[test]
    fn test_generate_bush_ply() {
        let mesh = generate_l_system_tree(TreeType::Bush, Vec3::ZERO);
        let mut file = File::create("test_outputs/lsystem_bush.ply").unwrap();

        let num_faces = mesh.indices.len() / 3;

        write_ply_header(&mut file, mesh.vertices.len(), num_faces);
        write_vertices(&mut file, &mesh.vertices);
        write_faces(&mut file, &mesh.indices);
    }

    #[test]
    fn test_generate_birch() {
        let mesh = generate_l_system_tree(TreeType::Birch, Vec3::ZERO);
        let mut file = File::create("test_outputs/lsystem_birch.ply").unwrap();

        let num_faces = mesh.indices.len() / 3;

        write_ply_header(&mut file, mesh.vertices.len(), num_faces);
        write_vertices(&mut file, &mesh.vertices);
        write_faces(&mut file, &mesh.indices);
    }

    #[test]
    fn test_generate_oak() {
        let mesh = generate_l_system_tree(TreeType::Oak, Vec3::ZERO);
        let mut file = File::create("test_outputs/lsystem_oak.ply").unwrap();

        let num_faces = mesh.indices.len() / 3;

        write_ply_header(&mut file, mesh.vertices.len(), num_faces);
        write_vertices(&mut file, &mesh.vertices);
        write_faces(&mut file, &mesh.indices);
    }

    #[test]
    fn test_generate_pine() {
        let pine = generate_l_system_tree(TreeType::Pine, Vec3::ZERO);
        let mut file = File::create("test_outputs/lsystem_pine.ply").unwrap();

        let num_faces = pine.indices.len() / 3;
        write_ply_header(&mut file, pine.vertices.len(), num_faces);
        write_vertices(&mut file, &pine.vertices);
        write_faces(&mut file, &pine.indices);
    }
}
