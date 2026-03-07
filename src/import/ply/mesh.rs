use anyhow::{Result, anyhow};

use super::parse::PlyParser;
use crate::model::{Color, Mesh, Rect, Vec3};
use crate::render;

pub(crate) struct PlyMesh {
    aabb: Rect,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl PlyMesh {
    pub(crate) fn import(data: &[u8]) -> Result<PlyMesh> {
        let (_, builder) = PlyParser::parse(data, PlyMeshBuilder::new())?;
        builder.build()
    }
}

impl Mesh for PlyMesh {
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
        let mut vertices = Vec::new();

        for f in self.faces.iter() {
            f.make_triangles(|vidx0, vidx1, vidx2| {
                let v0 = self.vertices.get(vidx0).unwrap();
                let v1 = self.vertices.get(vidx1).unwrap();
                let v2 = self.vertices.get(vidx2).unwrap();

                let p0 = v0.position;
                let p1 = v1.position;
                let p2 = v2.position;

                let face_normal = (p1 - p0).cross(p2 - p0).normalize();

                vertices.push(v0.to_render_vertex(face_normal));
                vertices.push(v1.to_render_vertex(face_normal));
                vertices.push(v2.to_render_vertex(face_normal));
            });
        }

        vertices
    }
}

#[derive(Debug, Clone)]
pub(super) struct Vertex {
    position: Vec3,
    normal: Option<Vec3>,
    color: Option<Color>,
}

impl Vertex {
    pub(super) fn new(position: Vec3, normal: Option<Vec3>, color: Option<Color>) -> Self {
        Self {
            position,
            normal,
            color,
        }
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn is_valid(&self) -> bool {
        self.color.as_ref().is_none_or(Color::is_valid)
    }

    fn to_render_vertex(&self, face_normal: Vec3) -> render::Vertex {
        let normal = self.normal.unwrap_or(face_normal);
        render::Vertex::new(self.position, self.color, normal)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Face {
    vidx: Vec<usize>,
}

impl Face {
    pub(super) fn new(vidx: Vec<usize>) -> Self {
        Self { vidx }
    }

    fn is_valid(&self, upper_index: usize) -> bool {
        2 < self.vidx.len() && self.vidx.iter().all(|i| *i < upper_index)
    }

    fn make_triangles<F: FnMut(usize, usize, usize)>(&self, mut f: F) {
        for i in 1..(self.vidx.len() - 1) {
            f(self.vidx[0], self.vidx[i], self.vidx[i + 1]);
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PlyMeshBuilder {
    aabb: Rect,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl PlyMeshBuilder {
    pub(super) fn new() -> Self {
        Self {
            aabb: Rect::new(),
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    fn build(self) -> Result<PlyMesh> {
        for v in &self.vertices {
            if !v.is_valid() {
                return Err(anyhow!("Invlid Vertex: {:?}", v));
            }
        }

        let upper_idx = self.vertices.len();
        for f in &self.faces {
            if !f.is_valid(upper_idx) {
                return Err(anyhow!("Invlid Face: {:?}", f));
            }
        }

        Ok(PlyMesh {
            aabb: self.aabb,
            vertices: self.vertices,
            faces: self.faces,
        })
    }

    pub(super) fn set_num_vertices(&mut self, num: usize) -> &mut Self {
        self.vertices.reserve(num);
        self
    }

    pub(super) fn set_num_faces(&mut self, num: usize) -> &mut Self {
        self.faces.reserve(num);
        self
    }

    pub(super) fn add_vertex(&mut self, v: Vertex) -> &mut Self {
        self.aabb.expand(v.position);
        self.vertices.push(v);
        self
    }

    pub(super) fn add_face(&mut self, f: Face) -> &mut Self {
        self.faces.push(f);
        self
    }
}
