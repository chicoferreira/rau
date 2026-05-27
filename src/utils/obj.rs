use futures_lite::io::{BufReader, Cursor};

use crate::{
    error::AppResult,
    file::file_system::{ProjectFileSystem, ProjectFileSystemTrait},
    project::paths::FilePath,
};

pub struct LoadedObj {
    pub models: Vec<tobj::Model>,
    pub materials: Vec<tobj::Material>,
}

pub async fn load_obj(obj_path: FilePath, file_system: ProjectFileSystem) -> AppResult<LoadedObj> {
    let obj_bytes = file_system.read(&obj_path).await?;

    let load_options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };

    let obj_reader = BufReader::new(Cursor::new(obj_bytes));
    let (models, materials) =
        tobj::futures::load_obj_buf(obj_reader, &load_options, move |material_path| {
            let material_path = material_path.to_string_lossy().to_string();
            let obj_path = obj_path.clone();
            let file_system = file_system.clone();

            async move {
                let relative_path = FilePath::from_str(&material_path)
                    .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                let material_path = obj_path
                    .parent()
                    .map(|parent| parent.join_path(&relative_path))
                    .unwrap_or(relative_path);

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
    })
}

pub fn calculate_tangents_and_bitangents(
    positions: &[[f32; 3]],
    texture_coords: &[[f32; 2]],
    indices: &[u32],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    use cgmath::{InnerSpace, Vector2, Vector3};

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

        let p0: Vector3<_> = positions.get(i0).copied().unwrap_or([0.0, 0.0, 0.0]).into();
        let p1: Vector3<_> = positions.get(i1).copied().unwrap_or([0.0, 0.0, 0.0]).into();
        let p2: Vector3<_> = positions.get(i2).copied().unwrap_or([0.0, 0.0, 0.0]).into();
        let uv0: Vector2<_> = texture_coords.get(i0).copied().unwrap_or([0.0, 0.0]).into();
        let uv1: Vector2<_> = texture_coords.get(i1).copied().unwrap_or([0.0, 0.0]).into();
        let uv2: Vector2<_> = texture_coords.get(i2).copied().unwrap_or([0.0, 0.0]).into();

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

        tangents[i0] = (Vector3::from(tangents[i0]) + tangent).into();
        tangents[i1] = (Vector3::from(tangents[i1]) + tangent).into();
        tangents[i2] = (Vector3::from(tangents[i2]) + tangent).into();

        bitangents[i0] = (Vector3::from(bitangents[i0]) + bitangent).into();
        bitangents[i1] = (Vector3::from(bitangents[i1]) + bitangent).into();
        bitangents[i2] = (Vector3::from(bitangents[i2]) + bitangent).into();

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

        let t = Vector3::from(tangents[i]) * denom;
        let b = Vector3::from(bitangents[i]) * denom;

        tangents[i] = if t.magnitude2() > 0.0 {
            t.normalize().into()
        } else {
            [0.0, 0.0, 0.0]
        };
        bitangents[i] = if b.magnitude2() > 0.0 {
            b.normalize().into()
        } else {
            [0.0, 0.0, 0.0]
        };
    }

    (tangents, bitangents)
}
