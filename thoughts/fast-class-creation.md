Using class properties is actually super slow :

```js
class Zabu {
	x = 12
}
```


When generating js we should use a constructor function :
```js
class Zabu {
	constructor() {
		this.x = 12
	}
}
```
