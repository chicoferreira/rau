pub mod backend;
mod contribute;
pub mod ir;
#[cfg(test)]
mod tests;
mod wgsl;

pub use backend::{Language, ShaderBackend};
pub use contribute::{BindGroupAt, ShaderGenCtx, ShaderInterface};
pub use wgsl::WgslBackend;

pub fn render(item: &impl ShaderInterface, ctx: &ShaderGenCtx, language: Language) -> String {
    let mut module = ir::ShaderModule::default();
    item.contribute(&mut module, ctx);
    match language {
        Language::Wgsl => WgslBackend.render(&module),
    }
}
