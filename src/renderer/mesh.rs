use std::iter::zip;

use log::debug;
use tobj::{load_obj, GPU_LOAD_OPTIONS};

use crate::{vec2, vec3, Vec2, Vec3};

#[derive(Debug)]
pub struct Mesh {
    pub vertices:  Vec<Vertex>,
    pub triangles: Vec<Triangle>,
}

#[derive(Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal:   Vec3,
    pub uv:       Vec2,
}

#[derive(Debug)]
pub struct Triangle {
    pub v1_idx: u32,
    pub v2_idx: u32,
    pub v3_idx: u32,
}

impl Mesh {
    pub fn from_path(path: &str) -> Self {
        let (models, materials_res) =
            load_obj(path, &GPU_LOAD_OPTIONS).unwrap_or_else(|_| panic!("Failed to load models from {}", path));
        let materials = materials_res.unwrap_or_else(|_| panic!("Failed to load materials for {}", path));

        debug!("Loaded {} models from {}", models.len(), path);
        let model = models.get(0).unwrap();
        debug!("Using model {} for {}", model.name, path);

        let positions = &model.mesh.positions;
        let normals = &model.mesh.normals;
        let uvs = &model.mesh.texcoords;
        let indices = &model.mesh.indices;

        let mut vertices = Vec::with_capacity(positions.len() / 3);
        let mut triangles = Vec::with_capacity(indices.len() / 3);

        for (pos_slice, (normal_slice, uv_slice)) in zip(
            positions.chunks_exact(3),
            zip(normals.chunks_exact(3), uvs.chunks_exact(2)),
        ) {
            // Hint to compiler the sice of each slice
            let pos_slice = &pos_slice[0..3];
            let normal_slice = &normal_slice[0..3];
            let uv_slice = &uv_slice[0..2];

            let position = vec3(pos_slice[0], pos_slice[1], pos_slice[2]);
            let normal = vec3(normal_slice[0], normal_slice[1], normal_slice[2]);
            let uv = vec2(uv_slice[0], uv_slice[1]);

            vertices.push(Vertex { position, normal, uv });
        }

        for idx_slice in indices.chunks_exact(3) {
            let idx_slice = &idx_slice[0..3];

            let v1_idx = idx_slice[0];
            let v2_idx = idx_slice[1];
            let v3_idx = idx_slice[2];

            triangles.push(Triangle { v1_idx, v2_idx, v3_idx })
        }

        Self { vertices, triangles }
    }

    pub fn len_vertices(&self) -> u32 { self.vertices.len() as u32 }
    pub fn len_triangles(&self) -> u32 { self.triangles.len() as u32 }
}
