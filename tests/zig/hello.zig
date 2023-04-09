const std = @import("std");
const foo = @import("foo.zig");

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("Hello, {}!\n", .{foo.x});
}
