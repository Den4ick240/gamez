struct Camera {
  width: f32,
  height: f32,
  position: vec2<f32>,
  fov: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Input {
  @location(0) position: vec2<f32>,
}

struct InstanceInput {
  @location(1) start: vec2<f32>,
  @location(2) end: vec2<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
}

fn to_camera_pos(world_pos: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (world_pos.x - camera.position.x) * 2 / camera.fov,
        (world_pos.y - camera.position.y) * 2 / (camera.fov * camera.height / camera.width)
    );
}

@vertex
fn vs_arrow(input: Input, instance: InstanceInput) -> Output {
    var length_vector = instance.end - instance.start;
    var norm = normalize(length_vector);
    var width_vector = vec2<f32>(-norm.y, norm.x) * 0.3;
    var local_position = vec2<f32>(input.position.x / 2.0, (input.position.y + 1.0) / 2.0);
    var world_pos = instance.start + length_vector * local_position.y + width_vector * local_position.x;
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_arrow(input: Output) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
