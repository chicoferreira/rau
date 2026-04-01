pub struct VertexBufferSpec {
    pub fields: Vec<VertexBufferField>,
}

pub enum VertexBufferField {
    Position,
    TextureCoordinates,
    Normal,
    Tangent,
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
