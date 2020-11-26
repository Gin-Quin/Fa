import pages from './_pages.js'

const contents = JSON.stringify(pages)

export function get(req, res) {
	res.writeHead(200, {
		'Content-Type': 'application/json'
	})

	res.end(contents)
}
