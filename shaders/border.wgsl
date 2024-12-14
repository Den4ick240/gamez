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
  @location(1) top: f32,
  @location(2) left: f32,
  @location(3) right: f32,
  @location(4) bottom: f32,
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
fn vs_border(input: Input, instance: InstanceInput) -> Output {
    var center = vec2<f32>((instance.left + instance.right) / 2, (instance.top + instance.bottom) / 2);
    var width = instance.right - instance.left;
    var height = instance.top - instance.bottom;
    var world_pos = vec2<f32>(input.position.x * width / 2 + center.x, input.position.y * height / 2 + center.y);
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_border(input: Output) -> @location(0) vec4<f32> {
    return vec4<f32>(0.5, 0.5, 0.5, 1.0);
}
