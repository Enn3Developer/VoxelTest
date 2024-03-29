use crate::camera::{Camera, CameraUniform, Projection};
use crate::command_buffer::{
    CommandBuffer, NCommandRender, NCommandSetup, NCommandUpdate, NResource,
};
use crate::create_render_pipeline;
use crate::frustum::{Aabb, FrustumCuller};
use crate::input::InputState;
use crate::model::{DrawModel, ModelVertex, Vertex};
use crate::resource::load_model;
use crate::texture::Texture;
use bytemuck::cast_slice;
use glam::{Mat4, Vec3A};
use glyphon::{
    Attrs, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer,
};
use rayon::prelude::*;
use std::cell::RefCell;
use std::iter;
use std::ops::Deref;
use std::rc::Rc;
use std::slice::Iter;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, Color, CommandEncoderDescriptor, CompareFunction, DepthStencilState, Device,
    Features, InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, PowerPreference, PresentMode, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RequestAdapterOptions, SamplerBindingType, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, Surface, SurfaceConfiguration, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::window::Window;

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

pub struct NBuffer {
    buffer: Buffer,
    uniform: Rc<RefCell<Vec<u8>>>,
}

impl NBuffer {
    pub fn new(buffer: Buffer, uniform: Rc<RefCell<Vec<u8>>>) -> Self {
        Self { buffer, uniform }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn update(&self, queue: &Queue) {
        queue.write_buffer(&self.buffer, 0, &self.uniform.borrow());
    }
}

pub struct NBindGroup {
    bind_group: BindGroup,
    layout: BindGroupLayout,
}

impl NBindGroup {
    pub fn new(bind_group: BindGroup, layout: BindGroupLayout) -> Self {
        Self { bind_group, layout }
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }
}

pub struct NModel {
    model: Box<dyn Model + Send + Sync>,
    pipelines: Vec<Rc<RenderPipeline>>,
    buffers: Vec<NBuffer>,
    bind_groups: Vec<NBindGroup>,
}

impl NModel {
    pub fn new(model: Box<dyn Model + Send + Sync>) -> Self {
        Self {
            model,
            pipelines: vec![],
            buffers: vec![],
            bind_groups: vec![],
        }
    }

    pub fn add_pipeline(&mut self, pipeline: RenderPipeline) {
        self.pipelines.push(Rc::new(pipeline));
    }

    pub fn add_pipeline_rc(&mut self, pipeline: Rc<RenderPipeline>) {
        self.pipelines.push(pipeline);
    }

    pub fn pipelines(&self) -> &[Rc<RenderPipeline>] {
        &self.pipelines
    }

    pub fn add_buffer(&mut self, buffer: NBuffer) {
        self.buffers.push(buffer);
    }

    pub fn buffers(&self) -> &[NBuffer] {
        &self.buffers
    }

    pub fn add_bind_group(&mut self, bind_group: NBindGroup) {
        self.bind_groups.push(bind_group);
    }

    pub fn bind_groups(&self) -> &[NBindGroup] {
        &self.bind_groups
    }

    pub fn update_buffer(&self, queue: &Queue, idx: usize) {
        self.buffers[idx].update(queue);
    }
}

impl Deref for NModel {
    type Target = Box<dyn Model + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

unsafe impl Sync for NModel {}

pub struct ModelState {
    models: Vec<NModel>,
}

impl ModelState {
    pub fn new() -> Self {
        Self { models: vec![] }
    }

    pub fn get_model(&self, id: &Uuid) -> Option<&NModel> {
        self.models.iter().find(|&model| model.id() == id)
    }

    pub fn push(&mut self, model: NModel) {
        self.models.push(model);
    }

    pub fn iter_models(&self) -> Iter<'_, NModel> {
        self.models.iter()
    }

    pub fn models(&self) -> &[NModel] {
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

pub struct App<'a> {
    actors: ActorState,
    models: Rc<RefCell<ModelState>>,
    input_state: InputState,

    surface: Surface<'a>,
    device: Rc<Device>,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Arc<Window>,
    depth_texture: Rc<Texture>,

    camera: Rc<RefCell<Camera>>,
    projection: Projection,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group_layout: BindGroupLayout,
    camera_bind_group: Rc<BindGroup>,

    model_layout: BindGroupLayout,
    obj_models: Vec<crate::model::ObjModel>,

    font_system: FontSystem,
    cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffer: glyphon::Buffer,

    calc_fps: u32,
    last_time: f32,
}

impl App<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
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
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth_texture = Rc::new(Texture::create_depth_texture(
            &device,
            &config,
            "depth_texture",
        ));

