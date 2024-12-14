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
  @location(2) direction: vec2<f32>,
  @location(3) color: vec3<f32>,
  @location(4) norm: f32,
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

const arrow_width = 1.0;

@vertex
fn vs_arrow(input: Input, instance: InstanceInput) -> Output {
    var length_vector = instance.direction;
    var width_vector = vec2<f32>(-instance.direction.y, instance.direction.x);
    var local_position = vec2<f32>(arrow_width * input.position.x / 2.0, (input.position.y + 1.0) * instance.norm / 2.0);
    var world_pos = instance.position + length_vector * local_position.y + width_vector * local_position.x;
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos, 0.0, 1.0);
    out.pos = vec2<f32>(local_position.x, instance.norm - local_position.y);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_arrow(input: Output) -> @location(0) vec4<f32> {
    //if true { return vec4<f32>(input.color, 1.0); }
    const arrow_head_length = arrow_width;
    var x = abs(input.pos.x) * 2;
    var y = input.pos.y;
    if y > arrow_head_length && x > 0.2 {
        discard;
    }
    if x > y {
        discard;
    }
    return vec4<f32>(input.color, 1.0);
}
