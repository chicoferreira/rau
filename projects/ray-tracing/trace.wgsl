// The path tracer. Runs every frame, after `generate.wgsl` has filled the scene
// texture and `reset.wgsl` has cleared the accumulation buffers.
//
// Ported from Peter Shirley, Trevor David Black and Steve Hollasch's
// *Ray Tracing in One Weekend* (CC0): https://raytracing.github.io
//
// One invocation owns one pixel. Every frame it traces `samples_per_frame` rays
// through that pixel, folds the result into the mean already stored for it, and
// bumps that pixel's sample count by one:
//
//     mean = (mean * n + sample) / (n + 1)
//
// So the longer the viewport holds still, the more samples each pixel has
// averaged and the cleaner it gets. `reset.wgsl` zeroes the counts whenever
// something affecting the image changes, which starts the average over, and
// `display.wgsl` only has to read the mean out and gamma correct it.
//
// The mean is kept in three `R32Float` textures, one per colour channel, plus a
// fourth for the count, rather than in one `Rgba32Float`. WebGPU only guarantees
// read-write storage access for the single-channel 32-bit formats. Otherwise
// there would need to be another texture to write the results and a copy pass
// to keep the two in sync.
//
// There is no acceleration structure, so intersection is brute forced: every ray
// tests every sphere and cost scales linearly with the sphere count. A few
// hundred spheres is the practical ceiling.

@group(0) @binding(0)
var accumulation_red: texture_storage_2d<r32float, read_write>;
@group(0) @binding(1)
var accumulation_green: texture_storage_2d<r32float, read_write>;
@group(0) @binding(2)
var accumulation_blue: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3)
var sample_count: texture_storage_2d<r32float, read_write>;
@group(0) @binding(4)
var scene: texture_2d<f32>;

struct Camera {
    position: vec4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
}
@group(0) @binding(5)
var<uniform> camera: Camera;

struct Render {
    sky_zenith: vec3<f32>,
    max_bounces: u32,
    sky_horizon: vec3<f32>,
    samples_per_frame: u32,
    defocus_angle: f32,
    focus_distance: f32,
}
@group(0) @binding(6)
var<uniform> render: Render;

const PI: f32 = 3.141592653589793;

const KIND_METAL: u32 = 1u;
const KIND_DIELECTRIC: u32 = 2u;

// Keeps a bounced ray from re-hitting the surface it just left.
const T_MIN: f32 = 0.001;
const T_MAX: f32 = 1e30;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct Hit {
    t: f32,
    sphere: i32,
}

// Same generator as `generate.wgsl`; see the comment there.
fn next_u32(state: ptr<function, u32>) -> u32 {
    let advanced = (*state) * 747796405u + 2891336453u;
    *state = advanced;
    let word = ((advanced >> ((advanced >> 28u) + 4u)) ^ advanced) * 277803737u;
    return (word >> 22u) ^ word;
}

fn next_f32(state: ptr<function, u32>) -> f32 {
    return f32(next_u32(state)) * (1.0 / 4294967296.0);
}

// A direction drawn uniformly over the sphere, sampled analytically rather than
// by rejection so no invocation loops longer than its neighbours.
fn random_unit_vector(state: ptr<function, u32>) -> vec3<f32> {
    let z = next_f32(state) * 2.0 - 1.0;
    let angle = next_f32(state) * 2.0 * PI;
    let radius = sqrt(max(1.0 - z * z, 0.0));
    return vec3<f32>(radius * cos(angle), radius * sin(angle), z);
}

fn random_in_unit_disk(state: ptr<function, u32>) -> vec2<f32> {
    let angle = next_f32(state) * 2.0 * PI;
    // The square root spreads the samples evenly over the area, not the radius.
    let radius = sqrt(next_f32(state));
    return vec2<f32>(cos(angle), sin(angle)) * radius;
}

