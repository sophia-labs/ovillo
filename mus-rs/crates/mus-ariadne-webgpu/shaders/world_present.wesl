struct PresentationParams {
    viewport: vec4<f32>,
    palette: vec4<f32>,
};

@group(0) @binding(0)
var optical_field: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> parameters: PresentationParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = 0.5 * (position + vec2<f32>(1.0));
    return output;
}

fn load_field(pixel: vec2<i32>) -> vec4<f32> {
    let dimensions = textureDimensions(optical_field);
    let maximum = vec2<i32>(i32(dimensions.x) - 1, i32(dimensions.y) - 1);
    return textureLoad(optical_field, clamp(pixel, vec2<i32>(0), maximum), 0);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(optical_field);
    let pixel = vec2<i32>(
        i32(clamp(input.uv.x, 0.0, 0.999999) * f32(dimensions.x)),
        i32(clamp(input.uv.y, 0.0, 0.999999) * f32(dimensions.y)),
    );
    let center = load_field(pixel);
    let derivative_x = load_field(pixel + vec2<i32>(1, 0)).r
        - load_field(pixel - vec2<i32>(1, 0)).r;
    let derivative_y = load_field(pixel + vec2<i32>(0, 1)).r
        - load_field(pixel - vec2<i32>(0, 1)).r;
    let normal = normalize(vec3<f32>(-3.8 * derivative_x, -3.8 * derivative_y, 1.0));
    let light = normalize(vec3<f32>(-0.46, -0.64, 0.84));
    let diffuse = 0.20 + 0.80 * max(dot(normal, light), 0.0);
    let fresnel = pow(1.0 - max(normal.z, 0.0), 2.4);

    let height = center.r;
    let motion = center.g;
    let structure = max(center.b, 0.0);
    let emission = max(center.a, 0.0);
    let deep = vec3<f32>(0.008, 0.016, 0.052);
    let shallow = vec3<f32>(0.045, 0.34, 0.56);
    var color = mix(deep, shallow, clamp(0.48 + 0.22 * height, 0.0, 1.0));
    color = color * diffuse + fresnel * vec3<f32>(0.22, 0.44, 0.72);
    color += parameters.palette.x * structure * vec3<f32>(0.62, 0.84, 1.0);
    color += parameters.palette.z * emission * vec3<f32>(1.0, 0.48, 0.16);
    color += parameters.palette.y * abs(motion)
        * select(vec3<f32>(0.28, 0.98, 0.80), vec3<f32>(1.0, 0.24, 0.64), motion < 0.0);

    let centered = (input.uv - vec2<f32>(0.5))
        * vec2<f32>(parameters.viewport.x, 1.0);
    color *= 1.0 - parameters.palette.w
        * smoothstep(0.30, 1.12, length(centered));
    color = vec3<f32>(1.0) - exp(-max(color, vec3<f32>(0.0)) * parameters.viewport.z);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / parameters.viewport.w));
    return vec4<f32>(color, 1.0);
}
