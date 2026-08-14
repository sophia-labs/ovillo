const WORKGROUP_X: u32 = 16u;
const WORKGROUP_Y: u32 = 16u;
const TILE_X: u32 = 18u;
const TILE_Y: u32 = 18u;
const TILE_CELLS: u32 = TILE_X * TILE_Y;
const WORKGROUP_CELLS: u32 = WORKGROUP_X * WORKGROUP_Y;

struct WaveSheetParams {
    dimensions: vec4<u32>,
    integration: vec4<f32>,
    constitutive: vec4<f32>,
    limits: vec4<f32>,
};

@group(0) @binding(0)
var previous_state: texture_2d<f32>;

@group(0) @binding(1)
var next_state: texture_storage_2d<rgba16float, write>;

@group(0) @binding(2)
var impulse_field: texture_2d<f32>;

@group(0) @binding(3)
var<uniform> parameters: WaveSheetParams;

var<workgroup> height_tile: array<f32, 324>;

fn clamped_height(pixel: vec2<i32>) -> f32 {
    let maximum = vec2<i32>(
        i32(parameters.dimensions.x) - 1,
        i32(parameters.dimensions.y) - 1,
    );
    return textureLoad(
        previous_state,
        clamp(pixel, vec2<i32>(0), maximum),
        0,
    ).r;
}

fn tile_index(x: u32, y: u32) -> u32 {
    return y * TILE_X + x;
}

@compute @workgroup_size(16, 16, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let workgroup_origin = vec2<i32>(
        i32(workgroup_id.x * WORKGROUP_X),
        i32(workgroup_id.y * WORKGROUP_Y),
    );
    let local_linear = local_id.y * WORKGROUP_X + local_id.x;
    var offset = local_linear;
    loop {
        if (offset >= TILE_CELLS) {
            break;
        }
        let tile_x = offset % TILE_X;
        let tile_y = offset / TILE_X;
        let source_pixel = workgroup_origin + vec2<i32>(i32(tile_x) - 1, i32(tile_y) - 1);
        height_tile[offset] = clamped_height(source_pixel);
        offset = offset + WORKGROUP_CELLS;
    }
    workgroupBarrier();

    if (global_id.x >= parameters.dimensions.x || global_id.y >= parameters.dimensions.y) {
        return;
    }

    let pixel = vec2<i32>(i32(global_id.x), i32(global_id.y));
    if (parameters.dimensions.z != 0u) {
        textureStore(next_state, pixel, vec4<f32>(0.0));
        return;
    }

    let center = textureLoad(previous_state, pixel, 0);
    let impulse = textureLoad(impulse_field, pixel, 0);
    let tile_x = local_id.x + 1u;
    let tile_y = local_id.y + 1u;
    let laplacian =
        height_tile[tile_index(tile_x - 1u, tile_y)] +
        height_tile[tile_index(tile_x + 1u, tile_y)] +
        height_tile[tile_index(tile_x, tile_y - 1u)] +
        height_tile[tile_index(tile_x, tile_y + 1u)] -
        4.0 * height_tile[tile_index(tile_x, tile_y)];

    let dt = parameters.integration.x;
    var velocity = (
        center.g +
        (parameters.integration.z * laplacian - parameters.integration.w * center.r) * dt +
        impulse.r * parameters.constitutive.w
    ) * parameters.integration.y;
    var height = center.r + velocity * dt;
    var foam =
        center.b * exp(-parameters.constitutive.x * dt) +
        parameters.constitutive.y * abs(velocity) +
        impulse.g;
    var optical =
        center.a * exp(-parameters.constitutive.z * dt) +
        impulse.b;

    height = clamp(height, -parameters.limits.x, parameters.limits.x);
    velocity = clamp(velocity, -parameters.limits.y, parameters.limits.y);
    foam = clamp(foam, 0.0, parameters.limits.z);
    optical = clamp(optical, 0.0, parameters.limits.w);
    textureStore(next_state, pixel, vec4<f32>(height, velocity, foam, optical));
}
