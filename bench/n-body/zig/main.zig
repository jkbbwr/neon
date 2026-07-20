// N-body: the benchmarks-game gravitational integrator, single-threaded.
// Faithful port of ../c/main.c — identical constants and operation order.
const std = @import("std");

const pi = 3.141592653589793;
const solar_mass = 4 * pi * pi;
const days_per_year = 365.24;
const n_bodies = 5;

const Body = struct {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    mass: f64,
};

var bodies = [n_bodies]Body{
    .{ // sun
        .x = 0, .y = 0, .z = 0, .vx = 0, .vy = 0, .vz = 0, .mass = solar_mass,
    },
    .{ // jupiter
        .x = 4.84143144246472090e+00,
        .y = -1.16032004402742839e+00,
        .z = -1.03622044471123109e-01,
        .vx = 1.66007664274403694e-03 * days_per_year,
        .vy = 7.69901118419740425e-03 * days_per_year,
        .vz = -6.90460016972063023e-05 * days_per_year,
        .mass = 9.54791938424326609e-04 * solar_mass,
    },
    .{ // saturn
        .x = 8.34336671824457987e+00,
        .y = 4.12479856412430479e+00,
        .z = -4.03523417114321381e-01,
        .vx = -2.76742510726862411e-03 * days_per_year,
        .vy = 4.99852801234917238e-03 * days_per_year,
        .vz = 2.30417297573763929e-05 * days_per_year,
        .mass = 2.85885980666130812e-04 * solar_mass,
    },
    .{ // uranus
        .x = 1.28943695621391310e+01,
        .y = -1.51111514016986312e+01,
        .z = -2.23307578892655734e-01,
        .vx = 2.96460137564761618e-03 * days_per_year,
        .vy = 2.37847173959480950e-03 * days_per_year,
        .vz = -2.96589568540237556e-05 * days_per_year,
        .mass = 4.36624404335156298e-05 * solar_mass,
    },
    .{ // neptune
        .x = 1.53796971148509165e+01,
        .y = -2.59193146099879641e+01,
        .z = 1.79258772950371181e-01,
        .vx = 2.68067772490389322e-03 * days_per_year,
        .vy = 1.62824170038242295e-03 * days_per_year,
        .vz = -9.51592254519715870e-05 * days_per_year,
        .mass = 5.15138902046611451e-05 * solar_mass,
    },
};

fn offsetMomentum() void {
    var px: f64 = 0;
    var py: f64 = 0;
    var pz: f64 = 0;
    for (0..n_bodies) |i| {
        px += bodies[i].vx * bodies[i].mass;
        py += bodies[i].vy * bodies[i].mass;
        pz += bodies[i].vz * bodies[i].mass;
    }
    bodies[0].vx = -px / solar_mass;
    bodies[0].vy = -py / solar_mass;
    bodies[0].vz = -pz / solar_mass;
}

fn advance(dt: f64) void {
    for (0..n_bodies) |i| {
        for (i + 1..n_bodies) |j| {
            const dx = bodies[i].x - bodies[j].x;
            const dy = bodies[i].y - bodies[j].y;
            const dz = bodies[i].z - bodies[j].z;
            const d2 = dx * dx + dy * dy + dz * dz;
            const mag = dt / (d2 * @sqrt(d2));
            bodies[i].vx -= dx * bodies[j].mass * mag;
            bodies[i].vy -= dy * bodies[j].mass * mag;
            bodies[i].vz -= dz * bodies[j].mass * mag;
            bodies[j].vx += dx * bodies[i].mass * mag;
            bodies[j].vy += dy * bodies[i].mass * mag;
            bodies[j].vz += dz * bodies[i].mass * mag;
        }
    }
    for (0..n_bodies) |i| {
        bodies[i].x += dt * bodies[i].vx;
        bodies[i].y += dt * bodies[i].vy;
        bodies[i].z += dt * bodies[i].vz;
    }
}

fn energy() f64 {
    var e: f64 = 0;
    for (0..n_bodies) |i| {
        e += 0.5 * bodies[i].mass *
            (bodies[i].vx * bodies[i].vx + bodies[i].vy * bodies[i].vy +
                bodies[i].vz * bodies[i].vz);
        for (i + 1..n_bodies) |j| {
            const dx = bodies[i].x - bodies[j].x;
            const dy = bodies[i].y - bodies[j].y;
            const dz = bodies[i].z - bodies[j].z;
            e -= bodies[i].mass * bodies[j].mass /
                @sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return e;
}

pub fn main() !void {
    const n = 20000000;
    offsetMomentum();
    var buf: [256]u8 = undefined;
    var file_writer = std.fs.File.stdout().writer(&buf);
    const out = &file_writer.interface;
    var before_buf: [32]u8 = undefined;
    var after_buf: [32]u8 = undefined;
    const before = try std.fmt.bufPrint(&before_buf, "{d:.9}", .{energy()});
    try out.print("{s}\n", .{before});
    var i: usize = 0;
    while (i < n) : (i += 1) advance(0.01);
    const after = try std.fmt.bufPrint(&after_buf, "{d:.9}", .{energy()});
    try out.print("{s}\n", .{after});
    try out.print("Result: {s} {s}\n", .{ before, after });
    try out.flush();
}
