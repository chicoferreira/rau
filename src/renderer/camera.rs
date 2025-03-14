use crate::renderer::uniform::{GpuUniform, GpuUniformAcessor};
use cgmath::{InnerSpace, Matrix4, Point3, Rad, Vector3, Zero};
use enum2egui::GuiInspect;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::event::*;
use winit::keyboard::KeyCode;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_position: camera.position.to_homogeneous().into(),
            view_proj: camera.calc_matrix().into(),
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct Camera {
    position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    up: Vector3<f32>,
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
    sensitivity: f32,
    speed: Vector3<f32>,
    max_speed_per_second: f32,
    acceleration_per_second: f32,
    friction_per_second: f32,
    input: CameraInput,
    gpu_uniform: GpuUniform<CameraUniform>,
}

#[derive(Debug, Default)]
struct CameraInput {
    foward_input: f32,
    back_input: f32,
    left_input: f32,
    right_input: f32,
    up_input: f32,
    down_input: f32,
    offset_input: (f32, f32),
}

impl Camera {
    pub fn new<P: Into<Point3<f32>>, F: Into<Rad<f32>>, Y: Into<Rad<f32>>, W: Into<Rad<f32>>>(
        position: P,
        yaw: Y,
        pitch: W,
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
        uniform_device: &wgpu::Device,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
        uniform_binding: u32,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            up: Vector3::unit_y(),
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
            sensitivity: 0.5,
            speed: Vector3::zero(),
            max_speed_per_second: 10.0,
            acceleration_per_second: 100.0,
            friction_per_second: 20.0,
            input: CameraInput::default(),
            gpu_uniform: GpuUniform::new(
                uniform_device,
                CameraUniform::default(),
                uniform_bind_group_layout,
                uniform_binding,
                Some("Camera"),
            ),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let projection_matrix = OPENGL_TO_WGPU_MATRIX
            * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar);

        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        let dir = Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();

        projection_matrix * Matrix4::look_to_rh(self.position, dir, Vector3::unit_y())
    }

    pub fn update_camera(&mut self, duration: Duration) {
        // Handle movement
        let duration = duration.as_secs_f32();

        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();

        let front = Vector3::new(cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch).normalize();
        let right = front.cross(self.up).normalize();

        let x_input = self.input.right_input - self.input.left_input;
        let y_input = self.input.up_input - self.input.down_input;
        let z_input = self.input.foward_input - self.input.back_input;

        let move_dir = (front * z_input + right * x_input) + self.up * y_input;
        let acceleration = move_dir * self.acceleration_per_second * duration;
        self.speed += acceleration;

        if self.speed.magnitude() > self.max_speed_per_second {
            self.speed = self.speed.normalize_to(self.max_speed_per_second);
        }

        self.position += self.speed * duration;
        if x_input == 0.0 && y_input == 0.0 && z_input == 0.0 {
            let friction = self.speed * self.friction_per_second * duration;
            self.speed -= friction;
            if self.speed.magnitude() < 0.01 {
                self.speed = Vector3::zero();
            }
        }

        // Handle camera direction
        let (x_offset, y_offset) = self.input.offset_input;

        self.yaw += Rad(x_offset) * self.sensitivity * duration;
        self.pitch += Rad(-y_offset) * self.sensitivity * duration;

        if self.pitch < -Rad(SAFE_FRAC_PI_2) {
            self.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if self.pitch > Rad(SAFE_FRAC_PI_2) {
            self.pitch = Rad(SAFE_FRAC_PI_2);
        }

        // Reset input
        self.input.offset_input = (0.0, 0.0);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        #[rustfmt::skip]
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };

        macro_rules! handle_keys {
            ($self:ident, $key:ident, $( ($key_pat:pat, $component:ident) ),*) => {
                match $key {
                    $(
                        $key_pat => {
                            $self.input.$component = amount;
                            true
                        }
                    )*
                    _ => false,
                }
            };
        }
        handle_keys!(
            self,
            key,
            (KeyCode::KeyW | KeyCode::ArrowUp, foward_input),
            (KeyCode::KeyS | KeyCode::ArrowDown, back_input),
            (KeyCode::KeyA | KeyCode::ArrowLeft, left_input),
            (KeyCode::KeyD | KeyCode::ArrowRight, right_input),
            (KeyCode::Space, up_input),
            (KeyCode::ShiftLeft, down_input)
        )
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.input.offset_input.0 = mouse_dx as f32;
        self.input.offset_input.1 = mouse_dy as f32;
    }
}

impl GpuUniformAcessor for Camera {
    fn get_bind_group(&self) -> &wgpu::BindGroup {
        self.gpu_uniform.get_bind_group()
    }

    fn upload_gpu_uniform(&mut self, queue: &wgpu::Queue) {
        let camera_uniform = CameraUniform::from_camera(self);
        self.gpu_uniform.write_to_queue(queue, camera_uniform);
    }
}

impl GuiInspect for Camera {
    fn ui(&self, _ui: &mut egui::Ui) {
        unimplemented!();
    }

    fn ui_mut(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Position");
            ui.add(egui::widgets::DragValue::new(&mut self.position.x));
            ui.add(egui::widgets::DragValue::new(&mut self.position.y));
            ui.add(egui::widgets::DragValue::new(&mut self.position.z));
        });

        ui.horizontal(|ui| {
            ui.label("Speed");
            ui.add(egui::widgets::DragValue::new(&mut self.speed.x));
            ui.add(egui::widgets::DragValue::new(&mut self.speed.y));
            ui.add(egui::widgets::DragValue::new(&mut self.speed.z));
        });

        ui.horizontal(|ui| {
            ui.label("Yaw");
            ui.add(
                egui::widgets::DragValue::new(&mut self.yaw.0)
                    .suffix(" rad")
                    .speed(0.05),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Pitch");
            ui.add(
                egui::widgets::DragValue::new(&mut self.pitch.0)
                    .suffix(" rad")
                    .speed(0.05),
            );
        });

        fn show_degrees(value: &mut Rad<f32>) -> impl FnMut(Option<f64>) -> f64 {
            |v: Option<f64>| {
                if let Some(v) = v {
                    *value = Rad(v.to_radians() as f32);
                }
                value.0.to_degrees() as f64
            }
        }

        ui.horizontal(|ui| {
            ui.label("Fov");
            ui.add(egui::widgets::Slider::from_get_set(
                30.0..=150.0,
                show_degrees(&mut self.fovy),
            ));
        });

        ui.horizontal(|ui| {
            ui.label("Near");
            ui.add(egui::widgets::DragValue::new(&mut self.znear).range(0.1..=999.0));
        });

        ui.horizontal(|ui| {
            ui.label("Far");
            ui.add(egui::widgets::DragValue::new(&mut self.zfar).range(self.znear..=999.0));
        });
    }
}
