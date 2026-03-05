use anyhow::{Result, anyhow};

use super::parse::parse;
use crate::model::{Color, Mesh, Rect, Vec3};
use crate::render;

pub(crate) struct OffMesh {
    aabb: Rect,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl OffMesh {
    pub(crate) fn import(data: &[u8]) -> Result<OffMesh> {
        let s = str::from_utf8(data)?;
        let (_, builder) = parse(s).map_err(|e| e.to_owned())?;

        builder.build()
    }
}

impl Mesh for OffMesh {
    fn aabb(&self) -> Rect {
        self.aabb
    }

    fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    fn num_faces(&self) -> usize {
        self.faces.len()
    }

    fn to_triangle_mesh(&self) -> Vec<render::Vertex> {
        let mut vertices = Vec::new();

        for f in self.faces.iter() {
            let face_color = f.color;

            f.make_triangle(|vidx0, vidx1, vidx2| {
                let v0 = self.vertices.get(vidx0).unwrap();
                let v1 = self.vertices.get(vidx1).unwrap();
                let v2 = self.vertices.get(vidx2).unwrap();

                let p0 = v0.position;
                let p1 = v1.position;
                let p2 = v2.position;

                let face_normal = (p1 - p0).cross(p2 - p0).normalize();

                vertices.push(v0.to_render_vertex(face_color, face_normal));
                vertices.push(v1.to_render_vertex(face_color, face_normal));
                vertices.push(v2.to_render_vertex(face_color, face_normal));
            });
        }

        vertices
    }
}

#[derive(Debug, Clone)]
pub(super) struct Vertex {
    position: Vec3,
    color: Option<Color>,
}

impl Vertex {
    pub(super) fn new(position: Vec3, color: Option<Color>) -> Self {
        Self { position, color }
    }

    fn is_valid(&self) -> bool {
        self.color.as_ref().is_none_or(Color::is_valid)
    }

    fn to_render_vertex(&self, color_overwrite: Option<Color>, normal: Vec3) -> render::Vertex {
        let color = color_overwrite.or(self.color);

        render::Vertex::new(self.position, color, normal)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Idx3 {
    idx: [usize; 3],
}

impl Idx3 {
    pub(super) fn new(idx: [usize; 3]) -> Self {
        Self { idx }
    }

    fn max_idx(&self) -> usize {
        self.idx.iter().fold(0, |m, v| std::cmp::max(m, *v))
    }

    fn make_triangle<F: FnMut(usize, usize, usize)>(&self, mut f: F) {
        f(self.idx[0], self.idx[1], self.idx[2]);
    }
}

#[derive(Debug, Clone)]
pub(super) struct Idx4 {
    idx: [usize; 4],
}

impl Idx4 {
    pub(super) fn new(idx: [usize; 4]) -> Self {
        Self { idx }
    }

    fn max_idx(&self) -> usize {
        self.idx.iter().fold(0, |m, v| std::cmp::max(m, *v))
    }

    fn make_triangle<F: FnMut(usize, usize, usize)>(&self, mut f: F) {
        f(self.idx[0], self.idx[1], self.idx[2]);
        f(self.idx[0], self.idx[2], self.idx[3]);
    }
}

#[derive(Debug, Clone)]
pub(super) struct IdxN {
    idx: Vec<usize>,
}

impl IdxN {
    pub(super) fn new(idx: Vec<usize>) -> Self {
        Self { idx }
    }

    fn max_idx(&self) -> usize {
        self.idx.iter().fold(0, |m, v| std::cmp::max(m, *v))
    }

    fn make_triangle<F: FnMut(usize, usize, usize)>(&self, mut f: F) {
        assert!(4 < self.idx.len());
        for i in 1..(self.idx.len() - 1) {
            f(self.idx[0], self.idx[i], self.idx[i + 1]);
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum VertIdx {
    Idx3(Idx3),
    Idx4(Idx4),
    IdxN(IdxN),
}

impl VertIdx {
    fn is_valid(&self, upper_idx: usize) -> bool {
        let max_idx = match self {
            Self::Idx3(idx) => idx.max_idx(),
            Self::Idx4(idx) => idx.max_idx(),
            Self::IdxN(idx) => idx.max_idx(),
        };
        max_idx <= upper_idx
    }

    fn make_triangle<F: FnMut(usize, usize, usize)>(&self, f: F) {
        match self {
            Self::Idx3(idx) => idx.make_triangle(f),
            Self::Idx4(idx) => idx.make_triangle(f),
            Self::IdxN(idx) => idx.make_triangle(f),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Face {
    vidx: VertIdx,
    color: Option<Color>,
}

impl Face {
    pub(super) fn new(vidx: VertIdx, color: Option<Color>) -> Self {
        Self { vidx, color }
    }

    fn is_valid(&self, upper_idx: usize) -> bool {
        self.vidx.is_valid(upper_idx) && self.color.as_ref().is_none_or(Color::is_valid)
    }

    fn make_triangle<F: FnMut(usize, usize, usize)>(&self, f: F) {
        self.vidx.make_triangle(f);
    }
}

#[derive(Debug, Clone)]
pub(super) struct OffMeshBuilder {
    aabb: Rect,
    num_vertices: usize,
    num_faces: usize,
    _num_edges: usize,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl OffMeshBuilder {
    pub(super) fn new() -> Self {
        Self {
            aabb: Rect::new(),
            num_vertices: 0,
            num_faces: 0,
            _num_edges: 0,
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    fn build(self) -> Result<OffMesh> {
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

        Ok(OffMesh {
            aabb: self.aabb,
            vertices: self.vertices,
            faces: self.faces,
        })
    }

    pub(super) fn set_num_vertices(&mut self, num: usize) -> &mut Self {
        self.num_vertices = num;
        self.vertices.reserve(num);
        self
    }

    pub(super) fn set_num_faces(&mut self, num: usize) -> &mut Self {
        self.num_faces = num;
        self.faces.reserve(num);
        self
    }

    pub(super) fn _set_num_edges(&mut self, num: usize) -> &mut Self {
        self._num_edges = num;
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
