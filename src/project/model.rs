use std::{io::BufReader, path::Path};

use crate::{
    error::{AppError, AppResult},
    resources::load_binary,
};

pub struct Model {
    pub label: String,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub texture_coords: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub material_id: Option<usize>,
}

pub struct Material {
    pub label: String,
    pub texture_paths: Vec<String>,
}

impl Model {
    pub async fn load_from_obj_file(label: String, file: impl AsRef<Path>) -> AppResult<Self> {
        let obj_bytes = load_binary(file).await.map_err(AppError::FileLoadError)?;

        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (models, obj_materials) =
            tobj::futures::load_obj_buf(obj_bytes.as_ref(), &load_options, |p| async move {
                let mat = load_binary(&p).await;
                let mat = mat.map_err(|_| tobj::LoadError::OpenFileFailed)?;
                let mat: &[u8] = mat.as_ref();
                let mut reader = BufReader::new(mat);
                tobj::load_mtl_buf(&mut reader)
            })
            .await?;

        let meshes = models.into_iter().map(Into::into).collect();
        let materials = obj_materials?.into_iter().map(Into::into).collect();

        Ok(Model {
            label,
            meshes,
            materials,
        })
    }
}

impl From<tobj::Material> for Material {
    fn from(material: tobj::Material) -> Self {
        let label = material.name;
        let texture_paths = [
            material.ambient_texture,
            material.diffuse_texture,
            material.normal_texture,
            material.specular_texture,
            material.shininess_texture,
            material.dissolve_texture,
        ]
        .into_iter()
        .filter_map(|tex| tex)
        .collect();

        Material {
            label,
            texture_paths,
        }
    }
}

impl From<tobj::Model> for Mesh {
    fn from(model: tobj::Model) -> Self {
        let (positions, _) = model.mesh.positions.as_chunks();
        let (normals, _) = model.mesh.normals.as_chunks();
        let (texture_coords, _) = model.mesh.texcoords.as_chunks();

        Mesh {
            positions: positions.to_vec(),
            normals: normals.to_vec(),
            texture_coords: texture_coords.to_vec(),
            indices: model.mesh.indices,
            material_id: model.mesh.material_id,
        }
    }
}
