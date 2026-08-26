/// Settings the window surface is created with.
#[derive(Default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(clap::Args))]
pub struct RenderSettings {
    /// Present mode at startup.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long, value_enum, global = true))]
    pub present_mode: Option<PresentMode>,
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
