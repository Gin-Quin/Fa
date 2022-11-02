What if we prevent completely the use of generics?

What if instead we rely on parameters' type?

# Simple example 

```ts
function pair<T>(a: T, b: T): [T, T] {
  return [a, b]
}
```

```coffee
# raw idea:
const pair(a: Any, b: TypeOf(a)): [TypeOf(a), TypeOf(a)] ->
  return [a, b]

# improvement with the `as` keyword:
const pair(a: Any as T, b: T): [T, T] => [a, b]
```

# Array example

```ts
class Array<Item> {
  constructor(items: Item[]) {
    this.push(items)
  }
  push(items: Item[]) {

  }
  at(index: number): Item {

  }
}
```

```coffee
object Array:
  items: Array<Any as Item>
  push(items: Item[]) ->
    # ...
  at(index: Number): Item ->
    return items[index]
```

## Conclusion

I'm not sure yet if i should implement it or not.

It means types cannot fully be resolved before some parameters are passed. Which is not a bad thing!