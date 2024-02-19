use crate::vertex::Vertex;

/// Generates a vertex buffer for a circle approximation with the given number of segments and radius.
pub fn circle_vertices(num_segments: u32, radius: f32) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    // Calculate the vertex data for the circle
    for i in 0..num_segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32) / (num_segments as f32);
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        let z = 0.0; // Assuming z-coordinate is 0 for 2D circle
        let s = 1.0 - (x / (2.0 * radius) + 0.5); // Map x to texture coordinates
        let t = 1.0 - (y / (2.0 * radius) + 0.5); // Map y to texture coordinates

        let vertex = Vertex {
            position: [x, y, z],
            tex_coords: [s, t],
        };

        vertices.push(vertex);
    }

    vertices
}

pub fn n_gon_index_buffer(n: u16) -> Vec<u16> {
    let mut indices = Vec::new();

    for i in 0..n {
        indices.push(0);
        indices.push(i + 1);
        indices.push(i + 2);
    }

    indices
}