Meta-programming with the file system is one core feature of Fa.

Using the file system is something that has grown recently. The best example is routers. It is a very natural way to use a file system to define a router. But in mlost programming languages, there is a rupture between the code logic and the file system itself, the first not being able to access informations from the second.

## Use cases

1. Reading content from file system to avoid code generation

```cpp
const #readVersion = () => #readRelativeFile("../version.txt") // read relatively to current source file location (not cwd)
const version = #readVersion() // will be transpiled into `const version = "1.0.0"` for example
```

2. Create a router based on the file system

```cpp
const routes = #readDirectory("/routes")
```
