struct Impulse {
    position_radius: vec4<f32>,
    channels: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> impulses: array<Impulse>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) channels: vec4<f32>,
    @location(2) compactness: f32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    let impulse = impulses[instance_index];
    let local = corners[vertex_index];
    let uv = impulse.position_radius.xy + local * impulse.position_radius.z;
    var output: VertexOutput;
    // WebGPU texture row zero is the top row, while clip-space +Y is up.
    // Flip Y so normalized event coordinates match textureLoad coordinates.
    let clip = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    output.position = vec4<f32>(clip, 0.0, 1.0);
    output.local = local;
    output.channels = impulse.channels;
    output.compactness = impulse.position_radius.w;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let radius_squared = dot(input.local, input.local);
    if (radius_squared > 1.0) {
        discard;
    }
    let kernel = exp(-max(input.compactness, 0.25) * radius_squared);
    return input.channels * kernel;
}
