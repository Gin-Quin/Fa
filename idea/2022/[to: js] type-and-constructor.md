With Fa, I've come to a point where I prefer plain old objects over classes.

For example, the following type in Fa:

```coffee
type Hero =
  name: string
  health = 100
  strength = 10
  
  attack(target: Hero) ->
    target.health -= strength

let heracles = Hero(name = "Heracles", strength = 12)
```

Will be converted into the following Typescript:

```coffee
type Hero = {
  name: string
  health: number
  strength: number
  
  attack(target: Hero): void
}

const $Hero = {
  health: 100,
  strength: 10,
  attack(target) {
    target.health -= this.strength
  }
}


# Option 1: performance (2 billions ops/s ; 1250x faster than #2)
# Chosen option
const ajax = {
  name: "Ajax",
  health: $Hero.health,
  strength: 12,
  attack: $Hero.attack,
}

# Option 2: simplicity (2 millions ops/s ; 99.92% slower)
const heracles = {
  ...$Hero,
  name: "Heracles",
  strength: 12,
}

# Option 3: function constructor (as fast as option #1)
const createHero = (
  name,
  health = 100,
  strength = 10,
  attack = function(target) { target.health -= this.strength }
): Hero => ({
  name,
  health,
  strength,
  attack
});

# Option 4: class (85.6% slower than #1)
class Hero {
  constructor(
    name,
    health = 100,
    strength = 10
  ) {
    this.name = name
    this.health = health
    this.strength = strength
  }

  attack(target) {
    target.health -= this.strength
  }
};
```

Fa will go with option #1, that has several advantages:
- it is very fast,
- you can access default values programmatically.

Option #3 is not bad, so if I need to use a function it is the way to go.