// Schlick's approximation of the Fresnel reflectance.
fn reflectance(cosine: f32, refraction_index: f32) -> f32 {
    var r0 = (1.0 - refraction_index) / (1.0 + refraction_index);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow(1.0 - cosine, 5.0);
}

// Nearest root of the ray/sphere quadratic inside `(T_MIN, t_max)`, or -1.
fn hit_sphere(center: vec3<f32>, radius: f32, ray: Ray, t_max: f32) -> f32 {
    let oc = center - ray.origin;
    let a = dot(ray.direction, ray.direction);
    let h = dot(ray.direction, oc);
    let c = dot(oc, oc) - radius * radius;

    let discriminant = h * h - a * c;
    if discriminant < 0.0 {
        return -1.0;
    }

    let sqrt_discriminant = sqrt(discriminant);
    var root = (h - sqrt_discriminant) / a;
    if root <= T_MIN || root >= t_max {
        root = (h + sqrt_discriminant) / a;
        if root <= T_MIN || root >= t_max {
            return -1.0;
        }
    }
    return root;
}

// Walks every occupied slot of the scene texture. This is the hot loop, and it
// only reads row 0, which is why `generate.wgsl` stores the spheres as a
// structure of arrays: geometry and material live in separate rows, so the
// materials are never fetched during the search.
fn closest_hit(ray: Ray) -> Hit {
    var hit: Hit;
    hit.t = T_MAX;
    hit.sphere = -1;

    let capacity = i32(textureDimensions(scene).x);
    for (var slot = 0; slot < capacity; slot++) {
        let geometry = textureLoad(scene, vec2<i32>(slot, 0), 0);
        if geometry.w <= 0.0 {
            continue; // empty slot
        }

        let t = hit_sphere(geometry.xyz, geometry.w, ray, hit.t);
        if t > 0.0 {
            hit.t = t;
            hit.sphere = slot;
        }
    }

    return hit;
}

// What a ray returns when it escapes the scene: a vertical gradient between the
// two sky colours in the render uniform.
fn sky(direction: vec3<f32>) -> vec3<f32> {
    let t = 0.5 * (normalize(direction).y + 1.0);
    return mix(render.sky_horizon, render.sky_zenith, t);
}

// Follows one camera ray until it escapes to the sky, is absorbed, or runs out
// of bounces. WGSL has no recursion, so this is a loop that carries the
// accumulated attenuation explicitly.
fn ray_color(primary: Ray, state: ptr<function, u32>) -> vec3<f32> {
    var ray = primary;
    var attenuation = vec3<f32>(1.0);
    let max_bounces = max(render.max_bounces, 1u);

    for (var bounce = 0u; bounce < max_bounces; bounce++) {
        let hit = closest_hit(ray);
        if hit.sphere < 0 {
            return attenuation * sky(ray.direction);
        }

        let geometry = textureLoad(scene, vec2<i32>(hit.sphere, 0), 0);
        let material = textureLoad(scene, vec2<i32>(hit.sphere, 1), 0);
        let kind = u32(textureLoad(scene, vec2<i32>(hit.sphere, 2), 0).x);

        let point = ray.origin + ray.direction * hit.t;
        let outward_normal = (point - geometry.xyz) / geometry.w;
        let front_face = dot(ray.direction, outward_normal) < 0.0;
        let normal = select(-outward_normal, outward_normal, front_face);

        let incident = normalize(ray.direction);
        var scattered: vec3<f32>;

        if kind == KIND_METAL {
            // `material.w` is the fuzz radius: a perfect mirror blurred by a
            // random offset proportional to it.
            scattered = normalize(reflect(incident, normal))
                + material.w * random_unit_vector(state);
            if dot(scattered, normal) <= 0.0 {
                return vec3<f32>(0.0); // scattered below the surface, absorbed
            }
            attenuation *= material.rgb;
        } else if kind == KIND_DIELECTRIC {
            // Entering the sphere divides by the index of refraction, leaving it
            // multiplies. Clear glass tints nothing, so attenuation is untouched.
            let ratio = select(material.w, 1.0 / material.w, front_face);
            let cos_theta = min(dot(-incident, normal), 1.0);
            let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));

            let total_internal_reflection = ratio * sin_theta > 1.0;
            if total_internal_reflection || reflectance(cos_theta, ratio) > next_f32(state) {
                scattered = reflect(incident, normal);
            } else {
                scattered = refract(incident, normal, ratio);
            }
        } else {
            // Lambertian: a cosine-weighted direction around the normal.
            scattered = normal + random_unit_vector(state);
            if all(abs(scattered) < vec3<f32>(1e-8)) {
                scattered = normal; // the random vector cancelled the normal out
            }
            attenuation *= material.rgb;
        }

        ray = Ray(point, scattered);
    }

    // Out of bounces: the remaining light is assumed lost.
    return vec3<f32>(0.0);
}

