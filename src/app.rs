use crate::camera::{Camera, CameraController, CameraUniform, Projection};
use crate::command_buffer::{CommandBuffer, NCommand};
use crate::frustum::{Aabb, FrustumCuller};
use crate::input::InputState;
use crate::texture::Texture;
use bytemuck::cast_slice;
use glam::{Mat4, Vec3A};
use std::iter;
use std::slice::{Iter, IterMut};
use std::time::Duration;
use wgpu::util::{BufferInitDescriptor, DeviceExt, StagingBelt};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Color, Device,
    Features, InstanceDescriptor, Limits, LoadOp, Operations, PowerPreference, PresentMode, Queue,
    RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RequestAdapterOptions, ShaderStages, Surface, SurfaceConfiguration, TextureUsages,
};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

pub trait Actor {
    fn update(&mut self, dt: &Duration, input_state: &InputState, queue: &Queue) -> CommandBuffer;
}
pub trait Model {
    fn aabb(&self) -> &Aabb;
    fn position(&self) -> &Vec3A;
    fn render(&self, render_pass: &mut RenderPass, device: &Device);
}

pub struct ModelState {
    models: Vec<Box<dyn Model>>,
}

impl ModelState {
    pub fn new() -> Self {
        Self { models: vec![] }
    }

    pub fn push(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    pub fn iter_models(&self) -> Iter<'_, Box<dyn Model>> {
        self.models.iter()
    }
}

pub struct ActorState {
    actors: Vec<Box<dyn Actor>>,
}

impl ActorState {
    pub fn new() -> Self {
        Self { actors: vec![] }
    }

    pub fn push(&mut self, actor: Box<dyn Actor>) {
        self.actors.push(actor);
    }

    pub fn iter_mut_actors(&mut self) -> IterMut<'_, Box<dyn Actor>> {
        self.actors.iter_mut()
    }
}

pub struct App {
    actors: ActorState,
    models: ModelState,
    input_state: InputState,

    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Window,
    glyph_brush: GlyphBrush<()>,
    staging_belt: StagingBelt,
    depth_texture: Texture,

    camera: Camera,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    camera_controller: CameraController,

    fps: u32,
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

        let camera = Camera::new((0.0, 5.0, 10.0), -1.57, -0.35);
        let projection = Projection::new(config.width, config.height, 0.78, 0.1, 1000.0);
        let camera_controller = CameraController::new(4.0, 1.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

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

        let font = FontArc::try_from_slice(include_bytes!("../fonts/FiraSans-Regular.ttf"))
            .expect("Can't load font");

        let glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, surface_format);
        let staging_belt = StagingBelt::new(1024);

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
            glyph_brush,
            staging_belt,
            depth_texture,

            camera,
            projection,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            camera_controller,

            fps: 0,
            calc_fps: 0,
            last_time: 0.0,
        }
    }

    pub fn add_model(&mut self, model: Box<dyn Model>) {
        self.models.push(model);
    }

    pub fn add_actor(&mut self, actor: Box<dyn Actor>) {
        self.actors.push(actor);
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

    pub fn parse_command(&mut self, command: NCommand) {
        match command {
            NCommand::CreateModel(model) => {
                self.models.push(model);
            }
            NCommand::CreateActor(actor) => {
                self.actors.push(actor);
            }
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));

        let mut buffers = vec![];

        for actor in self.actors.iter_mut_actors() {
            let command_buffer = actor.update(&dt, &self.input_state, &self.queue);
            buffers.push(command_buffer);
        }

        for command_buffer in buffers {
            for command in command_buffer.iter_command() {
                self.parse_command(command);
            }
        }

        self.last_time += dt.as_secs_f32();
        self.calc_fps += 1;

        if self.last_time >= 1.0 {
            self.fps = self.calc_fps;
            self.calc_fps = 0;
            self.last_time = 0.0;
        }

        self.input_state.clear();
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

            self.models
                .iter_models()
                .filter(|model| culling.test_bounding_box(model.aabb()))
                .filter(|model| {
                    model.position().distance_squared(self.camera.position())
                        < self.projection.z_far().powi(2)
                })
                .for_each(|model| model.render(&mut render_pass, &self.device));
        }

        self.glyph_brush.queue(Section {
            screen_position: (10.0, 10.0),
            bounds: (self.config.width as f32, self.config.height as f32),
            text: vec![Text::new(&format!("fps: {}", self.fps)).with_color([1.0, 1.0, 1.0, 1.0])],
            ..Default::default()
        });

        self.glyph_brush
            .draw_queued(
                &self.device,
                &mut self.staging_belt,
                &mut encoder,
                &view,
                self.config.width,
                self.config.height,
            )
            .expect("Can't draw text");

        self.staging_belt.finish();
        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}
