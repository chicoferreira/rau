use std::f32::consts::FRAC_PI_2;

use derive_more::{Add, AddAssign, Deref};
use glam::{Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    project::{
        CameraId, Creatable, DimensionId, ProjectResource,
        resource::dimension::Dimension,
        storage::Storage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::key::{Key, KeyboardState},
};

pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 1.0),
);

const MIN_ZNEAR: f32 = 0.0001;
const Z_CLIP_GAP: f32 = 0.001;
const MIN_FOVY: Deg = Deg(1.0);
const MAX_FOVY: Deg = Deg(179.0);
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - MIN_ZNEAR;

#[derive(Debug, Clone, Copy, PartialEq, Add, AddAssign, Deref, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rad(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Add, AddAssign, Deref, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Deg(pub f32);

impl From<Deg> for Rad {
    fn from(degrees: Deg) -> Self {
        Rad(degrees.0.to_radians())
    }
}

impl From<Rad> for Deg {
    fn from(radians: Rad) -> Self {
        Deg(radians.0.to_degrees())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deref, Add, AddAssign, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Yaw(Rad);

#[derive(Debug, Clone, Copy, PartialEq, Deref, Add, AddAssign, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pitch(Rad);

#[derive(Debug, Clone, Copy, PartialEq, Deref, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fov(Rad);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClipRange {
    znear: f32,
    zfar: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deref, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PositiveF32(f32);

impl Yaw {
    pub fn new(yaw: impl Into<Rad>) -> Self {
        use std::f32::consts::PI;

        let Rad(yaw) = yaw.into();
        Self(Rad((yaw + PI).rem_euclid(2.0 * PI) - PI))
    }
}

impl Pitch {
    pub fn new(pitch: impl Into<Rad>) -> Self {
        let Rad(pitch) = pitch.into();
        Self(Rad(pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2)))
    }
}

impl Fov {
    pub fn new(fov: impl Into<Rad>) -> Self {
        let Rad(fov) = fov.into();
        Self(Rad(fov.clamp(Rad::from(MIN_FOVY).0, Rad::from(MAX_FOVY).0)))
    }
}

impl ClipRange {
    pub fn new(znear: f32, zfar: f32) -> Self {
        let znear = znear.max(MIN_ZNEAR);
        let zfar = zfar.max(znear + Z_CLIP_GAP);

        Self { znear, zfar }
    }

    pub fn with_znear(self, znear: f32) -> Self {
        Self::new(znear, self.zfar)
    }

    pub fn with_zfar(self, zfar: f32) -> Self {
        Self::new(self.znear, zfar)
    }

    pub fn znear(self) -> f32 {
        self.znear
    }

    pub fn zfar(self) -> f32 {
        self.zfar
    }
}

impl PositiveF32 {
    pub fn new(value: f32) -> Self {
        Self(value.max(0.0))
    }
}

pub struct CameraCreationContext<'a> {
    pub dimensions: &'a Storage<Dimension>,
    pub dt: instant::Duration,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Camera {
    label: String,
    position: Vec3,
    yaw: Yaw,
    pitch: Pitch,
    dimension_id: Option<DimensionId>,
    fovy: Fov,
    #[serde(flatten)]
    clip: ClipRange,
    current_speed: Vec3,
    max_speed: PositiveF32,
    acceleration: PositiveF32,
    drag: PositiveF32,
    sensitivity: PositiveF32,
    scroll_speed: PositiveF32,
    #[serde(skip)]
    input: CameraFrameInput,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, PartialEq)]
