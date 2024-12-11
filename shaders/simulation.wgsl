
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
  @location(1) position: vec2<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) pos: vec2<f32>,
}

@vertex
fn vs_mouse(input: Input, instance: InstanceInput) -> Output {
    const radius = 10;
    var world_pos = instance.position + input.position;
    var x = (world_pos.x - camera.position.x) * 2 / camera.fov;
    var y = (world_pos.y - camera.position.y) * 2 / (camera.fov * camera.height / camera.width);
    var out: Output;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.pos = input.position;
    return out;
}

@fragment
fn fs_mouse(input: Output) -> @location(0) vec4<f32> {
    return vec4<f32>(input.pos.x, 1.0, 1.0, 1.0);
}
