use super::*;

fn runtime_field(data: UniformFieldData) -> UniformRuntimeField {
    UniformRuntimeField { data }
}

#[test]
fn cast_pads_vec2_to_vec4_alignment() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec2f([1.0, 2.0])),
        runtime_field(UniformFieldData::Vec4f([3.0, 4.0, 5.0, 6.0])),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn cast_pads_vec2_to_rgb_alignment() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec2f([1.5, 2.5])),
        runtime_field(UniformFieldData::Rgb([0.1, 0.2, 0.3])),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1.5, 2.5, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0]);
}

#[test]
fn cast_pads_f32_before_vec3_to_vec3_alignment() {
    let fields = vec![
        runtime_field(UniformFieldData::Float(0.5)),
        runtime_field(UniformFieldData::Vec3f([1.0, 2.0, 3.0])),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[0.5, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 0.0]);
}

#[test]
fn cast_no_padding_between_vec3_and_vec2() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec3f([9.0, 8.0, 7.0])),
        runtime_field(UniformFieldData::Vec2f([0.25, 0.5])),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[9.0, 8.0, 7.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
}
