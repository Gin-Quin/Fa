# Module

A `module` is the default file type in `Fa`. All files that :

- have the `.fa` extension,
- and do not have a **capital first letter**,

Unlike a Javascript module, a Fa module can only contain declarations and definitions.

Any code that has to be executed at the module initialization must be placed after the `@start` header.

A `module` can declare and define constants, variables, getters, setters, aliases, types, classes, namespaces.

## Exported values and privacy

Field names that start with a single underscore `_` are **private** : only this

## Examples

#### Basic module

```coffee
# myFile.fa
@use math
@use axios

pi = math.pi
user = "Johnny"
url = "https://twitter.api/getMessages?user={Johnny}"

@start
let x = 121
run Axios.get(url) receive @Response
   print "Received response :"
   print @Response

```
