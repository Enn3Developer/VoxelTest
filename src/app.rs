use crate::camera::{Camera, CameraUniform, Projection};
use crate::command_buffer::{CommandBuffer, NCommandRender, NCommandSetup, NCommandUpdate};
use crate::frustum::{Aabb, FrustumCuller};
use crate::input::InputState;
use crate::texture::Texture;
use crate::ui::{Label, UI};
use bytemuck::cast_slice;
use glam::{Mat4, Vec3A};
use rayon::prelude::*;
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::slice::{Iter, IterMut};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use uuid::Uuid;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Color,
    CommandEncoderDescriptor, Device, Features, InstanceDescriptor, Limits, LoadOp, Operations,
    PowerPreference, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RequestAdapterOptions, ShaderStages,
    Surface, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

// TODO: implement buffers for everything (update, setup render, render)
// TODO: abstract render work from Model

pub trait Actor {
    fn id(&self) -> &Uuid;
    fn update(&mut self, dt: &Duration, input_state: &InputState) -> CommandBuffer<NCommandUpdate>;
}

pub trait Model {
    fn id(&self) -> &Uuid;
    fn aabb(&self) -> &Aabb;
    fn position(&self) -> &Vec3A;
    fn setup(&self) -> CommandBuffer<NCommandSetup>;
    fn render(&self) -> CommandBuffer<NCommandRender>;
}

pub struct ModelState {
    models: Vec<Box<dyn Model + Send + Sync>>,
}

impl ModelState {
    pub fn new() -> Self {
        Self { models: vec![] }
    }

    pub fn push(&mut self, model: Box<dyn Model + Send + Sync>) {
        self.models.push(model);
    }

    pub fn iter_models(&self) -> Iter<'_, Box<dyn Model + Send + Sync>> {
        self.models.iter()
    }

    pub fn models(&self) -> &Vec<Box<dyn Model + Send + Sync>> {
        &self.models
    }

    fn remove(&mut self, idx: usize) {
        self.models.swap_remove(idx);
    }
}

pub struct ActorState {
    actors: Vec<Box<dyn Actor + Send>>,
}

impl ActorState {
    pub fn new() -> Self {
        Self { actors: vec![] }
    }

    pub fn push(&mut self, actor: Box<dyn Actor + Send>) {
        self.actors.push(actor);
    }

    pub fn iter_mut_actors(&mut self) -> IterMut<'_, Box<dyn Actor + Send>> {
        self.actors.iter_mut()
    }

    pub fn iter_actors(&self) -> Iter<'_, Box<dyn Actor + Send>> {
        self.actors.iter()
    }

    pub fn remove(&mut self, idx: usize) {
        self.actors.swap_remove(idx);
    }

    pub fn mut_actors(&mut self) -> &mut Vec<Box<dyn Actor + Send>> {
        &mut self.actors
    }
}

pub struct App {
    actors: ActorState,
    models: ModelState,
    input_state: InputState,

    surface: Surface,
    device: Rc<Device>,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Window,
    depth_texture: Texture,
    ui: UI,

    camera: Rc<RefCell<Camera>>,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,

    fps_label: Label,
    calc_fps: u32,
    last_time: f32,
}

impl App {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let device = Rc::new(device);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let camera = Rc::new(RefCell::new(Camera::new((0.0, 5.0, 10.0), -1.57, -0.35)));
        let projection = Projection::new(config.width, config.height, 0.78, 0.1, 1000.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera.borrow(), &projection);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let ui = UI::new(
            include_bytes!("../fonts/FiraSans-Regular.ttf"),
            device.clone(),
            surface_format,
        );

        let fps_label = Label::default()
            .with_position((10.0, 10.0))
            .with_bounds((config.width as f32, config.height as f32))
            .with_text("0 fps");

        Self {
            actors: ActorState::new(),
            models: ModelState::new(),
            input_state: InputState::new(),

            surface,
            device,
            queue,
            config,
            size,
            window,
            depth_texture,
            ui,

            camera,
            projection,
            camera_buffer,
            camera_bind_group,
            camera_uniform,

            fps_label,
            calc_fps: 0,
            last_time: 0.0,
        }
    }

    pub fn add_model(&mut self, model: Box<dyn Model + Send + Sync>) {
        self.models.push(model);
    }

    pub fn add_actor(&mut self, actor: Box<dyn Actor + Send>) {
        self.actors.push(actor);
    }

    pub fn camera(&self) -> Rc<RefCell<Camera>> {
        self.camera.clone()
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.projection.resize(new_size.width, new_size.height);

            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.input_state.input(event)
    }

    pub fn parse_command(&mut self, command: NCommandUpdate) {
        match command {
            NCommandUpdate::CreateModel(model) => {
                self.models.push(model);
            }
            NCommandUpdate::CreateActor(actor) => {
                self.actors.push(actor);
            }
            NCommandUpdate::RemoveModel(id) => {
                let mut idx = None;
                for (i, model) in self.models.iter_models().enumerate() {
                    if model.id() == &id {
                        idx = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx {
                    self.models.remove(i);
                }
            }
            NCommandUpdate::RemoveActor(id) => {
                let mut idx = None;
                for (i, actor) in self.actors.iter_actors().enumerate() {
                    if actor.id() == &id {
                        idx = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx {
                    self.actors.remove(i);
                }
            }
            NCommandUpdate::MoveCamera(offset) => {
                self.camera.borrow_mut().move_position(offset);
            }
            NCommandUpdate::RotateCamera(yaw, pitch) => {
                self.camera.borrow_mut().add_yaw(yaw);
                self.camera.borrow_mut().add_pitch(pitch);
            }
            NCommandUpdate::FovCamera(_fov) => {}
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.actors
            .mut_actors()
            .par_iter_mut()
            .map(|actor| actor.update(&dt, &self.input_state))
            .collect::<Vec<CommandBuffer<NCommandUpdate>>>()
            .into_iter()
            .for_each(|buffer| {
                for command in buffer.iter_command() {
                    self.parse_command(command);
                }
            });

        self.camera_uniform
            .update_view_proj(&self.camera.borrow(), &self.projection);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));

        self.last_time += dt.as_secs_f32();
        self.calc_fps += 1;

        if self.last_time >= 1.0 {
            self.fps_label.set_text(format!("{} fps", self.calc_fps));
            self.calc_fps = 0;
            self.last_time = 0.0;
        }

        self.input_state.update();
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let culling = FrustumCuller::from_matrix(Mat4::from_cols_array_2d(
                &self.camera_uniform.view_proj,
            ));
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            let cam_position = self.camera.borrow().position();

            self.models
                .models()
                .par_iter()
                .filter(|model| culling.test_bounding_box(model.aabb()))
                .filter(|model| {
                    model.position().distance_squared(cam_position)
                        < self.projection.z_far().powi(2)
                })
                .map(|model| model.render())
                .for_each(|_command_buffer| {});
        }

        self.ui.render(&self.fps_label);
        self.ui
            .draw(&mut encoder, &view, self.config.width, self.config.height)
            .expect("can't draw");

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        self.ui.recall();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}
