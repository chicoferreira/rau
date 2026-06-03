use super::ir::{ShaderBinding, ShaderItem, ShaderModule, ShaderStruct, ShaderType};

#[derive(Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::Display)]
pub enum Language {
    #[strum(to_string = "WGSL")]
    Wgsl,
}

impl Language {
    pub fn highlight_extension(self) -> &'static str {
        match self {
            Language::Wgsl => "wgsl",
        }
    }
}

pub trait ShaderBackend {
    fn format_type(&self, ty: &ShaderType) -> String;

    fn format_struct(&self, definition: &ShaderStruct) -> String;

    fn format_binding(&self, binding: &ShaderBinding) -> String;

    fn format_comment(&self, text: &str) -> String {
        format!("// {text}")
    }

    /// Structs are emitted first so the declarations that reference them are
    /// valid top-to-bottom. The ordering is language-independent, so backends
    /// rarely need to override this.
    fn render(&self, module: &ShaderModule) -> String {
        let mut structs = Vec::new();
        let mut rest = Vec::new();

        for item in module.items() {
            match item {
                ShaderItem::Struct(definition) => structs.push(self.format_struct(definition)),
                ShaderItem::Binding(binding) => rest.push(self.format_binding(binding)),
                ShaderItem::Comment(text) => rest.push(self.format_comment(text)),
            }
        }

        let mut blocks = Vec::new();
        if !structs.is_empty() {
            blocks.push(structs.join("\n\n"));
        }
        if !rest.is_empty() {
            blocks.push(rest.join("\n"));
        }
        blocks.join("\n\n")
    }
}
