# data object

A `data object` is to `Fa` what `json` is to `Javascript`. They are very useful for configuration.

A `data object` :

- has the `.do` extension,
- cannot have `methods`, `getters` or `setters`,
- values can only be `constants` or `computed from constants`.
- values cannot be uninitialized,
- can import other `data objects`,
- can import other `.fa` files but can only access to the constant values,
- can be defined as an instance of a `class file` (using `@is`).

Because of these properties, a `data object` can fully be loaded at runtime.

## Examples

#### Basic example

```coffee
name: "Johnny John"
job: "Painter"
skill: "elite"
employed: true

foods:
  - "Apple"
  - "Orange"
  - "Strawberry"
  - "Mango"

miscellanous:
   height: 180
   weight: 66

   hair:
      color: "brown"
      length: "medium"
```

#### With computations
```coffee
me:
   firstname: "Johnny"
   lastname: "John"
   fullname: firstname + " " + lastname

theGuyYouNeedIs: me.fullname
```

#### Importing another data object

- `/me.do` :
   ```coffee
   firstname: "Johnny"
   lastname: "John"
   fullname: firstname + " " + lastname

   theGuyYouNeedIs: self.fullname
   ```

- `/hero.do` :
   ```coffee
   import me

   firstname: "Johnny"
   lastname: "John"
   fullname: firstname + " " + lastname

   theGuyYouNeedIs: me.fullname
   ```
