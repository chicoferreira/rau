use futures_lite::io::{BufReader, Cursor};

use crate::{
    error::AppResult,
    file::file_system::{ProjectFileSystem, ProjectFileSystemTrait},
    project::paths::FilePath,
    utils::background_task,
};

pub struct LoadedObj {
    pub models: Vec<tobj::Model>,
    pub materials: Vec<tobj::Material>,
    pub mtl_dependencies: Vec<FilePath>,
}

pub async fn load_obj(obj_path: FilePath, file_system: ProjectFileSystem) -> AppResult<LoadedObj> {
    background_task::spawn_future("obj-loader", async move {
        let obj_bytes = file_system.read(&obj_path).await?;

        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (mtl_tx, mtl_rx) = std::sync::mpsc::channel();

        let obj_reader = BufReader::new(Cursor::new(obj_bytes));
        let (models, materials) =
            tobj::futures::load_obj_buf(obj_reader, &load_options, move |material_path| {
                let obj_path = obj_path.clone();
                let file_system = file_system.clone();
                let mtl_tx = mtl_tx.clone();

                async move {
                    let relative_path = FilePath::from_relative_path(&material_path)
                        .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                    let material_path = obj_path
                        .parent()
                        .map(|parent| parent.join_path(&relative_path))
                        .unwrap_or(relative_path);

                    let _ = mtl_tx.send(material_path.clone());

                    let mtl_bytes = file_system
                        .read(&material_path)
                        .await
                        .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                    let mtl_reader = BufReader::new(Cursor::new(mtl_bytes));

                    tobj::futures::load_mtl_buf(mtl_reader).await
                }
            })
            .await?;

        Ok(LoadedObj {
            models,
            materials: materials?,
            mtl_dependencies: mtl_rx.try_iter().collect(),
        })
    })
    .await
}

pub fn calculate_tangents_and_bitangents(
    positions: &[[f32; 3]],
    texture_coords: &[[f32; 2]],
    indices: &[u32],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    use glam::{Vec2, Vec3};

    let vertex_count = positions.len();

    let mut tangents = vec![[0.0f32; 3]; vertex_count];
    let mut bitangents = vec![[0.0f32; 3]; vertex_count];
    let mut triangles_included = vec![0u32; vertex_count];

    let (triangles, _) = indices.as_chunks();
    for [i0, i1, i2] in triangles {
        let (i0, i1, i2) = (*i0 as usize, *i1 as usize, *i2 as usize);
        if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
            continue;
        }

        let p0 = Vec3::from_array(positions.get(i0).copied().unwrap_or([0.0, 0.0, 0.0]));
        let p1 = Vec3::from_array(positions.get(i1).copied().unwrap_or([0.0, 0.0, 0.0]));
        let p2 = Vec3::from_array(positions.get(i2).copied().unwrap_or([0.0, 0.0, 0.0]));
        let uv0 = Vec2::from_array(texture_coords.get(i0).copied().unwrap_or([0.0, 0.0]));
        let uv1 = Vec2::from_array(texture_coords.get(i1).copied().unwrap_or([0.0, 0.0]));
        let uv2 = Vec2::from_array(texture_coords.get(i2).copied().unwrap_or([0.0, 0.0]));

        let dp1 = p1 - p0;
        let dp2 = p2 - p0;
        let duv1 = uv1 - uv0;
        let duv2 = uv2 - uv0;

        let denom = duv1.x * duv2.y - duv1.y * duv2.x;
        if denom.abs() < 1.0e-8 {
            continue;
        }
        let r = 1.0 / denom;

        let tangent = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let bitangent = (dp2 * duv1.x - dp1 * duv2.x) * r;

        tangents[i0] = (Vec3::from_array(tangents[i0]) + tangent).to_array();
        tangents[i1] = (Vec3::from_array(tangents[i1]) + tangent).to_array();
        tangents[i2] = (Vec3::from_array(tangents[i2]) + tangent).to_array();

        bitangents[i0] = (Vec3::from_array(bitangents[i0]) + bitangent).to_array();
        bitangents[i1] = (Vec3::from_array(bitangents[i1]) + bitangent).to_array();
        bitangents[i2] = (Vec3::from_array(bitangents[i2]) + bitangent).to_array();

        triangles_included[i0] += 1;
        triangles_included[i1] += 1;
        triangles_included[i2] += 1;
    }

    for i in 0..vertex_count {
        let n = triangles_included[i];
        if n <= 0 {
            continue;
        }
        let denom = 1.0 / n as f32;

        let t = Vec3::from_array(tangents[i]) * denom;
        let b = Vec3::from_array(bitangents[i]) * denom;

        tangents[i] = if t.length_squared() > 0.0 {
            t.normalize().to_array()
        } else {
            [0.0, 0.0, 0.0]
        };
        bitangents[i] = if b.length_squared() > 0.0 {
            b.normalize().to_array()
        } else {
            [0.0, 0.0, 0.0]
        };
    }

    (tangents, bitangents)
}
