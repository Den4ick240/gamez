struct MouseState {
    position: vec2<f32>,
    animation_progress: f32,
    is_clicked: u32
};

struct Camera {
  width: f32,
  height: f32
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var<uniform> mouse: MouseState;

const clicked_radius = 0.01;
const default_radius = 0.005;

fn get_mouse_radius() -> f32 {
    if mouse.is_clicked > 0 {
        return mix(default_radius, clicked_radius, mouse.animation_progress);
    } else {
        return mix(clicked_radius, default_radius, mouse.animation_progress);
    }
}

fn get_mouse_center() -> vec2<f32> {
    return vec2<f32>(-1, 1) + mouse.position * vec2<f32>(2.0, -2.0) / vec2<f32>(camera.width, camera.height);
}

struct Input {
  @location(0) position: vec2<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_mouse(input: Input) -> Output {
    var center = get_mouse_center();
    var pos = center + input.position * get_mouse_radius();
    var out: Output;
    out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_mouse(input: Output) -> @location(0) vec4<f32> {
    var radius = get_mouse_radius();
    var center = get_mouse_center();
    var pos = vec2<f32>(-1 + input.clip_position.x * 2 / camera.width, 1 + input.clip_position.y * -2 / camera.height);
    var distance = length(center - pos);

    if distance > radius {
      discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
