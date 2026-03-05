use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::model::{Color, Mat4, Vec3};

#[derive(serde::Deserialize, serde::Serialize, PartialEq)]
pub(crate) enum ProjectionType {
    Perspective,
    Orthographic,
}

impl ProjectionType {
    pub(crate) fn is_perspective(&self) -> bool {
        matches!(self, Self::Perspective)
    }
}

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub(crate) struct Extent2d {
    width: u32,
    height: u32,
}

impl Extent2d {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Default, Debug)]
pub(crate) struct SceneDeta {
    is_dirty_vertices: bool,
    vertices: Option<Vec<Vertex>>,

    is_dirty_camera: bool,
    camera: Camera,
}

impl SceneDeta {
    pub(crate) fn new(vertices: Option<Vec<Vertex>>, camera: Camera) -> Self {
        Self {
            is_dirty_vertices: true,
            vertices,
            is_dirty_camera: true,
            camera,
        }
    }

    pub(crate) fn from_camera(camera: Camera) -> Self {
        Self::new(None, camera)
    }

    pub(crate) fn is_dirty_vertices(&self) -> bool {
        self.is_dirty_vertices
    }

    pub(crate) fn is_dirty_camera(&self) -> bool {
        self.is_dirty_camera
    }

    pub(crate) fn set_dirty_vertices(&mut self, is_dirty: bool) {
        self.is_dirty_vertices = is_dirty;
    }

    pub(crate) fn set_dirty_camera(&mut self, is_dirty: bool) {
        self.is_dirty_camera = is_dirty;
    }

    pub(crate) fn vertices(&self) -> Option<&Vec<Vertex>> {
        self.vertices.as_ref()
    }

    pub(crate) fn vertices_mut(&mut self) -> &mut Option<Vec<Vertex>> {
        self.is_dirty_vertices = true;
        &mut self.vertices
    }

    pub(crate) fn camera(&self) -> &Camera {
        &self.camera
    }

    pub(crate) fn camera_mut(&mut self) -> &mut Camera {
        self.is_dirty_camera = true;
        &mut self.camera
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
    normal: [f32; 3],
}

impl Vertex {
    pub(crate) fn new(position: Vec3, color: Option<Color>, normal: Vec3) -> Self {
        let color = color.unwrap_or_default();

        Self {
            position: [position.x, position.y, position.z],
            color: color.to_rgba(),
            normal: [normal.x, normal.y, normal.z],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn from_camera(camera: &Camera) -> Self {
        Self::from_mat4(&camera.to_mat())
    }
    fn from_mat4(mat4: &Mat4) -> Self {
        Self {
            view_proj: mat4.to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LightUniform {
    dir: [f32; 3],
    _padding0: f32,
    color: [f32; 3],
    _padding1: f32,
}

impl LightUniform {
    const COLOR: [f32; 3] = [1.0, 1.0, 1.0];

    fn new(dir: Vec3) -> Self {
        Self {
            dir: [dir.x, dir.y, dir.z],
            color: Self::COLOR,
            _padding0: 0.,
            _padding1: 0.,
        }
    }

    pub(crate) fn from_camera(camera: &Camera) -> Self {
        let pos = camera.position();
        Self::new(pos)
    }
}

pub(crate) struct RenderTarget {
    egui_id: egui::TextureId,
    _color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,

    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    extent: Extent2d,
}

impl RenderTarget {
    pub(crate) fn new(render_state: &eframe::egui_wgpu::RenderState, extent: Extent2d) -> Self {
        let device = &render_state.device;
        let target_format = render_state.target_format;

        let size = wgpu::Extent3d {
            width: extent.width(),
            height: extent.height(),
            depth_or_array_layers: 1,
        };

        let desc_color = wgpu::TextureDescriptor {
            label: Some("RTT Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let desc_depth = wgpu::TextureDescriptor {
            label: Some("Depth Buffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float, // 32bit浮動小数点が一般的
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let color_texture = device.create_texture(&desc_color);
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&desc_depth);
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let egui_id = render_state.renderer.write().register_native_texture(
            device,
            &color_view,
            wgpu::FilterMode::Linear,
        );

        Self {
            egui_id,
            _color_texture: color_texture,
            color_view,
            _depth_texture: depth_texture,
            depth_view,
            extent,
        }
    }

    pub(crate) fn egui_id(&self) -> egui::TextureId {
        self.egui_id
    }

    fn extent(&self) -> &Extent2d {
        &self.extent
    }

    fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }
}

pub(crate) struct TriangleRenderer {
    pipeline: wgpu::RenderPipeline,

    target: Option<RenderTarget>,

    vertex_buffer: Option<wgpu::Buffer>,
    vertex_buffer_size: u32,
    vertex_buffer_capacity: u32,

    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
}

impl TriangleRenderer {
    pub(crate) fn target(&self) -> Option<&RenderTarget> {
        self.target.as_ref()
    }

    pub(crate) fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        let device = &render_state.device;
        let target_format = render_state.target_format;

        // camera_uniformの初期化のみ
        let camera = Camera::default();
        let camera_uniform = CameraUniform::from_camera(&camera);
        let light_uniform = LightUniform::from_camera(&camera);

        // シェーダーの読み込み
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        };

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[
                    // Camera
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,                             // シェーダー内の @binding(0) と一致させる
                        visibility: wgpu::ShaderStages::VERTEX, // 頂点シェーダーで使う
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Light
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
            ],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout], // ここにレイアウトを入れる
                push_constant_ranges: &[],
            });

        // パイプラインの作成
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout], // レイアウトを指定
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    // blend: None,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    // blend: Some(wgpu::BlendState {
                    //     color: wgpu::BlendComponent {
                    //         src_factor: wgpu::BlendFactor::SrcAlpha,
                    //         dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    //         operation: wgpu::BlendOperation::Add,
                    //     },
                    //     alpha: wgpu::BlendComponent {
                    //         src_factor: wgpu::BlendFactor::One,
                    //         dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    //         operation: wgpu::BlendOperation::Add,
                    //     },
                    // }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true, // 距離を書き込む
                depth_compare: wgpu::CompareFunction::Less, // 「今より近い」場合のみ描画
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            target: None,
            vertex_buffer: None,
            vertex_buffer_size: 0,
            vertex_buffer_capacity: 0,
            camera_bind_group,
            camera_buffer,
            light_buffer,
        }
    }

