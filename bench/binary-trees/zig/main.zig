// Binary-trees: allocation, teardown, and pointer-chasing. Single-threaded,
// plain per-node create/destroy through a general-purpose allocator — no
// pools or arenas. (smp_allocator: the build command does not link libc.)
const std = @import("std");

const Node = struct {
    left: ?*Node,
    right: ?*Node,
};

fn make(allocator: std.mem.Allocator, depth: i32) *Node {
    const n = allocator.create(Node) catch unreachable;
    if (depth == 0) {
        n.left = null;
        n.right = null;
    } else {
        n.left = make(allocator, depth - 1);
        n.right = make(allocator, depth - 1);
    }
    return n;
}

fn check(n: ?*Node) i64 {
    const node = n orelse return 0;
    return 1 + check(node.left) + check(node.right);
}

fn drop(allocator: std.mem.Allocator, n: ?*Node) void {
    const node = n orelse return;
    drop(allocator, node.left);
    drop(allocator, node.right);
    allocator.destroy(node);
}

pub fn main() !void {
    const allocator = std.heap.smp_allocator;
    var stdout_buf: [4096]u8 = undefined;
    var stdout_writer = std.fs.File.stdout().writer(&stdout_buf);
    const out = &stdout_writer.interface;

    const max_depth: i32 = 18;
    var total: i64 = 0;

    const stretch = make(allocator, max_depth + 1);
    const sc = check(stretch);
    drop(allocator, stretch);
    try out.print("stretch tree of depth {d} check: {d}\n", .{ max_depth + 1, sc });
    total += sc;

    const long_lived = make(allocator, max_depth);

    var depth: i32 = 4;
    while (depth <= max_depth) : (depth += 2) {
        const iterations = @as(i64, 1) << @intCast(max_depth - depth + 4);
        var sum: i64 = 0;
        var i: i64 = 0;
        while (i < iterations) : (i += 1) {
            const t = make(allocator, depth);
            sum += check(t);
            drop(allocator, t);
        }
        try out.print("{d} trees of depth {d} check: {d}\n", .{ iterations, depth, sum });
        total += sum;
    }

    const ll = check(long_lived);
    drop(allocator, long_lived);
    try out.print("long lived tree of depth {d} check: {d}\n", .{ max_depth, ll });
    total += ll;

    try out.print("Result: {d}\n", .{total});
    try out.flush();
}
