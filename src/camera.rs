use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec3A};
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta, VirtualKeyCode};

use crate::input::InputState;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct Camera {
    position: Vec3A,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new<V: Into<Vec3A>>(position: V, yaw: f32, pitch: f32) -> Self {
        Self {
            position: position.into(),
            yaw,
            pitch,
        }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Mat4::look_to_rh(
            self.position.into(),
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vec3::Y,
        )
    }

    pub fn position(&self) -> Vec3A {
        self.position
    }
}

pub struct Projection {
    aspect: f32,
    fov_y: f32,
    z_near: f32,
    z_far: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fov_y: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fov_y,
            z_near,
            z_far,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn z_far(&self) -> f32 {
        self.z_far
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
    pub ambient_strength: f32,
    _padding: [f32; 3],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::default().to_cols_array_2d(),
            ambient_strength: 0.01,
            _padding: [0.0; 3],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        let eye = camera.position.to_array();
        self.view_position = [eye[0], eye[1], eye[2], 0.0];
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d();
    }
}

pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    samples: u32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            samples: 0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, inputs: &InputState) {
        if inputs.is_key_just_pressed(&VirtualKeyCode::W) {
            self.amount_forward = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::W) {
            self.amount_forward = 0.0;
        }

        if inputs.is_key_just_pressed(&VirtualKeyCode::S) {
            self.amount_backward = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::S) {
            self.amount_backward = 0.0;
        }

        if inputs.is_key_just_pressed(&VirtualKeyCode::A) {
            self.amount_left = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::A) {
            self.amount_left = 0.0;
        }

        if inputs.is_key_just_pressed(&VirtualKeyCode::D) {
            self.amount_right = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::D) {
            self.amount_right = 0.0;
        }

        if inputs.is_key_just_pressed(&VirtualKeyCode::Space) {
            self.amount_up = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::Space) {
            self.amount_up = 0.0;
        }

        if inputs.is_key_just_pressed(&VirtualKeyCode::LShift) {
            self.amount_down = 1.0;
        } else if inputs.is_key_just_released(&VirtualKeyCode::LShift) {
            self.amount_down = 0.0;
        }
    }

    pub fn process_mouse(&mut self, inputs: &InputState) {
        let delta = inputs.mouse_delta();
        self.rotate_horizontal = delta.0;
        self.rotate_vertical = delta.1;
    }

    pub fn process_scroll(&mut self, inputs: &InputState) {
        self.scroll = inputs.mouse_scroll();
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration, inputs: &InputState) {
        self.process_keyboard(inputs);
        self.process_mouse(inputs);
        self.process_scroll(inputs);

        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vec3A::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vec3A::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward =
            Vec3A::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        if self.samples > 0 {
            camera.yaw += self.rotate_horizontal / self.samples as f32 * self.sensitivity * dt;
            camera.pitch += -self.rotate_vertical / self.samples as f32 * self.sensitivity * dt;

            // If process_mouse isn't called every frame, these values
            // will not get set to zero, and the camera will rotate
            // when moving in a non cardinal direction.
            self.rotate_horizontal = 0.0;
            self.rotate_vertical = 0.0;
            self.samples = 0;

            // Keep the camera's angle from going too high/low.
            if camera.pitch < -SAFE_FRAC_PI_2 {
                camera.pitch = -SAFE_FRAC_PI_2;
            } else if camera.pitch > SAFE_FRAC_PI_2 {
                camera.pitch = SAFE_FRAC_PI_2;
            }
        }
    }
}
