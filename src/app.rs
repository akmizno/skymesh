use anyhow::{Result, anyhow};
use eframe::egui_wgpu;
use egui::widgets::RadioButton;
use poll_promise::Promise;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::camera::Camera;
use crate::import::{ImportedMesh, import};
use crate::model::Mesh;
use crate::render::{Extent2d, ProjectionType, SceneDeta, TriangleRenderer};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    projection_type: ProjectionType,

    #[serde(skip)] // This how you opt-out of serialization of a field
    import_promise: Option<Promise<Result<()>>>,

    #[serde(skip)] // This how you opt-out of serialization of a field
    model: Arc<Mutex<Option<(String, ImportedMesh)>>>,

    #[serde(skip)] // This how you opt-out of serialization of a field
    scene_data: Arc<Mutex<SceneDeta>>,

    #[serde(skip)] // This how you opt-out of serialization of a field
    renderer: Arc<Mutex<Option<TriangleRenderer>>>,
}

impl Default for App {
    fn default() -> Self {
        let projection_type = ProjectionType::Perspective;
        let camera = Camera::new(projection_type.is_perspective());
        let scene_data = SceneDeta::from_camera(camera);

        Self {
            projection_type,
            import_promise: None,
            model: Arc::new(Mutex::new(None)),
            scene_data: Arc::new(Mutex::new(scene_data)),
            renderer: Arc::new(Mutex::new(None)),
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        if let Ok(mut scene_data_guard) = app.scene_data.lock() {
            scene_data_guard
                .camera_mut()
                .set_projection_type(app.projection_type.is_perspective());
        }

        let wgpu_render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("Wgpu render state not found");

        let renderer = TriangleRenderer::new(wgpu_render_state);

        if let Ok(mut renderer_guard) = app.renderer.lock() {
            *renderer_guard = Some(renderer);
        }

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(app.renderer.clone());

        app
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let render_state = frame.wgpu_render_state().unwrap();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open file...").clicked() {
                        let open_file = async {
                            let file = rfd::AsyncFileDialog::new()
                                .add_filter("PLY File Format", &["ply"])
                                .add_filter("STL File Format", &["stl"])
                                .add_filter("Object File Format", &["off"])
                                .pick_file()
                                .await;

                            if let Some(file_handle) = file {
                                let name = file_handle.file_name();
                                if let Some(ext) = Path::new(&name)
                                    .extension()
                                    .map(|s| s.display().to_string())
                                {
                                    let bytes = file_handle.read().await;
                                    match import(&ext, bytes.as_slice()) {
                                        Ok(mesh) => Ok((name, mesh)),
                                        Err(e) => Err(e),
                                    }
                                } else {
                                    Err(anyhow!("No extension."))
                                }
                            } else {
                                Err(anyhow!("Canceled."))
                            }
                        };

                        let ctx = ctx.clone();
                        let model = self.model.clone();
                        let scene_data = self.scene_data.clone();
                        let task = async move {
                            let result = open_file.await;

                            let (name, mesh) = if let Err(e) = result {
                                log::warn!("{}", e.root_cause());
                                return Err(e);
                            } else {
                                result.unwrap()
                            };

                            if let Ok(mut model_guard) = model.lock()
                                && let Ok(mut scene_data_guard) = scene_data.lock()
                            {
                                *scene_data_guard.vertices_mut() = Some(mesh.to_triangle_mesh());
                                scene_data_guard
                                    .camera_mut()
                                    .reset_camera_by_aabb(&mesh.aabb());
                                ctx.request_repaint();

                                *model_guard = Some((name, mesh));
                            }

                            Ok(())
                        };

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            self.import_promise =
                                Some(Promise::from_ready(pollster::block_on(task)));
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            self.import_promise = Some(Promise::spawn_local(task));
                        }
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Reset view").clicked()
                        && let Ok(model_guard) = self.model.lock()
                        && let Some((_, mesh)) = model_guard.as_ref()
                        && let Ok(mut scene_data_guard) = self.scene_data.lock()
                    {
                        scene_data_guard
                            .camera_mut()
                            .reset_camera_by_aabb(&mesh.aabb());
                    }

                    ui.menu_button("Projection type", |ui| {
                        if ui
                            .add(RadioButton::new(
                                self.projection_type == ProjectionType::Perspective,
                                "Perspective",
                            ))
                            .clicked()
                        {
                            self.projection_type = ProjectionType::Perspective;
                            if let Ok(mut scene_data_guard) = self.scene_data.lock() {
                                scene_data_guard.camera_mut().set_projection_type(true);
                            }
                        } else if ui
                            .add(RadioButton::new(
                                self.projection_type == ProjectionType::Orthographic,
                                "Orthographic",
                            ))
                            .clicked()
                        {
                            self.projection_type = ProjectionType::Orthographic;
                            if let Ok(mut scene_data_guard) = self.scene_data.lock() {
                                scene_data_guard.camera_mut().set_projection_type(false);
                            }
                        }
                    });
                });

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    egui::widgets::global_theme_preference_buttons,
                );
            });
        });

        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(promise) = &self.import_promise {
                    match promise.ready() {
                        None => {
                            ui.spinner();
                        }
                        Some(Ok(_)) => {
                            ui.label("Imported.");
                        }
                        Some(Err(e)) => {
                            ui.label(e.to_string());
                        }
                    };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{} v{}",
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION")
                    ));
                });
            });
        });

        // egui::SidePanel::left("left panel").show(ctx, |ui| {
        //     ui.heading("Left Panel");
        // });

        egui::SidePanel::right("right panel").show(ctx, |ui| {
            ui.heading("Properties");

            egui::Grid::new("Model properties")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    if let Ok(model_guard) = self.model.lock()
                        && let Some((name, mesh)) = model_guard.as_ref()
                    {
                        let size = mesh.aabb().size();

                        ui.label("Name");
                        ui.label(name);
                        ui.end_row();

                        ui.label("Width");
                        ui.label(format!("{}", size.x));
                        ui.end_row();

                        ui.label("Height");
                        ui.label(format!("{}", size.y));
                        ui.end_row();

                        ui.label("Depth");
                        ui.label(format!("{}", size.z));
                        ui.end_row();

                        ui.label("Number of Vertices");
                        ui.label(format!("{}", mesh.num_vertices()));
                        ui.end_row();

                        ui.label("Number of Faces");
                        ui.label(format!("{}", mesh.num_faces()));
                        ui.end_row();
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            if let Ok(mut scene_data_guard) = self.scene_data.lock() {
                scene_data_guard
                    .camera_mut()
                    .set_aspect_ratio(rect.aspect_ratio());
            }

            ui.input(|i| {
                if let Ok(mut scene_data_guard) = self.scene_data.lock() {
                    let area_size = (rect.width(), rect.height());

                    if i.pointer.primary_down() {
                        let delta = i.pointer.delta();
                        scene_data_guard
                            .camera_mut()
                            .orbit((delta.x, delta.y), area_size);
                    } else if i.pointer.secondary_down() {
                        let delta = i.pointer.delta();
                        scene_data_guard
                            .camera_mut()
                            .pan((delta.x, delta.y), area_size);
                    }

                    let scroll_delta = i.smooth_scroll_delta.y;
                    if !(-0.01..0.01).contains(&scroll_delta) {
                        let scroll_sensitivity = 0.005;
                        scene_data_guard
                            .camera_mut()
                            .dolly(scroll_delta, scroll_sensitivity);
                    }
                }
            });

            if let Ok(mut renderer_guard) = self.renderer.lock()
                && let Some(renderer) = renderer_guard.as_mut()
            {
                renderer.update_target_size(
                    render_state,
                    Extent2d::new(rect.width() as u32, rect.height() as u32),
                );
            }

            let cb = egui_wgpu::Callback::new_paint_callback(
                rect,
                TriangleCallback {
                    scene_data: self.scene_data.clone(),
                },
            );
            ui.painter().add(cb);

            if let Ok(renderer_guard) = self.renderer.lock()
                && let Some(renderer) = renderer_guard.as_ref()
                && let Some(target) = renderer.target()
            {
                ui.image(egui::load::SizedTexture::new(target.egui_id(), rect.size()));
            }
        });
    }
}

struct TriangleCallback {
    scene_data: Arc<Mutex<SceneDeta>>,
}

impl egui_wgpu::CallbackTrait for TriangleCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(r) = callback_resources.get::<Arc<Mutex<Option<TriangleRenderer>>>>()
            && let Ok(mut renderer_guard) = r.lock()
            && let Some(renderer) = renderer_guard.as_mut()
            && let Ok(mut scene_data_guard) = self.scene_data.lock()
        {
            renderer.prepare(device, queue, &mut scene_data_guard);

            if let Some(mut render_pass) = renderer.create_render_pass(egui_encoder) {
                renderer.paint(&mut render_pass);
                drop(render_pass);
            }
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        // noop
    }
}
