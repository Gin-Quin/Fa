# File headers

**File headers** describe the meta properties of a file.

The default file headers in `Fa` are :

<table>
   <tr>
      <th> File header
      <th> Module
      <th> Class file
      <th> Data object

   <tr>
      <td> @use&nbsp;<i>path</i>
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅<br> Yes ; but cannot access methods nor non-constant values

   <tr>
      <td> @import&nbsp;<i>path</i>
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅<br> Yes ; but cannot access methods nor non-constant values

   <tr>
      <td> @is&nbsp;<i>path</i>
      <td style="text-align: center"> ❌
      <td style="text-align: center"> ✅<br> "extends"
      <td style="text-align: center"> ✅<br> "is instance of"

   <tr>
      <td> @with&nbsp;<i>path</i>
      <td style="text-align: center"> ❌
      <td style="text-align: center"> ✅<br> "extends trait"
      <td style="text-align: center"> ✅<br> "has trait"

   <tr>
      <td> @start
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ❌

   <tr>
      <td> @native
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ✅
      <td style="text-align: center"> ❌

</table>


#### @use *path*

Use a library.
```coffee
@use path
@use value from path
@use value1, value2, ... from path
```

#### @import *path*

Import a module, a class or a data object.
```coffee
@import path
@import value from path
@import value1, value2, ... from path
```

#### @is *path*

*path* can be a `class file` only.

- `class file` : the class extends the given parent class,
- `data object` : the object is an instance of the given class.

```coffee
@is path
@is path with trait1, trait2
@is superclass from path
@is superclass with trait1, trait2 from path
```

A class can inherit from one and only one parent class.

#### @with *path*

*path* can be a `trait file` only.

- `class file` : the class extends the given traits,
- `data object` : the object has the given traits.
```coffee
@with path
@with trait1, trait2 from path
```

#### @start

All code following the `@start` header will be executed the first time the file is loaded.

#### @native

In case of hybrid programming (Javascript + Wasm), indicates this module should be compiled as `Wasm`.

Other ways, this header is ignored.

### Custom headers

Custom file headers can be used without breaking compilation.

A file header has the following expression : `@([a-z]+) +(.*)`. Two values are captured&nbsp;: the header name and its value (which can be anything).

`Fa` exports a very fast function to parse the headers of a file.
