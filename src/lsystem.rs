use crate::vertex::Vertex;
use glam::{Quat, Vec3};

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
                'X' => next.push_str("F[+X][-X][^X][&X]"),
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

pub fn generate_l_system_tree(iterations: usize, origin: Vec3) -> EntityMesh {
    let string = generate_l_system_string("X", iterations);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let angle = std::f32::consts::PI / 6.0; // 30 degrees

    let mut state = TurtleState {
        pos: origin,
        dir: Vec3::Y,
        up: Vec3::Z,
        right: Vec3::X,
        thickness: 0.3,
        length: 1.0,
    };
    let mut stack = Vec::new();

    for c in string.chars() {
        match c {
            'F' => {
                let end = state.pos + state.dir * state.length;

                // Color: brown for trunk/branches
                let color = [101, 67, 33, 255];
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
            '[' => stack.push(state),
            ']' => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
            }
            'X' => {
                // Draw a leaf at the end of the bud
                let color = [34, 139, 34, 255]; // green
                                                // leaf is just a bigger box at current pos
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

    #[test]
    fn test_lsystem_string_expansion() {
        let axiom = "X";
        let iter_1 = generate_l_system_string(axiom, 1);
        assert_eq!(iter_1, "F[+X][-X][^X][&X]");

        let iter_2 = generate_l_system_string(axiom, 2);
        // F becomes FF
        // X becomes F[+X][-X][^X][&X]
        assert!(iter_2.starts_with("FF[+F[+X]"));
    }
}
