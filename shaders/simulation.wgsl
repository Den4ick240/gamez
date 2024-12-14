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
  @location(2) speed: vec2<f32>,
  @location(3) radius: f32,
  @location(4) color: vec3<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) pos: vec2<f32>,
  @location(1) color: vec3<f32>,
}

fn to_camera_pos(world_pos: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (world_pos.x - camera.position.x) * 2 / camera.fov,
        (world_pos.y - camera.position.y) * 2 / (camera.fov * camera.height / camera.width)
    );
}

@vertex
fn vs_mouse(input: Input, instance: InstanceInput) -> Output {
    var radius = instance.radius;
    var world_pos = instance.position + input.position * radius;
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos.xy, 0.0, 1.0);
    out.pos = input.position;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_mouse(input: Output) -> @location(0) vec4<f32> {
    if length(input.pos) > 1.0 {
      discard;
    }
    let c = input.pos.y * 0.5 + 0.5;
    return vec4<f32>(1.0, c, c, 1.0) * vec4<f32>(input.color, 1.0);
}
