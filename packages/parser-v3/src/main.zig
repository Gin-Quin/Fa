const std = @import("std");

pub const Parser = struct {
    // Parser state will go here
    allocator: std.mem.Allocator,

    pub fn init(allocator: std.mem.Allocator) Parser {
        return .{
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Parser) void {
        // Cleanup code will go here
        _ = self;
    }
};

test "basic parser initialization" {
    var parser = Parser.init(std.testing.allocator);
    defer parser.deinit();
}
