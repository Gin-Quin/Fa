import polka from 'polka'
import path from 'path'
import url from 'url'
// import fs from 'fs'

import serveStaticDirectory from 'serve-static'

const __dirname = path.dirname(url.fileURLToPath(import.meta.url))
const client = path.resolve(__dirname, '..', 'client-svelte', 'public')
console.log("client", client)

const port = 5555


polka()
	.use(serveStaticDirectory(client))

	.get('/compile/:file', (request, response) => {
		const { file } = request.params
		console.log(`file to compile :`, file)
		res.setHeader('Content-Type', 'application/json')
		res.end(JSON.stringify({ file }))
	})

	.listen(port, err => {
		if (err) throw err
		console.log(`> Running on http://localhost:${port}`)
	})

