import std/memfiles

type
  Line* = ref object
    slice*: MemSlice # need to convert this into an OpenArray
    subLines*: seq[Line]
