import init, { parseFaProgram } from "../pkg/fa.js"

init().then(() => {
	console.time("parseFaProgram")
	const result = parseFaProgram("x = 321")
	console.timeEnd("parseFaProgram")

	console.log(result)
})
