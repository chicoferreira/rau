/// Settings the window surface is created with.
#[derive(Default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(clap::Args))]
pub struct RenderSettings {
    /// Graphics backend to render with. Defaults to whichever the platform prefers.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long, value_enum, global = true))]
    pub backend: Option<Backend>,

    /// Present mode at startup.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long, value_enum, global = true))]
    pub present_mode: Option<PresentMode>,
}

/// Which graphics API the WGPU instance is created against.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_arch = "wasm32"), derive(clap::ValueEnum))]
#[cfg_attr(
    target_arch = "wasm32",
    derive(serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum Backend {
    #[cfg(not(target_arch = "wasm32"))]
    Vulkan,
    #[cfg(not(target_arch = "wasm32"))]
    Dx12,
    #[cfg(not(target_arch = "wasm32"))]
    Metal,
    /// Falls back to OpenGL ES where desktop OpenGL is unavailable.
    #[cfg(not(target_arch = "wasm32"))]
    Gl,
    #[cfg(target_arch = "wasm32")]
    Webgpu,
    #[cfg(target_arch = "wasm32")]
    Webgl2,
}

pub const DEFAULT_BACKENDS: wgpu::Backends = cfg_select! {
    target_arch = "wasm32" => wgpu::Backends::BROWSER_WEBGPU.union(wgpu::Backends::GL),
    _ => wgpu::Backends::PRIMARY,
};

impl From<Backend> for wgpu::Backends {
    fn from(backend: Backend) -> Self {
        match backend {
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Vulkan => Self::VULKAN,
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Dx12 => Self::DX12,
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Metal => Self::METAL,
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Gl => Self::GL,
            #[cfg(target_arch = "wasm32")]
            Backend::Webgpu => Self::BROWSER_WEBGPU,
            #[cfg(target_arch = "wasm32")]
            Backend::Webgl2 => Self::GL,
        }
    }
}

/// How rendered frames are presented to the window surface.
///
/// Mirrors [`wgpu::PresentMode`] but implements [`clap::ValueEnum`].
#[derive(Clone, Copy)]
#[cfg_attr(not(target_arch = "wasm32"), derive(clap::ValueEnum))]
pub enum PresentMode {
    /// `fifo-relaxed` if supported, otherwise `fifo`.
    AutoVsync,
    /// `immediate` if supported, otherwise `mailbox`, otherwise `fifo`.
    AutoNoVsync,
    /// Present one queued frame per refresh, waiting when the queue is full. Never tears.
    Fifo,
    /// Like `fifo`, but a late frame is presented immediately, which tears.
    FifoRelaxed,
    /// Present every frame immediately, which tears.
    Immediate,
    /// Like `fifo`, but a new frame replaces the queued one instead of waiting. Never tears.
    Mailbox,
}

impl From<PresentMode> for wgpu::PresentMode {
    fn from(present_mode: PresentMode) -> Self {
        match present_mode {
            PresentMode::AutoVsync => Self::AutoVsync,
            PresentMode::AutoNoVsync => Self::AutoNoVsync,
            PresentMode::Fifo => Self::Fifo,
            PresentMode::FifoRelaxed => Self::FifoRelaxed,
            PresentMode::Immediate => Self::Immediate,
            PresentMode::Mailbox => Self::Mailbox,
        }
    }
}