        let camera = Rc::new(RefCell::new(Camera::new((0.0, 5.0, 10.0), -1.57, -0.35)));
        let projection = Projection::new(config.width, config.height, 0.78, 0.1, 4096.0);

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

        let camera_bind_group = Rc::new(device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        }));

        let mut font_system = FontSystem::new();
        let cache = SwashCache::new();
        let mut atlas = TextAtlas::new(&device, &queue, TextureFormat::Bgra8UnormSrgb);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            &device,
            MultisampleState::default(),
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Never,
                stencil: Default::default(),
                bias: Default::default(),
            }),
        );
        let mut buffer = glyphon::Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));

        buffer.set_size(&mut font_system, config.width as f32, config.height as f32);
        buffer.set_text(
            &mut font_system,
            "0 fps",
            Attrs::new().family(Family::SansSerif),
            Shaping::Basic,
        );
        buffer.shape_until_scroll(&mut font_system);

        let model_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        Self {
            actors: ActorState::new(),
            models: Rc::new(RefCell::new(ModelState::new())),
            input_state: InputState::new(),

            surface,
            device,
            queue,
            config,
            size,
            window,
            depth_texture,

            camera,
            projection,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            camera_uniform,

            model_layout,
            obj_models: vec![],

            font_system,
            cache,
            text_atlas: atlas,
            text_renderer,
            text_buffer: buffer,

            calc_fps: 0,
            last_time: 0.0,
        }
    }

    pub fn add_model(&mut self, mut model: NModel) {
        let buffer = model.setup();

        for command in buffer.iter_command() {
            self.parse_setup_command(command, &mut model);
        }
        self.models.borrow_mut().push(model);
    }

    pub fn add_actor(&mut self, actor: Box<dyn Actor + Send>) {
        self.actors.push(actor);
    }

    pub fn register_model(&mut self, name: &str) {
        self.obj_models
            .push(load_model(name, &self.device, &self.queue, &self.model_layout).unwrap());
    }

    pub fn camera(&self) -> Rc<RefCell<Camera>> {
        self.camera.clone()
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = *new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.projection.resize(new_size.width, new_size.height);

            self.depth_texture = Rc::new(Texture::create_depth_texture(
                &self.device,
                &self.config,
                "depth_texture",
            ));
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.input_state.input(event)
    }

    pub fn parse_update_command(&mut self, command: NCommandUpdate) {
        match command {
            NCommandUpdate::CreateModel(model) => {
                let mut n_model = NModel::new(model);
                let buffer = n_model.setup();

                for command in buffer.iter_command() {
                    self.parse_setup_command(command, &mut n_model);
                }

                self.models.borrow_mut().push(n_model);
            }
            NCommandUpdate::CreateActor(actor) => {
                self.actors.push(actor);
            }
            NCommandUpdate::RemoveModel(id) => {
                let mut idx = None;
                for (i, model) in self.models.borrow().iter_models().enumerate() {
                    if model.id() == &id {
                        idx = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx {
                    self.models.borrow_mut().remove(i);
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
            NCommandUpdate::UpdateBuffer(id, idx) => {
                self.models
                    .borrow_mut()
                    .get_model(&id)
                    .unwrap()
                    .update_buffer(&self.queue, idx);
            }
        }
    }

    pub fn parse_setup_command(&self, command: NCommandSetup, n_model: &mut NModel) {
        match command {
            NCommandSetup::CreateBuffer(uniform, buffer_usages) => {
                let buffer = self.device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: &uniform.borrow(),
                    usage: buffer_usages,
                });
                let n_buffer = NBuffer::new(buffer, uniform);
                n_model.add_buffer(n_buffer);
            }
            NCommandSetup::CreateBindGroup(layout_entries, resources) => {
                let layout = self
                    .device
                    .create_bind_group_layout(&BindGroupLayoutDescriptor {
                        label: None,
                        entries: &layout_entries,
                    });

                let entries: Vec<BindGroupEntry> = resources
                    .iter()
                    .enumerate()
                    .map(|(idx, resource)| {
                        let r = match resource {
                            NResource::Buffer(i) => {
                                n_model.buffers()[*i].buffer().as_entire_binding()
                            }
                        };
                        BindGroupEntry {
                            binding: idx as u32,
                            resource: r,
                        }
                    })
                    .collect();

                let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &layout,
                    entries: &entries,
                });

                n_model.add_bind_group(NBindGroup::new(bind_group, layout));
            }
            NCommandSetup::CreatePipeline(bind_groups, shader, mut vertex_layouts, use_model) => {
                let mut bind_group_layouts = vec![];
                if use_model {
                    bind_group_layouts.push(&self.model_layout);
                    vertex_layouts.insert(0, ModelVertex::desc());
                }
                bind_group_layouts.push(&self.camera_bind_group_layout);
                bind_group_layouts.append(
                    &mut bind_groups
                        .iter()
                        .map(|idx| n_model.bind_groups[*idx].layout())
                        .collect::<Vec<_>>(),
                );

                let pipeline_layout =
                    self.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &bind_group_layouts,
                            push_constant_ranges: &[],
                        });
                let shader = ShaderModuleDescriptor {
                    label: None,
                    source: ShaderSource::Wgsl(shader.into()),
                };

                let render_pipeline = create_render_pipeline(
                    &self.device,
                    &pipeline_layout,
                    self.config.format,
                    Some(Texture::DEPTH_FORMAT),
                    &vertex_layouts,
                    shader,
                );

                n_model.add_pipeline(render_pipeline);
            }
            NCommandSetup::SharePipeline(id, idx) => {
                if let Some(model) = self.models.borrow().get_model(id) {
                    let pipeline = model.pipelines()[idx].clone();
                    n_model.add_pipeline_rc(pipeline);
                }
            }
        }
    }

    pub fn parse_render_command<'b, 'a: 'b>(
        &'a self,
        command: NCommandRender,
        model: &'a NModel,
        render_pass: &'b mut RenderPass<'a>,
    ) {
        match command {
            NCommandRender::SetPipeline(idx) => {
                render_pass.set_pipeline(&model.pipelines()[idx]);
            }
            NCommandRender::SetVertexBuffer(slot, idx) => {
                render_pass.set_vertex_buffer(slot, model.buffers[idx].buffer().slice(..));
            }
            NCommandRender::SetIndexBuffer(idx, index_format) => {
                render_pass.set_index_buffer(model.buffers[idx].buffer().slice(..), index_format);
            }
            NCommandRender::SetBindGroup(i, idx) => {
                render_pass.set_bind_group(i, model.bind_groups()[idx].bind_group(), &[]);
            }
            NCommandRender::DrawIndexed(indices, instances) => {
                render_pass.draw_indexed(0..indices, 0, 0..instances);
            }
            NCommandRender::DrawModelIndexed(idx, instances, bind_groups_idx) => {
                let bind_groups: Vec<&BindGroup> = bind_groups_idx
                    .iter()
                    .map(|i| model.bind_groups()[*i].bind_group())
                    .collect();
                render_pass.draw_model_instanced(
                    &self.obj_models[idx],
                    0..instances,
                    &self.camera_bind_group,
                    None,
                    &bind_groups,
                );
            }
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
                    self.parse_update_command(command);
                }
            });

        self.camera_uniform
            .update_view_proj(&self.camera.borrow(), &self.projection);
        self.queue
            .write_buffer(&self.camera_buffer, 0, cast_slice(&[self.camera_uniform]));

        self.last_time += dt.as_secs_f32();
        self.calc_fps += 1;

        if self.last_time >= 1.0 {
            println!("{} fps", self.calc_fps);
            self.text_buffer.set_text(
                &mut self.font_system,
                &format!("{} fps", self.calc_fps),
                Attrs::new().family(Family::SansSerif),
                Shaping::Basic,
            );
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
            let depth = self.depth_texture.clone();
            let cam_bind_group = self.camera_bind_group.clone();
            let models = self.models.clone();
            let models = models.borrow();
            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.text_atlas,
                    Resolution {
                        width: self.config.width,
                        height: self.config.height,
                    },
                    [TextArea {
                        buffer: &self.text_buffer,
                        left: 10.0,
                        top: 10.0,
                        scale: 1.0,
                        bounds: TextBounds {
                            left: 0,
                            top: 0,
                            right: 600,
                            bottom: 160,
                        },
                        default_color: glyphon::Color::rgb(255, 255, 255),
                    }],
                    &mut self.cache,
                )
                .unwrap();
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
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_bind_group(0, &cam_bind_group, &[]);

            let cam_position = self.camera.borrow().position();

            models
                .models()
                .par_iter()
                .filter(|model| culling.test_bounding_box(model.aabb()))
                .filter(|model| {
                    model.position().distance_squared(cam_position)
                        < self.projection.z_far().powi(2)
                })
                .map(|model| (model, model.render()))
                .collect::<Vec<(&NModel, CommandBuffer<NCommandRender>)>>()
                .into_iter()
                .for_each(|(model, command_buffer)| {
                    for command in command_buffer.iter_command() {
                        self.parse_render_command(command, model, &mut render_pass);
                    }
                });
            self.text_renderer
                .render(&self.text_atlas, &mut render_pass)
                .unwrap();
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        self.text_atlas.trim();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}
