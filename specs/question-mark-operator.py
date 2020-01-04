# The question mark operator have three use cases :

#1: Along with the 'else' keyword it can be used as the ternary conditional operator
x = y > 0 ? 12 else 5312

type = node?.attributes.type?.name else nil

#2: Without the 'else'
x = y > 0 ? 12
# which is exactly the same as :
x = 12  if y > 0


#3: It can indicate when a variable or property is nilable
class Human
	friend? : Human
