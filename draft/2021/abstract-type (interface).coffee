# I often found myself in the situation where I declare those kind of entities:
# 1. A "BaseNode" which is an abstract class that defines all properties and methods the children classes must implement
# 2. Many children classes that extends "BaseNode" (for example, "Node1", "Node2" and "Node3")
# 3. A "Node" class that is a constant enumerable union of all children classes: "Node" = "Node1" | "Node2" | "Node3"

# In Fa, an abstract class is named an "interface"
# An interface has automatically the type of the union of all its children
# If no child is defined, it has its actual type


# An example of solution:
# /Node
# /Node/Node.fa
# /Node/Node1.fa
# /Node/Node2.fa
# /Node/Node3.fa


# /Node/Node.fa:
export default interface =  # we call it an "interface" instead of a type because it needs to be implemented by a static number of children
	unique type: string # we tell it is unique to make sure two children don't have the same value. A unique must be a constant.
	children: [...Self] = []

# /Node/Node1.fa
import Node
export default type implements Node =
	type = "Node1" # an error is raised if a constant value is not initiailized

# /Node/Node2.fa
import Node
export default type implements Node = 
	type = "Node2"


# Then if I write this:
let node: Node

# It is the same as if I write this:
let node: Node1 | Node2

# at the condition that Node1 and Node2 has been previously imported (the order of imports matter)
# maybe I can add "lazy" imports that resolve at the end so that it can be added to the main interface:
import lazy Node1 # bring import to scope, but the import is not necessary for this module
import lazy Node2
# or:
defer import Node1
defer import Node2
# or maybe "defer" or "lazy" is unnecessary
# that sounds like a great combination
