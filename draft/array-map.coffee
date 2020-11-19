


let array = [5, 2, 3].map(value -> )
let array = [5, 2, 3]..
   transformEach value -> value++
   map value, index ->


let array = [5, 2, 3]
|> Array.map


class Array[Type]
   elements:[Type]

   forEach
      for value in elements
         send value

   map<GeneratedType> callback:(Type -> GeneratedType)
      let newArray = Array<GeneratedType>
      for value in elements
         newArray << callback value


let array = Array
array.forEach value -> print value
array.map value -> value + 1

let array: [Integer] -> String



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