// Reconstructs a world-space ray through a jittered point inside the pixel, from
// the camera's inverse projection and inverse view. The jitter is what
// antialiases the edges: every frame samples a different point in the pixel, and
// the accumulation averages them.
fn camera_ray(pixel: vec2<u32>, size: vec2<u32>, state: ptr<function, u32>) -> Ray {
    let jitter = vec2<f32>(next_f32(state), next_f32(state));
    let uv = (vec2<f32>(pixel) + jitter) / vec2<f32>(size);
    // Clip space has y pointing up, the framebuffer has it pointing down.
    let clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 1.0, 1.0);

    let view_position = camera.inv_proj * clip;
    let view_direction = view_position.xyz / view_position.w;

    var origin = camera.position.xyz;
    var direction = normalize((camera.inv_view * vec4<f32>(view_direction, 0.0)).xyz);

    if render.defocus_angle > 0.0 {
        // The inverse view matrix is world-from-camera, so its columns are the
        // camera basis the lens disk is spread over.
        let right = normalize(camera.inv_view[0].xyz);
        let up = normalize(camera.inv_view[1].xyz);
        let forward = -normalize(camera.inv_view[2].xyz);

        // Everything on the plane `focus_distance` ahead stays sharp; the rest
        // blurs by however far the origin moved across the lens.
        let focus_point = origin
            + direction * (render.focus_distance / max(dot(direction, forward), 1e-4));
        let lens_radius = render.focus_distance * tan(radians(render.defocus_angle) * 0.5);
        let lens = random_in_unit_disk(state) * lens_radius;

        origin += right * lens.x + up * lens.y;
        direction = normalize(focus_point - origin);
    }

    return Ray(origin, direction);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(sample_count);
    if gid.x >= size.x || gid.y >= size.y {
        return;
    }

    let coord = vec2<i32>(gid.xy);
    // How many samples this texel already holds. An `f32` counts exactly up to
    // 2^24, far longer than anyone leaves a viewport still.
    let count = textureLoad(sample_count, coord).x;

    // Seeding from the pixel *and* the sample count gives every frame a fresh
    // sequence, which is what turns the noise into convergence rather than a
    // fixed pattern.
    var state = gid.x * 1973u + gid.y * 9277u + u32(count) * 26699u;
    _ = next_u32(&state);

    let samples = max(render.samples_per_frame, 1u);
    var color = vec3<f32>(0.0);
    for (var sample = 0u; sample < samples; sample++) {
        color += ray_color(camera_ray(gid.xy, size, &state), &state);
    }
    color /= f32(samples);

    var result = color;
    if count > 0.0 {
        let previous = vec3<f32>(
            textureLoad(accumulation_red, coord).x,
            textureLoad(accumulation_green, coord).x,
            textureLoad(accumulation_blue, coord).x,
        );
        result = (previous * count + color) / (count + 1.0);
    }

    textureStore(accumulation_red, coord, vec4<f32>(result.r));
    textureStore(accumulation_green, coord, vec4<f32>(result.g));
    textureStore(accumulation_blue, coord, vec4<f32>(result.b));
    textureStore(sample_count, coord, vec4<f32>(count + 1.0));
}
