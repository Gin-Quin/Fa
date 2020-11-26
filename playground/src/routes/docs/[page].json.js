import pages from './_pages.js'

const lookup = new Map()
pages.forEach(page => {
	lookup.set(page.title, JSON.stringify(page))
})

export function get(req, res, next) {
	const { page } = req.params

	if (lookup.has(page)) {
		res.writeHead(200, {
			'Content-Type': 'application/json'
		})

		res.end(lookup.get(page))
	} else {
		res.writeHead(404, {
			'Content-Type': 'application/json'
		})

		res.end(JSON.stringify({
			message: `Not found`
		}))
	}
}
