enumeration State
	ready
	set
	go

enumeration HealthStatus
	dead
	alive:
		sick
		healthy

let healthStatus: HealthStatus = dead
let healthStatus: HealthStatus = alive
let healthStatus: HealthStatus = alive { sick }




enumeration State { ready, set, go }


# Field and 
type Animal
	name: String
	speed: Number
	friends: [Animal]

let AnimalFields: Field<Animal> = speed
let AnimalFields: Fields<Animal> = [name, friends] # Set<Field<Animal>>
let AnimalFields: RecursiveFields<Animal> = [speed, friends[name]]
let AnimalFields: RecursiveFields<Animal> =
	- speed
	- name
	- friends:
		- name
