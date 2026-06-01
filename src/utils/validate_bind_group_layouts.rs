use crate::error::{AppError, AppResult};

/// Prevents wgpu from panicking when sparse bind group slots expand into more
/// pipeline layout entries than the device bind group limit allows.
///
/// Check https://github.com/chicoferreira/rau/issues/107 for more information
/// to why this is needed and we can't lean on wgpu scoped error handling.
pub fn validate_bind_group_layouts(
    bind_groups: &[Option<&wgpu::BindGroupLayout>],
    limits: &wgpu::Limits,
) -> AppResult<()> {
    let count = bind_groups.len();
    let max = limits.max_bind_groups as usize;

    if count > max {
        Err(AppError::BindGroupLayoutLimitExceeded { count, max })
    } else {
        Ok(())
    }
}
