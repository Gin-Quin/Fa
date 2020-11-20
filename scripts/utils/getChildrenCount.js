

const incrementers = [
	"Left",
	"Right",
	"WeakLeft",
	"WeakRight",
	"Group",
	// we do not count Body and OptionalBody
]

const infiniteChildren = ({glue}) =>
	!  glue.includes('Single')
	&& glue.includes('Left') || glue.includes('WeakLeft')
	&& glue.includes('Right') || glue.includes('WeakRight')

module.exports = (token) => {
	if (infiniteChildren(token))
		return Infinity

	let count = 0

	for (let glue of token.glue)
		if (incrementers.includes(glue))
			count++
	
	return count
}