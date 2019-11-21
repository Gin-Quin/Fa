# Opérateurs
La plupart des opérateurs suivent les règles de base des autres langages de programmation, on ne parlera ici que des opérateurs spécifiques à Fa ou à des utilisations particulières.

## Addition
L'opérateur d'addition est également celui de concaténation. On peut concaténer :

- des chaînes de caractères,
- des tableaux,
- des listes.

Exemples :

```

```

## Méthodes & Parenthèses
Les parenthèses sont des opérateurs spéciaux qui peuvent avoir plusieurs sens selon le contexte ils sont utilisés.

Dans un contexte de *déclaration*, les parenthèses en leftValue définissent une nouvelle méthode :

```
class Zabu
	moo()
		print 'Zabuuuuu!'

	name() -> String
		return 'Zabuuuuu!'

	--- format minimal : pas d'arguments et la valeur de retour est déduite
	name -> 'Zabuuuuu!'
```


Il n'est pas possible de définir une méthode grâce à l'opérateur *=* comme ceci :


```
class Zabu
	moo = () ->
		print 'Zabuuuuu!'
		return "That was good"
```

Car c'est contre la philosophie de Fa. Fa valorise l'utilisation de promesses joliment chaînées et l'utilisation de callbacks grâce au mot-clé "receive"



# Callbacks & promesses

## Callbacks

```
class Button
	on(trigger:String, action:(Event))
		e = Event
		action(e)

myButton = Button
myButton.on "click" receive event
	print "You clicked!"

```


## Promesses
Les promesses sont des fonctions qui lors de la création

Les promesses permettent d'éviter un enchaînement de callbacks qui créé une pyramide d'indentation.

```
promise = Promise(resolve:(), reject:())
	/- do a thing, possibly async, then… -/
	if (/- everything turned out fine -/)
		resolve("Stuff worked!");
	else
		reject(Error("It broke"));
```