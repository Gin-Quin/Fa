# Type mapping

Real-world example of type mapping: we create one column for each field in an object.

```rs
type Column(Type: Object) = {
  iterate() {

  }
}

type Columnar(Item: Object) = {
  [readable Field in Fields(Item)] = Column(Object[Field])

  iterate() {

  }
}

```
