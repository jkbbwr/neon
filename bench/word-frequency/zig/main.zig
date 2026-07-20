// Word-frequency: string building, string hashing, and hash-map upserts using the
// standard std.StringHashMap with owned string keys.
const std = @import("std");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    var counts = std.StringHashMap(i64).init(allocator);
    defer counts.deinit();

    var x: i64 = 42;
    const n: i64 = 10000000;
    var buf: [32]u8 = undefined;

    var i: i64 = 0;
    while (i < n) : (i += 1) {
        x = @mod(x * 48271, 2147483647);
        const word = std.fmt.bufPrint(&buf, "w{d}", .{@mod(x, 10000)}) catch unreachable;
        const gop = try counts.getOrPut(word);
        if (gop.found_existing) {
            gop.value_ptr.* += 1;
        } else {
            gop.key_ptr.* = try allocator.dupe(u8, word);
            gop.value_ptr.* = 1;
        }
    }

    var max: i64 = 0;
    var distinct: usize = 0;
    var it = counts.iterator();
    while (it.next()) |entry| {
        distinct += 1;
        if (entry.value_ptr.* > max) max = entry.value_ptr.*;
    }

    const stdout = std.fs.File.stdout();
    var out_buf: [128]u8 = undefined;
    const str = std.fmt.bufPrint(&out_buf, "Result: {d} {d} {d}\n", .{ distinct, n, max }) catch unreachable;
    stdout.writeAll(str) catch {};
}
