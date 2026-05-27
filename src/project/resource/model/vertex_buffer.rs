use egui_dnd::utils::shift_vec;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VertexBufferSpec {
    pub fields: Vec<VertexBufferField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Display, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    pub(super) fn compute_vertex_contents(
        &self,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        texture_coords: &[[f32; 2]],
        tangents: &[[f32; 3]],
        bitangents: &[[f32; 3]],
    ) -> Vec<f32> {
        let vertex_count = positions.len();

        let stride: usize = self
            .fields
            .iter()
            .map(|f| f.vertex_format().size() as usize / std::mem::size_of::<f32>())
            .sum();

        let mut result = Vec::with_capacity(vertex_count * stride);

        for i in 0..vertex_count {
            let p = positions.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let n = normals.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let uv = texture_coords.get(i).unwrap_or(&[0.0, 0.0]);
            let t = tangents.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let b = bitangents.get(i).unwrap_or(&[0.0, 0.0, 0.0]);

            for f in &self.fields {
                let value: &[f32] = match f {
                    VertexBufferField::Position => p,
                    VertexBufferField::TextureCoordinates => uv,
                    VertexBufferField::Normal => n,
                    VertexBufferField::Tangent => t,
                    VertexBufferField::Bitangent => b,
                };
                result.extend_from_slice(value);
            }
        }

        result
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
