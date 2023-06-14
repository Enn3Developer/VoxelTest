#![allow(non_snake_case)]

use crate::app::App;
use crate::camera::{Camera, CameraController, CameraUniform, Projection};
use crate::frustum::FrustumCuller;
use crate::instance::{Instance, InstanceRaw, NUM_INSTANCES_PER_ROW, SPACE_BETWEEN};
use crate::light::LightUniform;
use crate::model::{DrawLight, DrawModel, Model, ModelVertex, Vertex};
use crate::resource::load_model;
use crate::texture::Texture;
use bytemuck::cast_slice;
use glam::{Mat4, Quat, Vec3, Vec3A};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::iter;
use std::time::{Duration, Instant};
use wgpu::util::{BufferInitDescriptor, DeviceExt, StagingBelt};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, Buffer, BufferBindingType,
    BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
    DepthStencilState, Device, Face, Features, FragmentState, FrontFace, InstanceDescriptor,
    Limits, LoadOp, MultisampleState, Operations, PipelineLayout, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassDepthStencilAttachment, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, SamplerBindingType, ShaderModuleDescriptor, ShaderSource, ShaderStages,
    StencilState, Surface, SurfaceConfiguration, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDimension, VertexBufferLayout, VertexState,
};
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::dpi::PhysicalSize;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod app;
mod assets;
mod camera;
mod frustum;
mod input;
mod instance;
mod light;
mod model;
mod resource;
mod texture;

pub fn create_render_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    color_format: TextureFormat,
    depth_format: Option<TextureFormat>,
    vertex_layouts: &[VertexBufferLayout],
    shader: ShaderModuleDescriptor,
) -> RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: color_format,
                blend: Some(BlendState {
                    alpha: BlendComponent::REPLACE,
                    color: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            polygon_mode: PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}

struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    camera: Camera,
    projection: Projection,
    camera_controller: CameraController,
    mouse_pressed: bool,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    window: Window,
    instances: Vec<Instance>,
    depth_texture: Texture,
    obj_model: Model,
    light_uniform: LightUniform,
    light_buffer: Buffer,
    light_bind_group: BindGroup,
    light_render_pipeline: RenderPipeline,
    glyph_brush: GlyphBrush<()>,
    staging_belt: StagingBelt,
    fps: u32,
    calc_fps: u32,
    last_time: f32,
}

impl State {
    async fn new(window: Window) -> Self {
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

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

        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    Instance::new((x, 0.0, z))
                })
            })
            .collect::<Vec<Instance>>();

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let obj_model = load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
            .await
            .unwrap();

        let light_uniform = LightUniform::new([2.0, 2.0, 2.0], [1.0, 1.0, 1.0], 8.0);

        let light_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Light VB"),
            contents: cast_slice(&[light_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let light_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                count: None,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
        });

        let light_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &light_bind_group_layout,
            label: None,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &camera_bind_group_layout,
                &light_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = {
            let shader = ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: ShaderSource::Wgsl(include_str!("../shaders/shader.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: ShaderSource::Wgsl(include_str!("../shaders/light.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[ModelVertex::desc()],
                shader,
            )
        };

        let font = FontArc::try_from_slice(include_bytes!("../fonts/FiraSans-Regular.ttf"))
            .expect("Can't load font");

        let glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, surface_format);
        let staging_belt = StagingBelt::new(1024);

        Self {
            mouse_pressed: false,
            fps: 0,
            calc_fps: 0,
            last_time: 0.0,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera,
            projection,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            window,
            instances,
            depth_texture,
            obj_model,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            glyph_brush,
            staging_belt,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
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

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));

        let old_position = Vec3A::from_array(*self.light_uniform.position());
        self.light_uniform.set_position(
            (Quat::from_axis_angle(Vec3::Y, 1.05 * dt.as_secs_f32()) * old_position).to_array(),
        );
        self.queue
            .write_buffer(&self.light_buffer, 0, cast_slice(&[self.light_uniform]));

        self.last_time += dt.as_secs_f32();
        self.calc_fps += 1;

        if self.last_time >= 1.0 {
            self.fps = self.calc_fps;
            self.calc_fps = 0;
            self.last_time = 0.0;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

            let instance_data = self
                .instances
                .par_iter()
                .filter(|instance| {
                    instance.position.distance_squared(self.camera.position())
                        < self.projection.z_far().powi(2)
                })
                .filter(|instance| culling.test_bounding_box(instance.aabb()))
                .map(|instance| instance.to_raw())
                .collect::<Vec<InstanceRaw>>();

            let instance_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: cast_slice(&instance_data),
                usage: BufferUsages::VERTEX,
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
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

            render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.obj_model,
                &self.camera_bind_group,
                &self.light_bind_group,
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..instance_data.len() as u32,
                &self.camera_bind_group,
                &self.light_bind_group,
            );
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
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("VoxelTest")
        .build(&event_loop)
        .unwrap();

    // let mut state = State::new(window).await;
    let mut app = App::new(window).await;
    let mut last_render_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // if state.mouse_pressed {
                //     state.camera_controller.process_mouse(delta.0, delta.1)
                // }
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.window().id() && !app.input(event) => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    app.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    app.resize(**new_inner_size);
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == app.window().id() => {
                let now = Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                app.update(dt);
                match app.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        app.resize(app.size())
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                app.window().request_redraw();
            }
            _ => {}
        }
    });
}
