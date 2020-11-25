
const { tokens } = require('./data')

const incrementers = [
	"Left",
	"Right",
	"WeakLeft",
	"WeakRight",
	"Group",
	// we do not count Body and OptionalBody
]

const cache = {}

const infiniteChildren = ({glue}) =>
	!  glue.includes('Single')
	&& glue.includes('Left') || glue.includes('WeakLeft')
	&& glue.includes('Right') || glue.includes('WeakRight')

const getChildrenCount = (tokenName) => {
	let token = tokens.find(t => t.name == tokenName)
	if (!token || !token.glue) return 0
	if (tokenName in cache)
		return cache[tokenName]

	if (infiniteChildren(token))
		return Infinity

	let count = 0
	for (let glue of token.glue)
		if (incrementers.includes(glue))
			count++
	cache[tokenName] = count
	return count
}

if (require.main === module) {
	for (let token of tokens)
		console.log(token.name, getChildrenCount(token.name))
}



module.exports = getChildrenCount
