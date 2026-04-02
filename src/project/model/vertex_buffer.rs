use egui_dnd::utils::shift_vec;

#[derive(Clone)]
pub struct VertexBufferSpec {
    pub fields: Vec<VertexBufferField>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display,
)]
pub enum VertexBufferField {
    #[strum(to_string = "Position")]
    Position,
    #[strum(to_string = "Texture Coordinates")]
    TextureCoordinates,
    #[strum(to_string = "Normal")]
    Normal,
    #[strum(to_string = "Tangent")]
    Tangent,
    #[strum(to_string = "Bitangent")]
    Bitangent,
}

impl VertexBufferSpec {
    pub fn new() -> Self {
        Self {
            fields: vec![
                VertexBufferField::Position,
                VertexBufferField::TextureCoordinates,
                VertexBufferField::Normal,
                VertexBufferField::Tangent,
                VertexBufferField::Bitangent,
            ],
        }
    }

    pub fn to_wgpu_attributes_and_stride(&self) -> (Vec<wgpu::VertexAttribute>, u64) {
        let mut attributes = vec![];
        let mut offset = 0;

        for (i, f) in self.fields.iter().enumerate() {
            attributes.push(wgpu::VertexAttribute {
                shader_location: i as u32,
                offset,
                format: f.vertex_format(),
            });
            offset += f.vertex_format().size();
        }

        (attributes, offset)
    }

    pub(super) fn reorder_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.fields);
    }
}

impl VertexBufferField {
    pub fn vertex_format(&self) -> wgpu::VertexFormat {
        match self {
            VertexBufferField::Position => wgpu::VertexFormat::Float32x3,
            VertexBufferField::TextureCoordinates => wgpu::VertexFormat::Float32x2,
            VertexBufferField::Normal => wgpu::VertexFormat::Float32x3,
            VertexBufferField::Tangent => wgpu::VertexFormat::Float32x3,
            VertexBufferField::Bitangent => wgpu::VertexFormat::Float32x3,
        }
    }
}
