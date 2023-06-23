#![allow(non_snake_case)]

use crate::app::App;
use app::NModel;
use camera::CameraController;
use chunks::Chunk;
use glam::{UVec3, Vec3A};
use std::time::Instant;
use uuid::Uuid;
use wgpu::{
    BlendComponent, BlendState, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
    DepthStencilState, Device, Face, FragmentState, FrontFace, MultisampleState, PipelineLayout,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, StencilState, TextureFormat, VertexBufferLayout, VertexState,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod app;
mod assets;
mod camera;
mod chunks;
mod command_buffer;
mod frustum;
mod input;
mod instance;
mod light;
mod model;
mod resource;
mod texture;
mod ui;

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

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("VoxelTest")
        .build(&event_loop)
        .unwrap();

    // let mut state = State::new(window).await;
    let mut app = App::new(window).await;
    let camera_controller = Box::new(CameraController::new(4.0, 1.0, app.camera()));
    app.add_actor(camera_controller);
    app.register_model("cube.obj");
    let radius = 32;
    let half_radius = radius / 2;
    for chunk_x in -half_radius..=half_radius {
        for chunk_z in -half_radius..=half_radius {
            let mut chunk = Chunk::new(
                Uuid::new_v4(),
                Vec3A::new(chunk_x as f32, 0., chunk_z as f32),
            );
            for x in 0..16 {
                for z in 0..16 {
                    chunk.add_block_data(UVec3::new(x, 0, z), 0);
                }
            }

            app.add_model(NModel::new(Box::new(chunk)));
        }
    }
    let mut last_render_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
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
