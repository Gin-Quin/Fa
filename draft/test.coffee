

x = 12321
options:
	pet: Animal = Dog:
		name = "Albert"
	y = 12


class Zabu
	strength: Integer
	coco: Integer
	y = 121


	from num: Number 
		strength = @Number



class Card
	@CardType
	name = ""
	cost = Resource null
	image = ""
	description = ""

class DeckCard extends Self
	@Faction()

class Gain
	minimumStats = UnitStats
	@Action

class UnitPermanentCard extends DeckCard
	@CardType = UnitPermanent
	@Skills
	subtype = PermanentType.Artifact
	class = Class.all
	select: (UnitTargetObject) -> Boolean
	bonus:
		@UnitStats
		@Powers
		

class Document
	on trigger: String receive @Event
	on trigger: String, receive: (@Event)
		

document.on 'click', @ClickEvent ->
	if event.target.tagName.toLowerCase() == 'input'
		event.target.select
