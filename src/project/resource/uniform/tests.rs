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
fn cast_pads_vec3_to_vec2_alignment() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec3f([9.0, 8.0, 7.0])),
        runtime_field(UniformFieldData::Vec2f([0.25, 0.5])),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[9.0, 8.0, 7.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
}

#[test]
fn cast_packs_f32_after_vec3_into_the_same_16_bytes() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec3f([1.0, 2.0, 3.0])),
        runtime_field(UniformFieldData::Float(0.5)),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1.0, 2.0, 3.0, 0.5]);
}

#[test]
fn cast_packs_f32_after_rgb_into_the_same_16_bytes() {
    let fields = vec![
        runtime_field(UniformFieldData::Rgb([0.9, 0.2, 0.2])),
        runtime_field(UniformFieldData::Float(0.09)),
    ];
    let result = cast_fields(&fields);
    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[0.9, 0.2, 0.2, 0.09]);
}

#[test]
fn cast_packs_u32_after_vec3u_into_the_same_16_bytes() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec3u([1, 2, 3])),
        runtime_field(UniformFieldData::UInt32(4)),
    ];
    let result = cast_fields(&fields);
    let result: &[u32] = bytemuck::cast_slice(&result);
    assert_eq!(result, &[1, 2, 3, 4]);
}

#[test]
fn cast_matches_wgsl_offsets_for_a_point_light() {
    let fields = vec![
        runtime_field(UniformFieldData::Vec3f([2.0, 4.0, -2.0])),
        runtime_field(UniformFieldData::Rgb([0.9, 0.2, 0.2])),
        runtime_field(UniformFieldData::Float(0.09)),
        runtime_field(UniformFieldData::Float(0.032)),
    ];
    let result = cast_fields(&fields);
    assert_eq!(
        result.len(),
        48,
        "struct rounds up to its 16 byte alignment"
    );

    let result: &[f32] = bytemuck::cast_slice(&result);
    assert_eq!(result[7], 0.09, "linear sits at byte 28");
    assert_eq!(result[8], 0.032, "quadratic sits at byte 32");
}
