use anyhow::{Result, anyhow};

use crate::model::{Mesh, Rect};
use crate::render::Vertex;

mod off;
use off::OffMesh;

mod stl;
use stl::StlMesh;

mod ply;
use ply::PlyMesh;

pub(crate) enum ImportedMesh {
    Off(OffMesh),
    Stl(StlMesh),
    Ply(PlyMesh),
}

impl Mesh for ImportedMesh {
    fn aabb(&self) -> Rect {
        match self {
            Self::Off(m) => m.aabb(),
            Self::Stl(m) => m.aabb(),
            Self::Ply(m) => m.aabb(),
        }
    }
    fn num_vertices(&self) -> usize {
        match self {
            Self::Off(m) => m.num_vertices(),
            Self::Stl(m) => m.num_vertices(),
            Self::Ply(m) => m.num_vertices(),
        }
    }
    fn num_faces(&self) -> usize {
        match self {
            Self::Off(m) => m.num_faces(),
            Self::Stl(m) => m.num_faces(),
            Self::Ply(m) => m.num_faces(),
        }
    }
    fn to_triangle_mesh(&self) -> Vec<Vertex> {
        match self {
            Self::Off(m) => m.to_triangle_mesh(),
            Self::Stl(m) => m.to_triangle_mesh(),
            Self::Ply(m) => m.to_triangle_mesh(),
        }
    }
}

pub(crate) fn import(ext: &str, data: &[u8]) -> Result<ImportedMesh> {
    let ext = ext.to_ascii_lowercase();
    match ext.as_str() {
        "off" => OffMesh::import(data).map(ImportedMesh::Off),
        "stl" => StlMesh::import(data).map(ImportedMesh::Stl),
        "ply" => PlyMesh::import(data).map(ImportedMesh::Ply),
        _ => Err(anyhow!("Unsupported type: {}.", &ext)),
    }
}
