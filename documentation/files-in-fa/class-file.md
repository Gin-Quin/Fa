# File as a class

In other OOP languages, it is a common patter to have a file that define one single class. This pattern creates a redundance between the file name and the class name, which are usually the same. If you change one, you change the other.

A typical example could be a **React component** :

```tsx
// Calendar.tsx
export default class Calendar extends React.Component {
   ...
}
```

The name of the file matches the name of the component. Furthermore, you repeat the name again when you import it :

```tsx
import Calendar from './Calendar'
```

So the day when you realize you should not name your component `Calendar` but `MiniCalendar`, you have to replace `Calendar` in four places :

- the class name,
- the file name,
- the import file name,
- the import value.

To make things straightforward and avoid useless repetitions, `Fa` makes a distinction between files which have a capital first letter or not.

Files that have a capital first letter are **class files**.

Files that do not have a capital first letter are **module files**.


- **Basic example**

   Let's suppose we have two files `main.fa` and `Pet.fa` in the same folder.

   Then we can import and instantiate a new `Pet` with zero effort :

   ```coffee
   # Pet.fa
   export class
      name: String
      specy: String
      speed: Number
   ```

   Because the file name `Pet.fa` starts with a capital letter, the `Fa` compiler understands the file describes a class that can be instantiated.

   <br>


   ```coffee
   # main.fa
   import Pet

   let charlie = Pet:
      name: "Charlie"
      specy: "dog"
      speed: 125
   ```

- **Inheritance and traits**

   A `class file` can inherit a `parent class` and have `traits`. For doign so, we use the keywords `@is` (superclass) and `@with` (traits) at the top-level, with the same syntax as an `import` :

   ```coffee
   # Pet.fa
   name: String
   specy: String
   speed: Number
   ```

   ```coffee
   # Dog.fa
   import Pet
   export class is Pet
      specy = "dog"
      speed = 120
      legs = 4  # a new property
   ```

   ```coffee
   import Dog

   let charlie = Dog(name: "Charlie", speed: 125, legs: 3)
   ```
