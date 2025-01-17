struct Camera {
  width: f32,
  height: f32,
  fov: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera;


struct Input {
  @location(0) position: vec2<f32>,
}

struct InstanceInput {
  @location(1) color: vec3<f32>,
  @location(2) radius: f32,
  @location(3) position: vec2<f32>,
  @location(4) velocity: vec2<f32>,
}

struct Output {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) pos: vec2<f32>,
  @location(1) color: vec3<f32>
}

fn to_camera_pos(world_pos: vec2<f32>) -> vec2<f32> {
    var radius = camera.fov;
    if camera.width < camera.height {
        return vec2<f32>(
            world_pos.x * 2 / radius,
            world_pos.y * 2 / (radius * camera.height / camera.width)
        );
    } else {
        return vec2<f32>(
            world_pos.x * 2 / (radius * camera.width / camera.height),
            world_pos.y * 2 / radius
        );
    }
}


@vertex
fn vs_particles(input: Input, instance: InstanceInput) -> Output {
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
fn fs_particles(input: Output) -> @location(0) vec4<f32> {
    if length(input.pos) > 1.0 {
      discard;
    }
    return vec4<f32>(input.color, 1.0);
}


@group(0) @binding(0) 
var<storage, read_write> particles:  array<InstanceInput>;

struct Simulation {
  spawned_particles: u32,
  dt: f32,
  bound_radius: f32,
}

@group(0) @binding(1)
var<uniform> simulation: Simulation;

@compute @workgroup_size(1)
fn integrate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x < simulation.spawned_particles && global_id.y == 0 && global_id.z == 0 {
        var gravity = vec2<f32>(0.0, -9.8);
        var i = global_id.x;
        var velocity = particles[i].velocity + gravity * simulation.dt;
        particles[i].velocity = particles[i].position;
        particles[i].position += velocity * simulation.dt;

        var bound_radius = simulation.bound_radius;
        var max_length = bound_radius - particles[i].radius;

        var direction = particles[i].position;
        var distance = length(direction);
        if distance > max_length {
            particles[i].position = normalize(direction) * max_length;
        }

        for (var j = 0u; j < simulation.spawned_particles; j = j + 1u) {
            if i == j {
              continue;
            }
            var direction = particles[i].position - particles[j].position;
            var distance = length(direction);
            if distance < particles[i].radius + particles[j].radius {
                var normal = normalize(direction);
                var penetration = (particles[i].radius + particles[j].radius - distance) / 2.0;
                particles[i].position += normal * penetration;
                particles[j].position -= normal * penetration;
            }
        }

        particles[i].velocity = (particles[i].position - particles[i].velocity) / simulation.dt;
        
        //var velocity = particles[i].velocity;
        //particles[i].velocity = particles[i].position;
        //particles[i].position.y = particles[i].position.y - 0.1;
    }
}

