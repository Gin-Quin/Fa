# Package

version       = "0.1.0"
author        = "Gin Quin"
description   = "Fa is a programming language which is intended to be as fast as low-level languages like C++ and Rust and as easy to use as high-level languages like Python and Javascript."
license       = "GPL-3.0-only"
srcDir        = "source"
bin           = @["bin/fa"]


# Dependencies

requires "nim >= 1.6.2"
requires "npeg >= 0.26.0"
