


let array = [5, 2, 3].map(value -> value++)
let array = [5, 2, 3]..
   transformEach(value -> value++)
   map(value -> value++)

let fruits:
  > 'banana'
  > 'orange'

<< 'banana'
let fruits:
   - "banana"
   - "orange"


let myHeroes:
   >  >  name = "Hercule"
         strength = 121
      >  name = "COco"
         strength = 12
   >  >  name = "zabu"
         strength = 12

let myHeroes:
   -  -  name = "Hercule"
         strength = 121
      -  name = "COco"
         strength = 12
   -  -  name = "zabu"
         strength = 12


let array = [5, 2, 3]
|> Array.map


class Array<Type>
   elements:[Type]

   forEach(callback: Type -> None)
      for value in elements
         callback(value)

   map<NewType>(callback: Type -> NewType)
      let newArray = Array<NewType>
      for value in elements
         newArray << callback(value)


let array = Array
array.forEach value -> print value
array.map value -> value + 1



let children[]:String = [5, 12, 13]
let children:[String] = [5, 12, 13]
let children:String[] = [5, 12, 13]

let children[String]:String = { hello: "123" }
let children:Map[String, String] = [5, 12, 13]
let children:[String -> String] = [5, 12, 13]
let children:String[] = [5, 12, 13]



routes[]:Route

routes = Route[Number]
routes = [Number] -> Route
routes = [Number -> Route]
routes = [Number : Route](5)
routes = Array[Route](5)

Hash[Number : String]
Hash[String]
[String]
