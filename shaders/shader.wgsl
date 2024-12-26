struct Camera {
  width: f32,
  height: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Input {
  @location(0) position: vec2<f32>,
}

struct InstanceInput {
  @location(1) position: vec2<f32>,
  @location(2) radius: f32,
  @location(3) color: vec3<f32>,
}

struct ColorInput {
  @location(4) color: vec3<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) pos: vec2<f32>,
  @location(1) color: vec3<f32>
}

fn to_camera_pos(world_pos: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        world_pos.x * 2 / 160,
        world_pos.y * 2 / (160 * camera.height / camera.width)
    );
}

@vertex
fn vs_simulation(input: Input, instance: InstanceInput, color_input: ColorInput) -> Output {
    var radius = instance.radius;
    var world_pos = instance.position + input.position * radius;
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos.xy, 0.0, 1.0);
    out.pos = input.position;
    out.color = color_input.color;
    return out;
}

@fragment
fn fs_simulation(input: Output) -> @location(0) vec4<f32> {
    if length(input.pos) > 1.0 {
      discard;
    }
    return vec4<f32>(input.color, 1.0);
}
