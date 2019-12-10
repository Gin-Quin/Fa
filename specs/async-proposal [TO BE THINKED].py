
# Instead of working with await/async :
# we can say every function is AUTOMATICALLY async if it calls an async function
# and any call is AUTOMATICALLY awaited
# each function has an "asynchronous" boolean tag which indicates its nature

# except if we use one keyword such as start, execute, call, spawn or run (to define)

# start seems to be the best choice (goes well with 'onFinish')

start readFile "Zabu"
on Data  # proposition : 'on' est une commande et 'Data' un type : on 
	print data
on Error
	print error

# Lors de la conversion en JS, le mot-clé "start" va empêcher l'utilisation de "await" :
F = readFile("Zabu");
F.then(fileData => {
	console.log(fileData);
});
F.catch(err => {
	console.error(err);
});

# En vrai JS :
readFile("Zabu")
.then(fileData => {
	console.log(fileData)
})
.catch(err => {
	console.error(err)
})


# sinon, ça aurait donné :
let content = readFile("Zabu")
print content

# JS
let content = (await readFile("Zabu"));
console.log(content);
