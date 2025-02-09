In JS/TS, it is impossible to pass an object as parameters to a function.

That is very annoying because it is a super-useful feature.

In Fa, you can use the spread operator like this:

```coffee
callMe(...myObject)
# or:
callMe:
  ...myObject
```

It can be done with additional properties:

```coffee
callMe(...myObject1, ...myObject2, property = 23)
```

Just like with types.