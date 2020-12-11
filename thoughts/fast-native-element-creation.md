
Slow Button constructor :
```js
const Button = document.createElement.bind(document, "button")
let button = Button()
```

Fast Button constructor :
```js
const Button = Object.create.bind(null, HTMLButtonElement.prototype);
let button = Button()
```
