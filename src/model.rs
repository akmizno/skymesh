pub(crate) use glam::Mat4;
pub(crate) use glam::Quat;
pub(crate) use glam::Vec3;

mod camera;
pub(crate) use camera::Camera;

mod light;
pub(crate) use light::{Lighting, Reflection};

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

pub(crate) trait Mesh: Send + Sync + std::fmt::Debug {
    fn aabb(&self) -> Rect;
    fn num_vertices(&self) -> usize;
    fn num_faces(&self) -> usize;
    fn to_triangle_mesh(&self) -> Vec<crate::render::Vertex>;
}

#[derive(Debug)]
pub(crate) struct Model {
    name: String,
    mesh: Box<dyn Mesh>,
}

impl Model {
    pub(crate) fn new<T: Mesh + 'static>(name: String, mesh: T) -> Self {
        Self {
            name,
            mesh: Box::new(mesh),
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

impl Mesh for Model {
    fn aabb(&self) -> Rect {
        self.mesh.aabb()
    }

    fn num_vertices(&self) -> usize {
        self.mesh.num_vertices()
    }

    fn num_faces(&self) -> usize {
        self.mesh.num_faces()
    }

    fn to_triangle_mesh(&self) -> Vec<crate::render::Vertex> {
        self.mesh.to_triangle_mesh()
    }
}

#[derive(Default, Debug)]
pub(crate) struct Document {
    model: Option<Model>,
    camera: Camera,
    lighting: Lighting,
}

impl Document {
    pub(crate) fn lighting(&self) -> &Lighting {
        &self.lighting
    }

    pub(crate) fn reset_lighting(&mut self) {
        self.lighting = Default::default();
    }

    pub(crate) fn set_lighting(&mut self, lighting: Lighting) {
        self.lighting = lighting;
    }

    pub(crate) fn model(&self) -> Option<&Model> {
        self.model.as_ref()
    }

    pub(crate) fn set_model(&mut self, model: Model) {
        let aabb = model.aabb();
        self.model = Some(model);
        self.camera.reset_camera_by_aabb(&aabb);
    }

    pub(crate) fn reset_view(&mut self) {
        if let Some(model) = self.model() {
            self.camera.reset_camera_by_aabb(&model.aabb());
        }
    }

    pub(crate) fn camera(&self) -> &Camera {
        &self.camera
    }

    pub(crate) fn set_view_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.camera.set_aspect_ratio(aspect_ratio);
    }

    pub(crate) fn is_perspective_projection(&self) -> bool {
        self.camera.is_perspective()
    }

    pub(crate) fn set_projection_type(&mut self, is_perspective: bool) {
        self.camera.set_projection_type(is_perspective);
    }

    pub(crate) fn pan_camera(&mut self, pointer_delta: (f32, f32), area_size: (f32, f32)) {
        self.camera.pan(pointer_delta, area_size);
    }

    pub(crate) fn orbit_camera(&mut self, pointer_delta: (f32, f32), area_size: (f32, f32)) {
        self.camera.orbit(pointer_delta, area_size);
    }

    pub(crate) fn dolly_camera(&mut self, scroll_delta: f32, sensitivity: f32) {
        self.camera.dolly(scroll_delta, sensitivity);
    }
}
