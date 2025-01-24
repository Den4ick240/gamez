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
  @location(4) velocity_or_previous_position: vec2<f32>,
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
fn vs_particles(input: Input, instance: InstanceInput, @builtin(instance_index) i: u32) -> Output {
    var radius = instance.radius;
    var world_pos = instance.position + input.position * radius;
    var camera_pos = to_camera_pos(world_pos);
    var out: Output;
    out.clip_position = vec4<f32>(camera_pos.xy, 0.0, 1.0);
    out.pos = input.position;
    out.color = instance.color;
    //out.color = vec3<f32>(1.0, 0.0, 0.0) * f32(i) / 256;
    //if i == 0 {
    //    out.color = vec3<f32>(0.0, 1.0, 1.0);
    //}
    //if i == 7 {
    //    out.color = vec3<f32>(1.0, 1.0, 1.0);
    //}
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

@group(0) @binding(2)
var<storage, read_write> grid: array<u32>;

struct Sort {
    pass_index: u32,
    sorting_length: u32,
    grid_size: vec2<u32>,
    cell_size: vec2<f32>,
    origin: vec2<f32>,
}

@group(0) @binding(3)
var <uniform> sort: Sort;

fn get_cell_index(position: vec2<f32>) -> u32 {
    var pos = vec2<u32>((position - sort.origin) / sort.cell_size);
    return pos.x + pos.y * sort.grid_size.x;
}

fn compare_and_swap(i1: u32, i2: u32) {
    if get_cell_index(particles[i1].position) > get_cell_index(particles[i2].position) {
        var tmp = particles[i1];
        particles[i1] = particles[i2];
        particles[i2] = tmp;
    }
}

fn do_flip(i: u32, h: u32) {
    let offset = ((i * 2) / h) * h;
    let imh = (i) % (h / 2);
    let i1 = offset + imh;
    let i2 = offset + h - imh - 1;
    compare_and_swap(i1, i2);
}

fn do_disperse(i: u32, h: u32) {
    let offset = ((i * 2) / h) * h;
    let imh = (i) % (h / 2);
    let i1 = offset + imh;
    let i2 = offset + imh + (h / 2);
    compare_and_swap(i1, i2);
}

@compute @workgroup_size(256)
fn sort_particles_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i * 2 < sort.sorting_length {
        for (var h = 2u; h <= sort.sorting_length; h = h * 2u) {
            if h != 2u {
                storageBarrier();
            }
            do_flip(i, h);
            for (var hh = h / 2u; hh >= 2; hh = hh / 2u) {
                storageBarrier();
                do_disperse(i, hh);
            }
        }
    }
}


@compute
@workgroup_size(16, 16)
fn clear_grid_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell = global_id.xy;
    if cell.x < sort.grid_size.x && cell.y < sort.grid_size.y {
        grid[cell.x + cell.y * sort.grid_size.x] = 0xffffffffu;
    }
}

@compute
@workgroup_size(16, 16)
fn collide_grid_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
}

@compute
@workgroup_size(256)
fn fill_grid_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < sort.sorting_length {
        if i == 0 {
            grid[get_cell_index(particles[0].position)] = 0u;
        } else {
            let i1 = i -1;
            let i2 = i;

            let cell1 = get_cell_index(particles[i1].position);
            let cell2 = get_cell_index(particles[i2].position);
            if cell1 != cell2 {
                grid[cell2] = i2;
            }
        }
    }
}

@compute
@workgroup_size(16, 16)
fn colorize_grid_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell = global_id.xy;
    if cell.x < sort.grid_size.x && cell.y < sort.grid_size.y {
        let cell_index = cell.x + cell.y * sort.grid_size.x;
        var i = grid[cell_index];
        if i == 0xffffffffu {
            return;
        }
        loop {
            particles[i].color = vec3<f32>(vec2<f32>(cell) / vec2<f32>(sort.grid_size), 0.0);
            if get_cell_index(particles[i + 1].position) == cell_index {
                i = i + 1u;
            } else {
                break;
            }
        }
    }
}

@compute @workgroup_size(256)
fn apply_circle_constraint_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < simulation.spawned_particles {
        apply_circle_constraint(i);
    }
}
fn apply_circle_constraint(i: u32) {
    var bound_radius = simulation.bound_radius;
    var max_length = bound_radius - particles[i].radius;
    var direction = particles[i].position;
    var distance = length(direction);
    if distance > max_length {
        particles[i].position = normalize(direction) * max_length;
            //particles[i].velocity = reflect(velocity, normalize(direction));
    }
}

@compute @workgroup_size(256)
fn apply_box_constraint_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < simulation.spawned_particles {
        apply_box_constraint(i);
    }
}

fn apply_box_constraint(i: u32) {
    var bound_radius = simulation.bound_radius;
    var max_length = bound_radius - particles[i].radius;

    if particles[i].position.x > max_length {
        particles[i].position.x = max_length;
        //    particles[i].velocity = reflect(velocity, vec2<f32>(-1.0, 0.0));
    }
    if particles[i].position.x < -max_length {
        particles[i].position.x = -max_length;
        //    particles[i].velocity = reflect(velocity, vec2<f32>(1.0, 0.0));
    }
    if particles[i].position.y > max_length {
        particles[i].position.y = max_length;
        //    particles[i].velocity = reflect(velocity, vec2<f32>(0.0, -1.0));
    }
    if particles[i].position.y < -max_length {
        particles[i].position.y = -max_length;
        //    particles[i].velocity = reflect(velocity, vec2<f32>(0.0, 1.0));
    }
}

@compute @workgroup_size(256)
fn naive_collisions_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < simulation.spawned_particles {
        naive_collisions(i);
    }
}

fn naive_collisions(i: u32) {
    for (var j = 0u; j < simulation.spawned_particles; j = j + 1u) {
        if i == j {
              continue;
        }
        var direction = particles[i].position - particles[j].position;
        var distance = length(direction);
        if distance == 0.0 {
            direction = vec2<f32>(0.001, 0.0);
            distance = 0.001;
        }
        if distance < particles[i].radius + particles[j].radius {
            var normal = direction / distance;
            var penetration = (particles[i].radius + particles[j].radius - distance) / 2.0;
            particles[i].position += normal * penetration;
            particles[j].position -= normal * penetration;
        }
    }
}

@compute @workgroup_size(256)
fn finalize_speed_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < simulation.spawned_particles {
        finalize_speed(i);
    }
}
fn finalize_speed(i: u32) {
    particles[i].velocity_or_previous_position = (particles[i].position - particles[i].velocity_or_previous_position) / simulation.dt;
}

fn integrate(i: u32) {
    var gravity = vec2<f32>(0.0, -9.8);
    var velocity = particles[i].velocity_or_previous_position + gravity * simulation.dt;
    particles[i].velocity_or_previous_position = particles[i].position;
    particles[i].position += velocity * simulation.dt;
} 

@compute @workgroup_size(256)
fn update_entry(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i = global_id.x;
    if i < simulation.spawned_particles {
        integrate(i);
        apply_box_constraint(i);
    }
}

