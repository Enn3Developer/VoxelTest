use std::cmp::Ordering;
use std::sync::Arc;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use vulkano::{Version, VulkanLibrary};
use vulkano_win::VkSurfaceBuild;
use winit::event::{DeviceEvent, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

pub struct Engine {
    frame_number: u64,
    surface: Arc<Surface>,
}

impl Engine {
    pub fn new(surface: Arc<Surface>) -> Self {
        Self {
            frame_number: 0,
            surface,
        }
    }

    pub fn run(&mut self) {}

    pub fn draw(&mut self) {}
}

pub fn run_engine() {
    let library = VulkanLibrary::new().unwrap();
    let required_extensions = vulkano_win::required_extensions(&library);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            max_api_version: Some(Version::V1_1),
            application_name: Some(String::from("VoxelTest")),
            ..Default::default()
        },
    )
    .unwrap();
    let physical_device = instance
        .enumerate_physical_devices()
        .unwrap()
        .max_by(|device| {
            if device.properties().device_type == PhysicalDeviceType::DiscreteGpu {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        })
        .unwrap();
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let mut engine = Engine::new(surface);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        }
        | Event::DeviceEvent {
            event:
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                }),
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::RedrawRequested(window_id) => {
            engine.run();
            engine.draw();
        }
        _ => (),
    });
}
