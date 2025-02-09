

x = 12321
options:
	pet: Animal = Dog:
		name = "Albert"
	y = 12


class Zabu
	strength: Int = null
	y = 121


	from num:Number
		strength = @Number



class Card
	type: CardType
	name = ""
	cost = new Resource
	image = ""
	description = ""

class DeckCard is Card
	faction : Faction = 0

class Gain
	minimumStats = UnitStats
	action = Action

class UnitPermanentCard is DeckCard
	type = CardType.UnitPermanent
	subtype = PermanentType.Artifact
	class = Class.all
	skills = Skills
	select: (UnitTargetObject) -> Boolean
	bonus:
		stats = UnitStats
		powers = Powers


class Document
	on(trigger: String) receive @Event
	on(trigger: String, receive: (@Event))


document.on('click') receive @ClickEvent
	if event.target.tagName.toLowerCase() == 'input'
		event.target.select()
