use anyhow::{Result, anyhow};
use ply_rs_bw as ply;

use crate::model::{Color, Mesh, Rect, Vec3};
use crate::render;

pub(crate) struct PlyMesh {
    aabb: Rect,
    vertices: Vec<Vertex>,
    faces: Vec<Face>,
}

impl PlyMesh {
    pub(crate) fn import(mut data: &[u8]) -> Result<PlyMesh> {
        // create a parser
        let parser = ply::parser::Parser::<ply::ply::DefaultElement>::new();

        // use the parser: read the entire file
        let ply = parser.read_ply(&mut data)?;

        let vertices = &ply.payload["vertex"];
        let faces = &ply.payload["face"];

        let mut builder = PlyMeshBuilder::new();

        builder.set_num_vertices(vertices.len());
        builder.set_num_faces(faces.len());

        let mut positions: Vec<Vec3> = Vec::with_capacity(vertices.len());
        let mut normals: Vec<Option<Vec3>> = Vec::with_capacity(vertices.len());
        let mut colors: Vec<Option<Color>> = Vec::with_capacity(vertices.len());

        for v in vertices.iter() {
            let x: f32 = v
                .get("x")
                .ok_or(anyhow!("Property 'x' not found."))
                .map(Self::property_as_float)??;
            let y: f32 = v
                .get("y")
                .ok_or(anyhow!("Property 'y' not found."))
                .map(Self::property_as_float)??;
            let z: f32 = v
                .get("z")
                .ok_or(anyhow!("Property 'z' not found."))
                .map(Self::property_as_float)??;

            let position = Vec3::new(x, y, z);

            let nx: Option<f32> = v.get("nx").map(Self::property_as_float).transpose()?;
            let ny: Option<f32> = v.get("ny").map(Self::property_as_float).transpose()?;
            let nz: Option<f32> = v.get("nz").map(Self::property_as_float).transpose()?;

            let normal = if let Some(nx) = nx
                && let Some(ny) = ny
                && let Some(nz) = nz
            {
                Some(Vec3::new(nx, ny, nz))
            } else {
                None
            };

            let r: Option<f32> = v
                .get("red")
                .map(Self::property_as_color_value)
                .transpose()?;
            let g: Option<f32> = v
                .get("green")
                .map(Self::property_as_color_value)
                .transpose()?;
            let b: Option<f32> = v
                .get("blue")
                .map(Self::property_as_color_value)
                .transpose()?;
            let a: Option<f32> = v
                .get("alpha")
                .map(Self::property_as_color_value)
                .transpose()?;

            let color = if let Some(r) = r
                && let Some(g) = g
                && let Some(b) = b
            {
                Some(Color::from_rgba(r, g, b, a))
            } else {
                None
            };

            positions.push(position);
            normals.push(normal);
            colors.push(color);
        }

        for f in faces.iter() {
            let vidx: Vec<usize> = f
                .get("vertex_indices")
                .ok_or(anyhow!("Property 'vertex_indices' not found."))
                .map(Self::property_as_list)??;

            let face = Face::new(vidx);

            builder.add_face(face);
        }

        positions
            .into_iter()
            .zip(normals)
            .zip(colors)
            .map(|((p, n), c)| Vertex::new(p, n, c))
            .for_each(|v| {
                builder.add_vertex(v);
            });

        builder.build()
    }

    fn property_as_float(prop: &ply::ply::Property) -> Result<f32> {
        match prop {
            ply::ply::Property::Char(n) => Ok(*n as f32),
            ply::ply::Property::UChar(n) => Ok(*n as f32),
            ply::ply::Property::Short(n) => Ok(*n as f32),
            ply::ply::Property::UShort(n) => Ok(*n as f32),
            ply::ply::Property::Int(n) => Ok(*n as f32),
            ply::ply::Property::UInt(n) => Ok(*n as f32),
            ply::ply::Property::Float(n) => Ok(*n),
            ply::ply::Property::Double(n) => Ok(*n as f32),
            _ => Err(anyhow!("Invalid property: {:?}", prop)),
        }
    }

    fn property_as_color_value(prop: &ply::ply::Property) -> Result<f32> {
        match prop {
            ply::ply::Property::UChar(n) => Ok(*n as f32 / 255.),
            ply::ply::Property::Float(n) => Ok(*n),
            ply::ply::Property::Double(n) => Ok(*n as f32),
            _ => Err(anyhow!("Invalid property: {:?}", prop)),
        }
    }

    fn property_as_list(prop: &ply::ply::Property) -> Result<Vec<usize>> {
        match prop {
            ply::ply::Property::ListChar(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            ply::ply::Property::ListUChar(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            ply::ply::Property::ListShort(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            ply::ply::Property::ListUShort(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            ply::ply::Property::ListInt(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            ply::ply::Property::ListUInt(lst) => Ok(lst.iter().map(|n| *n as usize).collect()),
            _ => Err(anyhow!("Invalid property: {:?}", prop)),
        }
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

                let p0 = v0.position();
                let p1 = v1.position();
                let p2 = v2.position();

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
