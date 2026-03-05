use anyhow::Result;

use super::parse;
use crate::model::{Color, Mesh, Rect, Vec3};
use crate::render;

pub(crate) struct StlMesh {
    aabb: Rect,
    faces: Vec<Face>,
}

impl StlMesh {
    pub(crate) fn import(data: &[u8]) -> Result<StlMesh> {
        // Try ASCII
        if let Ok(s) = str::from_utf8(data)
            && let Ok((_, builder)) = parse::ascii::parse(s).map_err(|e| log::debug!("{}", e))
        {
            return builder.build();
        }

        // Try Binary
        let (_, builder) = parse::binary::parse(data).map_err(|e| e.to_owned())?;
        builder.build()
    }
}

impl Mesh for StlMesh {
    fn aabb(&self) -> Rect {
        self.aabb
    }

    fn num_vertices(&self) -> usize {
        self.num_faces() * 3
    }

    fn num_faces(&self) -> usize {
        self.faces.len()
    }

    fn to_triangle_mesh(&self) -> Vec<render::Vertex> {
        let mut v = Vec::new();

        for face in &self.faces {
            let normal = face.normal();
            let vertices = face.vertices();
            let vert0 = &vertices[0];
            let vert1 = &vertices[1];
            let vert2 = &vertices[2];

            v.push(vert0.to_render_vertex(None, normal));
            v.push(vert1.to_render_vertex(None, normal));
            v.push(vert2.to_render_vertex(None, normal));
        }

        v
    }
}

#[derive(Debug, Clone)]
pub(super) struct Vertex {
    position: Vec3,
}

impl Vertex {
    pub(super) fn new(position: Vec3) -> Self {
        Self { position }
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn to_render_vertex(&self, color: Option<Color>, normal: Vec3) -> render::Vertex {
        render::Vertex::new(self.position, color, normal)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Face {
    normal: Vec3,
    vertices: [Vertex; 3],
}

impl Face {
    pub(super) fn new(normal: Vec3, vertices: [Vertex; 3]) -> Self {
        Self { normal, vertices }
    }

    fn vertices(&self) -> &[Vertex; 3] {
        &self.vertices
    }

    fn normal(&self) -> Vec3 {
        self.normal
    }
}

#[derive(Debug, Clone)]
pub(super) struct StlMeshBuilder {
    aabb: Rect,
    faces: Vec<Face>,
}

impl StlMeshBuilder {
    pub(super) fn new() -> Self {
        Self {
            aabb: Rect::new(),
            faces: Vec::new(),
        }
    }

    fn build(self) -> Result<StlMesh> {
        Ok(StlMesh {
            aabb: self.aabb,
            faces: self.faces,
        })
    }

    pub(super) fn set_num_faces(&mut self, num: usize) -> &mut Self {
        self.faces.reserve(num);
        self
    }

    pub(super) fn add_face(&mut self, f: Face) -> &mut Self {
        let vertices = f.vertices();
        let v0 = &vertices[0];
        let v1 = &vertices[1];
        let v2 = &vertices[2];

        self.aabb.expand(v0.position());
        self.aabb.expand(v1.position());
        self.aabb.expand(v2.position());

        self.faces.push(f);
        self
    }
}
