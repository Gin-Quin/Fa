

x = 12321
options = 
	pet: Animal = Dog:
		name = "Albert"
	y = 12


type Zabu =
	strength: Integer
	coco: Integer
	y = 121

	from(number: Number)
		strength = number

type Card =
	type: CardType
	name = ""
	cost = Resource null
	image = ""
	description = ""

type DeckCard =
	...Card
	faction: Faction

type Gain =
	minimumStats = UnitStats
	action: Action

type UnitPermanentCard =
	...DeckCard
	type = CardType.UnitPermanent
	skills: Skills
	subtype = PermanentType.Artifact
	class = Class.all
	select: (target: UnitTargetObject) -> Boolean
	bonus:
		stats: UnitStats
		powers: Powers

type Document =
	on: (trigger: String, event: Event) -> void
		

document.on:
	event = 'click'
	action = event ->
		if event.target.tagName.toLowerCase() == 'input'
			event.target.select

document.on('click'): event ->
	if event.target.tagName.toLowerCase() == 'input'
		event.target.select