pub struct CameraRuntime {
    aspect: f32,
    matrix: CameraMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraMatrix {
    pub projection: Mat4,
    pub view: Mat4,
    pub projection_view: Mat4,
    pub inv_proj: Mat4,
    pub inverse_view: Mat4,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
        let position = Vec3::new(0.0, 0.0, -1.0);
        let pitch = Pitch::new(Rad(0.0));
        let yaw = Yaw::new(Rad(0.0));
        let fovy = Fov::new(Deg(60.0));
        let clip = ClipRange::new(0.1, 100.0);

        Self {
            label,
            position,
            pitch,
            yaw,
            dimension_id: None,
            fovy,
            max_speed: PositiveF32::new(20.0),
            clip,
            acceleration: PositiveF32::new(150.0),
            drag: PositiveF32::new(12.0),
            current_speed: Vec3::ZERO,
            sensitivity: PositiveF32::new(0.1),
            scroll_speed: PositiveF32::new(0.05),
            input: CameraFrameInput::default(),
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    pub fn input_mut(&mut self) -> &mut CameraFrameInput {
        &mut self.input
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn position() -> Vec3;
        pub fn current_speed() -> Vec3;
        pub fn yaw() -> Yaw;
        pub fn pitch() -> Pitch;
        pub fn fovy() -> Fov;
        pub fn clip() -> ClipRange;
        pub fn max_speed() -> PositiveF32;
        pub fn acceleration() -> PositiveF32;
        pub fn sensitivity() -> PositiveF32;
        pub fn scroll_speed() -> PositiveF32;
        pub fn dimension_id() -> Option<DimensionId>;
        pub fn drag() -> PositiveF32;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_label(label: String);
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_position(position: Vec3);
        pub fn set_dimension_id(dimension_id: Option<DimensionId>);
        pub fn set_yaw(yaw: Yaw);
        pub fn set_pitch(pitch: Pitch);
        pub fn set_fovy(fovy: Fov);
        pub fn set_clip(clip: ClipRange);
        pub fn set_max_speed(max_speed: PositiveF32);
        pub fn set_acceleration(acceleration: PositiveF32);
        pub fn set_drag_factor(drag: PositiveF32);
        pub fn set_sensitivity(sensitivity: PositiveF32);
        pub fn set_scroll_speed(scroll_speed: PositiveF32);
    }

    fn calculate_aspect(&self, dimensions: &Storage<Dimension>) -> AppResult<f32> {
        let dimension_id = self
            .dimension_id
            .ok_or(AppError::uninit_field("Dimension"))?;

        let dimension = dimensions.get(dimension_id)?;
        Ok(dimension.size().width() as f32 / dimension.size().height() as f32)
    }

    pub fn update(&mut self, dt: instant::Duration) {
        let dt = dt.as_secs_f32();
        let previous_position = self.position;
        let previous_yaw = self.yaw;
        let previous_pitch = self.pitch;
        let previous_current_speed = self.current_speed;

        let (yaw_sin, yaw_cos) = self.yaw.0.0.sin_cos();
        let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        let amount_forward = self.input.forward - self.input.backward;
        let amount_right = self.input.right - self.input.left;
        let amount_up = self.input.up - self.input.down;

        self.current_speed += forward * amount_forward * *self.acceleration * dt;
        self.current_speed += right * amount_right * *self.acceleration * dt;
        self.current_speed += Vec3::Y * amount_up * *self.acceleration * dt;

        self.current_speed -= self.current_speed * *self.drag * dt;

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

        let speed = self.current_speed.length();
        if speed > *self.max_speed {
            self.current_speed *= *self.max_speed / speed;
        }

        self.position += self.current_speed * dt;

        let (pitch_sin, pitch_cos) = (**self.pitch).sin_cos();
        let front = Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        self.position += front * self.input.scroll * *self.scroll_speed;

        if self.input.mouse_h != 0.0 {
            self.yaw += Yaw::new(Deg(self.input.mouse_h * *self.sensitivity));
        }
        if self.input.mouse_v != 0.0 {
            self.pitch += Pitch::new(Deg(-self.input.mouse_v * *self.sensitivity));
        }

        self.input.scroll = 0.0;
        self.input.mouse_h = 0.0;
        self.input.mouse_v = 0.0;

        if self.position != previous_position
            || self.yaw != previous_yaw
            || self.pitch != previous_pitch
            || self.current_speed != previous_current_speed
        {
            self.runtime_revision.increase();
        }
    }
}

impl CameraRuntime {
    pub fn matrix(&self) -> &CameraMatrix {
        &self.matrix
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }
}

impl Creatable for Camera {
    fn create(label: String) -> Self {
        Self::new(label)
    }
}

impl ProjectResource for Camera {
    type Id = CameraId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for Camera {
    type Context<'a> = CameraCreationContext<'a>;
    type Runtime = CameraRuntime;
    type Job = ();

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        _id: Self::Id,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
        _job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        let aspect = self.calculate_aspect(ctx.dimensions)?;

        let new_matrix = CameraMatrix::new(
            self.position,
            self.yaw,
            self.pitch,
            self.fovy,
            aspect,
            self.clip,
        );

        let new_runtime = CameraRuntime {
            aspect,
            matrix: new_matrix,
        };

        match previous {
            Some(runtime) if runtime == new_runtime => Ok(SyncOutcome::Unchanged(runtime)),
            _ => Ok(SyncOutcome::Changed(new_runtime)),
        }
    }

    fn needs_rebuild(&self, _: Self::Id, _: &Self::Context<'_>, tracker: &SyncTracker) -> bool {
        self.dimension_id
            .map_or(false, |id| tracker.was_changed(id))
            || self.current_speed.length_squared() > 0.0
            || self.input != CameraFrameInput::default()
    }
}

impl CameraMatrix {
    pub fn new(
        position: Vec3,
        Yaw(Rad(yaw)): Yaw,
        Pitch(Rad(pitch)): Pitch,
        Fov(Rad(fovy)): Fov,
        aspect: f32,
        ClipRange { zfar, znear }: ClipRange,
    ) -> Self {
        let projection = OPENGL_TO_WGPU_MATRIX * Mat4::perspective_rh_gl(fovy, aspect, znear, zfar);
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let (sin_yaw, cos_yaw) = yaw.sin_cos();

        let view_direction =
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();
        let view = Mat4::look_at_rh(position, position + view_direction, Vec3::Y);

        let projection_view = projection * view;
        let inv_proj = projection.inverse();
        let inverse_view = view.inverse();

        Self {
            projection,
            view,
            projection_view,
            inv_proj,
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
