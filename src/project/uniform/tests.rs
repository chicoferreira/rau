use super::*;

#[test]
fn cast_pads_vec2_to_vec4_alignment() {
    let fields = vec![
        UniformField::new(
            "uv",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec2f([1.0, 2.0])),
        ),
        UniformField::new(
            "tint",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec4f([3.0, 4.0, 5.0, 6.0])),
        ),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn cast_pads_vec2_to_rgb_alignment() {
    let fields = vec![
        UniformField::new(
            "uv",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec2f([1.5, 2.5])),
        ),
        UniformField::new(
            "color",
            UniformFieldSource::new_user_defined(UniformFieldData::Rgb([0.1, 0.2, 0.3])),
        ),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1.5, 2.5, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0]);
}

#[test]
fn cast_no_padding_between_vec3_and_vec2() {
    let fields = vec![
        UniformField::new(
            "position",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([9.0, 8.0, 7.0])),
        ),
        UniformField::new(
            "scale",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec2f([0.25, 0.5])),
        ),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[9.0, 8.0, 7.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
}
