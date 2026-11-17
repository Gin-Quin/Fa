# Independent compile-time widening follow-up

`fa run --target=bun signed-wide-comptime.fa` reports META013 for
`-128 %% 65537` inside `@wide(-128)`. Removing the final two lines executes
and prints the correct `65409` on Bun and native. The result explicitly goes
to Int128; the compile-time lowerer uses the expression storage width rather
than its compute width / widened sink. The original archived compiler behaves
the same. This differs from native narrowing of unsigned operands.

The division boundary fixtures convert signed Int8/Int16 operands explicitly
to Int128 for their large-divisor probe, while their normal division matrices
keep their own widths. No existing expectation was changed or skipped.
