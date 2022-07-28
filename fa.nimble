# Package

version       = "0.1.0"
author        = "lepzulnag"
description   = "Fa is a programming language which is intended to be as fast as low-level languages like C++ and Rust and as easy to use as high-level languages like Python and Javascript."
license       = "GPL-3.0-only"
srcDir        = "src"
bin           = @["bin/fa"]


# Dependencies

requires "nim >= 1.6.2"
requires "npeg >= 0.26.0"
