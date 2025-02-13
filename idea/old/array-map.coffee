


let array = [5, 2, 3].map: value => value++
let array = [5, 2, 3]..
   transformEach: value => value++
   map: value => value++

let fruits =
   - "banana"
   - "orange"

let myHeroes =
   -  -  name = "Hercule"
         strength = 121
      -  name = "Coco"
         strength = 12
   -  -  name = "zabu"
         strength = 12


class Array<Item>
   elements: [Item]

   forEach(callback: Item -> Any) ->
      for value in elements
         callback(value)

   map(callback: Item -> Any): [...ReturnType<callback>] ->
      let newArray = Array<MappedItem>
      for value in elements
         newArray.push(callback(value))

let array = Array
array.forEach: value -> print value
array.map: value => value + 1
hero = Hero("Zabu").call: "911"


let children[String] = [5, 12, 13]
let children: [String] = [5, 12, 13]
let children: [...String] = [5, 12, 13]
let children: Array[String] = [5, 12, 13]
let children: Array<String> = [5, 12, 13]
let children: String[] = [5, 12, 13]

let children[String]: String = { hello: "123" }
let children[String: String] = [hello: "123", coco: zabu(12), :zabu]
let children: Map[String, String] = { hello: "123" }
let children: Map<String, String> = { hello: "123" }
let children: [String -> String] = { hello: "123" }
let children: String[] = { hello: "123" }

routes = Route[Number]
routes = [Number] -> Route
routes = [Number -> Route]
routes = [Number : Route](5)
routes = Array[Route](5)
