use crate::model::{Mat4, Quat, Rect, Vec3};

const DEFAULT_AREA_SCALE: f32 = 2.;

#[derive(Debug, Clone)]
struct PerspectiveProjection {
    fov_y_radians: f32,
    aspect_ratio: f32,
    z_near: f32,
    z_far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            fov_y_radians: std::f32::consts::PI * 0.25,
            aspect_ratio: 1.0,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }
}

impl PerspectiveProjection {
    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    fn fov(&self) -> f32 {
        self.fov_y_radians
    }

    fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    fn to_mat(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_radians,
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }
}

#[derive(Debug, Clone)]
struct View {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
}

impl Default for View {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 0.0, 1.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
        }
    }
}

impl View {
    fn reset_by_aabb(&mut self, aabb: &Rect, fov: f32) {
        let target = aabb.center();
        let h = aabb.size().y;
        let eye_depth = DEFAULT_AREA_SCALE * ((h * 0.5) / (fov * 0.5).tan()).abs();
        let eye_z = target.z + eye_depth;

        self.eye = Vec3::new(target.x, target.y, eye_z);
        self.target = target;
        self.up = Vec3::Y;
    }

    fn eye(&self) -> Vec3 {
        self.eye
    }

    fn target(&self) -> Vec3 {
        self.target
    }

    fn up(&self) -> Vec3 {
        self.up
    }

    fn relative_pos(&self) -> Vec3 {
        self.eye - self.target
    }

    fn distance(&self) -> f32 {
        self.relative_pos().length()
    }

    fn right(&self) -> Vec3 {
        self.up().cross(self.relative_pos()).normalize()
    }

    fn pan(&mut self, dx: f32, dy: f32) {
        let v = self.up() * dy + self.right() * dx;
        self.eye += v;
        self.target += v;
    }

    fn orbit(&mut self, yaw: f32, pitch: f32) {
        let yaw_quat = Quat::from_axis_angle(self.up(), yaw);
        let pitch_quat = Quat::from_axis_angle(self.right(), pitch);

        let rot = yaw_quat * pitch_quat;

        self.eye = self.target() + rot * self.relative_pos();
        self.up = rot * self.up();
    }

    fn dolly(&mut self, dolly_factor: f32) {
        self.eye = self.target() + dolly_factor * self.relative_pos();
    }

    fn to_mat(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye(), self.target(), self.up())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Camera {
    view: View,

    is_perspective: bool,
    proj: PerspectiveProjection,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            view: Default::default(),
            is_perspective: true,
            proj: Default::default(),
        }
    }
}

impl Camera {
    pub(crate) fn new(is_perspective: bool) -> Self {
        let mut camera = Camera::default();
        camera.set_projection_type(is_perspective);
        camera
    }

    pub(crate) fn set_projection_type(&mut self, is_perspective: bool) {
        self.is_perspective = is_perspective
    }

    pub(crate) fn reset_camera_by_aabb(&mut self, aabb: &Rect) {
        self.view.reset_by_aabb(aabb, self.proj.fov());
    }

    pub(crate) fn _reset_camera_by_default(&mut self) {
        self.view = Default::default();
        self.proj = Default::default();
    }

    pub(crate) fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.proj.set_aspect_ratio(aspect_ratio);
    }

    pub(crate) fn pan(&mut self, pointer_delta: (f32, f32), area_size: (f32, f32)) {
        let world_unit_per_px = self.ortho_height() / area_size.1;

        let dx = -pointer_delta.0 * world_unit_per_px;
        let dy = pointer_delta.1 * world_unit_per_px;
        self.view.pan(dx, dy);
    }

    pub(crate) fn orbit(&mut self, pointer_delta: (f32, f32), area_size: (f32, f32)) {
        let pi = std::f32::consts::PI;
        let yaw = -pi * pointer_delta.0 / area_size.0;
        let pitch = -pi * pointer_delta.1 / area_size.1;
        self.view.orbit(yaw, pitch);
    }

    pub(crate) fn dolly(&mut self, scroll_delta: f32, sensitivity: f32) {
        let dolly_factor = (-scroll_delta * sensitivity).exp();
        self.view.dolly(dolly_factor);
    }

    fn ortho_height(&self) -> f32 {
        self.view.distance() * 2.0 * (self.proj.fov() * 0.5).tan()
    }

    fn ortho_size(&self) -> (f32, f32) {
        let h = self.ortho_height();
        let w = h * self.proj.aspect_ratio();
        (w, h)
    }

    fn proj_mat_ortho(&self) -> Mat4 {
        let (w, h) = self.ortho_size();
        let w_half = 0.5 * w;
        let h_half = 0.5 * h;

        let left = -w_half;
        let right = w_half;
        let bottom = -h_half;
        let top = h_half;

        Mat4::orthographic_rh(left, right, bottom, top, self.proj.z_near, self.proj.z_far)
    }

    fn proj_mat_persp(&self) -> Mat4 {
        self.proj.to_mat()
    }

    fn proj_mat(&self) -> Mat4 {
        if self.is_perspective {
            self.proj_mat_persp()
        } else {
            self.proj_mat_ortho()
        }
    }

    fn view_mat(&self) -> Mat4 {
        self.view.to_mat()
    }

    // View-Projection Matrix
    pub(crate) fn to_mat(&self) -> Mat4 {
        self.proj_mat() * self.view_mat()
    }

    pub(crate) fn direction(&self) -> Vec3 {
        self.view.relative_pos()
    }
}