    pub(crate) fn update_target_size(
        &mut self,
        render_state: &eframe::egui_wgpu::RenderState,
        extent: Extent2d,
    ) -> egui::TextureId {
        if self.target.is_none() {
            self.target = Some(RenderTarget::new(render_state, extent));
        } else if let Some(target) = self.target.as_mut()
            && *target.extent() != extent
        {
            self.target = Some(RenderTarget::new(render_state, extent));
        }

        self.target.as_ref().unwrap().egui_id()
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene_data: &mut SceneDeta,
    ) {
        if let Some(vertices) = scene_data.vertices() {
            let size = vertices.len() as u32;
            if self.vertex_buffer_capacity < size {
                self.vertex_buffer = Some(device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Vertex Buffer"),
                        contents: bytemuck::cast_slice(vertices),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
                self.vertex_buffer_size = size;
                self.vertex_buffer_capacity = size;
            } else {
                self.vertex_buffer_size = size;
            }
        } else {
            self.vertex_buffer = None;
            self.vertex_buffer_size = 0;
            self.vertex_buffer_capacity = 0;
        }

        if scene_data.is_dirty_vertices()
            && let Some(vertices) = scene_data.vertices()
            && let Some(vertex_buffer) = self.vertex_buffer.as_ref()
        {
            let raw_data = bytemuck::cast_slice(vertices);
            queue.write_buffer(vertex_buffer, 0, raw_data);

            scene_data.set_dirty_vertices(false);
        }

        if scene_data.is_dirty_camera() {
            let camera = scene_data.camera();

            let camera_uniform = [CameraUniform::from_camera(camera)];
            let light_uniform = [LightUniform::from_camera(camera)];

            let camera_raw_data = bytemuck::cast_slice(&camera_uniform);
            let light_raw_data = bytemuck::cast_slice(&light_uniform);

            queue.write_buffer(&self.camera_buffer, 0, camera_raw_data);
            queue.write_buffer(&self.light_buffer, 0, light_raw_data);

            scene_data.set_dirty_camera(false);
        }
    }

    pub(crate) fn create_render_pass<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> Option<wgpu::RenderPass<'a>> {
        self.target.as_ref().map(|target| {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Offscreen Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.color_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: target.depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            })
        })
    }

    pub(crate) fn paint<'a>(&self, render_pass: &mut wgpu::RenderPass<'a>) {
        if 0 < self.vertex_buffer_size {
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // バッファをスロット0にセットして描画
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.draw(0..self.vertex_buffer_size, 0..1);
        }
    }
}
