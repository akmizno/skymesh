pub(crate) type Vec3 = glam::Vec3;
pub(crate) type Mat4 = glam::Mat4;
pub(crate) type Quat = glam::Quat;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pt_min: Vec3,
    pt_max: Vec3,
}

impl Rect {
    pub(crate) fn new() -> Self {
        Self {
            pt_min: Vec3::MAX,
            pt_max: Vec3::MIN,
        }
    }

    pub(crate) fn expand(&mut self, pt: Vec3) {
        self.pt_min = self.pt_min.min(pt);
        self.pt_max = self.pt_max.max(pt);
    }

    pub(crate) fn center(&self) -> Vec3 {
        self.pt_min.midpoint(self.pt_max)
    }

    pub(crate) fn size(&self) -> Vec3 {
        self.pt_max - self.pt_min
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    pub(crate) fn is_valid(&self) -> bool {
        !((self.r < 0. || 1. < self.r)
            || (self.g < 0. || 1. < self.g)
            || (self.b < 0. || 1. < self.b)
            || (self.a < 0. || 1. < self.a))
    }

    pub(crate) fn from_rgba(r: f32, g: f32, b: f32, a: Option<f32>) -> Self {
        Color {
            r,
            g,
            b,
            a: a.unwrap_or(1.),
        }
    }

    pub(crate) fn from_rgba8(r: u8, g: u8, b: u8, a: Option<u8>) -> Self {
        const SCALE: f32 = 1. / 255.;

        Color::from_rgba(
            r as f32 * SCALE,
            g as f32 * SCALE,
            b as f32 * SCALE,
            a.map(|a| a as f32 * SCALE),
        )
    }

    pub(crate) fn to_rgba(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 0.7,
            g: 0.7,
            b: 0.7,
            a: 1.0,
        }
    }
}

pub(crate) trait Mesh {
    fn aabb(&self) -> Rect;
    fn num_vertices(&self) -> usize;
    fn num_faces(&self) -> usize;
    fn to_triangle_mesh(&self) -> Vec<crate::render::Vertex>;
}
