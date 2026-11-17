# Deferred static-type field enumeration acceptance

Lead decision, 2026-11-16: [O34c](../../../../todo.md#o34c-static-type-field-enumeration), with ownership and written origins. The string redesign retains trunk's enumeration behavior. These fixtures describe the decided future rule and are deliberately outside the required program suites.

Run printing and stringify cases on the compile-time interpreter, Bun, Node, web, native, wasm and native-freestanding. JSON's current freestanding AN004 refusal belongs to WP15; that target's JSON acceptance needs WP15 as well. The A/B/anonymous cases include conversion to B and test static-type order, not the physical runtime object's order. The other fixtures pin two spellings, nesting, recursion, schemas/key loops, UTF-16 name comparison, and source-order evaluation.

The shared differential generator retains the `field-order-` and `field-enumeration-` classes. `DEFERRED_CASE_CLASSES` names them explicitly and points to O34c. Run them with:

```sh
FA_STRINGS_DIFFERENTIAL_DEFERRED=1 FA_STRINGS_DIFFERENTIAL_FULL=1 FA_PROGRAMS=field- cargo test -p fa --test programs generated_string_differential -- --nocapture
```

They are expected to fail on this branch. Remove the named exclusions when the written-origin implementation satisfies the decision. `field-order.fa` and its `.js` show the intended property order and effect staging; they are no longer an active golden.
