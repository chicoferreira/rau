use std::f32::consts::FRAC_PI_2;

use cgmath::{Deg, InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Vector3, Zero};

use crate::{
    error::AppResult,
    key::{Key, KeyboardState},
    project::{
        CameraId, DimensionId, ProjectResource,
        dimension::Dimension,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
        storage::Storage,
    },
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct CameraCreationContext<'a> {
    pub dimensions: &'a Storage<DimensionId, Dimension>,
    pub dt: instant::Duration,
}

#[derive(Debug)]
pub struct Camera {
    pub label: String,
    position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    dimension_id: Option<DimensionId>,
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
    current_speed: Vector3<f32>,
    max_speed: f32,
    acceleration: f32,
    drag: f32,
    sensitivity: f32,
    scroll_speed: f32,
    matrix: CameraMatrix,
    input: CameraFrameInput,
    dirty: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraMatrix {
    pub projection: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub projection_view: Matrix4<f32>,
    pub inverse_projection: Matrix4<f32>,
    pub inverse_view: Matrix4<f32>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CameraFrameInput {
    pub left: f32,
    pub right: f32,
    pub forward: f32,
    pub backward: f32,
    pub up: f32,
    pub down: f32,
    pub mouse_h: f32,
    pub mouse_v: f32,
    pub scroll: f32,
}

impl Camera {
    pub fn new(label: String) -> Self {
        let position = (0.0, 0.0, -1.0).into();
        let pitch = Deg(0.0).into();
        let yaw = Deg(0.0).into();
        let fovy = Deg(60.0).into();
        let znear = 0.1;
        let zfar = 100.0;

        // this will be updated when the camera is attached to a dimension
        let aspect = 1.0;

        let matrix = CameraMatrix::new(position, yaw, pitch, fovy, aspect, znear, zfar);
        let input = CameraFrameInput::default();

        Self {
            label,
            position,
            pitch,
            yaw,
            dimension_id: None,
            aspect,
            fovy,
            max_speed: 20.0,
            znear,
            zfar,
            acceleration: 150.0,
            drag: 12.0,
            current_speed: Vector3::zero(),
            sensitivity: 0.1,
            scroll_speed: 0.05,
            matrix,
            input,
            dirty: false,
        }
    }

    pub fn input_mut(&mut self) -> &mut CameraFrameInput {
        &mut self.input
    }

    pub fn position(&self) -> Point3<f32> {
        self.position
    }

    pub fn matrix(&self) -> &CameraMatrix {
        &self.matrix
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn current_speed(&self) -> Vector3<f32> {
        self.current_speed
    }

    pub fn yaw(&self) -> Rad<f32> {
        self.yaw
    }

    pub fn pitch(&self) -> Rad<f32> {
        self.pitch
    }

    pub fn fovy(&self) -> Rad<f32> {
        self.fovy
    }

    pub fn znear(&self) -> f32 {
        self.znear
    }

    pub fn zfar(&self) -> f32 {
        self.zfar
    }

    pub fn max_speed(&self) -> f32 {
        self.max_speed
    }

    pub fn acceleration(&self) -> f32 {
        self.acceleration
    }

    pub fn drag_factor(&self) -> f32 {
        self.drag
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn scroll_speed(&self) -> f32 {
        self.scroll_speed
    }

    pub fn dimension_id(&self) -> Option<DimensionId> {
        self.dimension_id
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn set_position(&mut self, position: impl Into<Point3<f32>>) {
        self.position = position.into();
        self.mark_dirty();
    }

    pub fn set_yaw(&mut self, yaw: impl Into<Rad<f32>>) {
        use std::f32::consts::PI;
        let rad: Rad<f32> = yaw.into();
        self.yaw = Rad((rad.0 + PI).rem_euclid(2.0 * PI) - PI);
        self.mark_dirty();
    }

    pub fn set_pitch(&mut self, pitch: impl Into<Rad<f32>>) {
        let rad: Rad<f32> = pitch.into();
        self.pitch = Rad(rad.0.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2));
        self.mark_dirty();
    }

    pub fn set_fovy(&mut self, fovy: impl Into<Rad<f32>>) {
        let rad: Rad<f32> = fovy.into();
        self.fovy = Rad(rad
            .0
            .clamp(Rad::from(Deg(1.0_f32)).0, Rad::from(Deg(179.0_f32)).0));
        self.mark_dirty();
    }

    pub fn set_znear(&mut self, znear: f32) {
        self.znear = znear.max(0.0001);
        self.mark_dirty();
    }

    pub fn set_zfar(&mut self, zfar: f32) {
        self.zfar = zfar.max(self.znear + 0.001);
        self.mark_dirty();
    }

    pub fn set_max_speed(&mut self, max_speed: f32) {
        self.max_speed = max_speed.max(0.0);
        self.mark_dirty();
    }

    pub fn set_acceleration(&mut self, acceleration: f32) {
        self.acceleration = acceleration.max(0.0);
        self.mark_dirty();
    }

    pub fn set_drag_factor(&mut self, drag: f32) {
        self.drag = drag.max(0.0);
        self.mark_dirty();
    }

    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.max(0.0);
        self.mark_dirty();
    }

    pub fn set_scroll_speed(&mut self, scroll_speed: f32) {
        self.scroll_speed = scroll_speed.max(0.0);
        self.mark_dirty();
    }

    pub fn set_dimension_id(&mut self, dimension_id: Option<DimensionId>) {
        self.dimension_id = dimension_id;
        self.mark_dirty();
    }

    fn update(&mut self, dt: instant::Duration) {
        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = self.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        let amount_forward = self.input.forward - self.input.backward;
        let amount_right = self.input.right - self.input.left;
        let amount_up = self.input.up - self.input.down;

        self.current_speed += forward * amount_forward * self.acceleration * dt;
        self.current_speed += right * amount_right * self.acceleration * dt;
        self.current_speed += Vector3::unit_y() * amount_up * self.acceleration * dt;

        self.current_speed -= self.current_speed * self.drag * dt;

        const SPEED_EPSILON: f32 = 0.0005;
        if self.current_speed.x.abs() < SPEED_EPSILON {
            self.current_speed.x = 0.0;
        }
        if self.current_speed.y.abs() < SPEED_EPSILON {
            self.current_speed.y = 0.0;
        }
        if self.current_speed.z.abs() < SPEED_EPSILON {
            self.current_speed.z = 0.0;
        }

        let speed = self.current_speed.magnitude();
        if speed > self.max_speed {
            self.current_speed *= self.max_speed / speed;
        }

        self.position += self.current_speed * dt;

        let (pitch_sin, pitch_cos) = self.pitch.0.sin_cos();
        let front = Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.position += front * self.input.scroll * self.scroll_speed;

        if self.input.mouse_h != 0.0 {
            self.set_yaw(self.yaw + Rad::from(Deg(self.input.mouse_h * self.sensitivity)));
        }
        if self.input.mouse_v != 0.0 {
            self.set_pitch(self.pitch + Rad::from(Deg(-self.input.mouse_v * self.sensitivity)));
        }

        self.input.scroll = 0.0;
        self.input.mouse_h = 0.0;
        self.input.mouse_v = 0.0;
    }
}

impl ProjectResource for Camera {
    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Camera {
    type Context<'a> = CameraCreationContext<'a>;
    type Id = CameraId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        let mut event = None;

        let (position, yaw, pitch) = (self.position, self.yaw, self.pitch);
        self.update(ctx.dt);
        if self.position != position || self.yaw != yaw || self.pitch != pitch {
            event = Some(ProjectEvent::CameraUpdated(id));
        }

        if self.dirty {
            self.dirty = false;
            event = Some(ProjectEvent::CameraUpdated(id));
        }

        let new_aspect = if let Some(dimension_id) = self.dimension_id
            && let Ok(dimension) = ctx.dimensions.get(dimension_id)
        {
            dimension.size.width() as f32 / dimension.size.height() as f32
        } else {
            1.0
        };

        if self.aspect != new_aspect {
            self.aspect = new_aspect;
            event = Some(ProjectEvent::CameraUpdated(id));
        }

        if event.is_some() {
            self.matrix = CameraMatrix::new(
                self.position,
                self.yaw,
                self.pitch,
                self.fovy,
                self.aspect,
                self.znear,
                self.zfar,
            );
        }

        Ok(event)
    }
}

impl CameraMatrix {
    pub fn new(
        position: Point3<f32>,
        Rad(yaw): Rad<f32>,
        Rad(pitch): Rad<f32>,
        fovy: Rad<f32>,
        aspect: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        let projection = OPENGL_TO_WGPU_MATRIX * cgmath::perspective(fovy, aspect, znear, zfar);
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let (sin_yaw, cos_yaw) = yaw.sin_cos();

        let view = Matrix4::look_to_rh(
            position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        );

        let projection_view = projection * view;
        let inverse_projection = projection.invert().unwrap().into();
        let inverse_view = view.invert().unwrap().into();

        Self {
            projection,
            view,
            projection_view,
            inverse_projection,
            inverse_view,
        }
    }
}

impl CameraFrameInput {
    pub fn handle_keyboard(&mut self, keyboard: KeyboardState) {
        macro_rules! handle_keys {
            ($kb:expr, $( $field:ident => $($key:path),+ );+ $(;)?) => {
                $(
                    self.$field = if $($kb.is_pressed($key))||+ { 1.0 } else { 0.0 };
                )+
            };
        }

        handle_keys!(keyboard,
            forward  => Key::W, Key::ArrowUp;
            backward => Key::S, Key::ArrowDown;
            left     => Key::A, Key::ArrowLeft;
            right    => Key::D, Key::ArrowRight;
            up       => Key::Space;
            down     => Key::Shift;
        );
    }

    pub fn handle_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.mouse_h = mouse_dx;
        self.mouse_v = mouse_dy;
    }

    pub fn handle_scroll_pixels(&mut self, scroll_pixels: f32) {
        self.scroll = scroll_pixels;
    }
}
