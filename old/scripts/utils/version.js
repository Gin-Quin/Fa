
import fs from 'fs'

const pack = JSON.parse(fs.readFileSync('./package.json', 'utf8'))

let [MAJOR, MINOR, PATCH] = pack.version.split('.')

export default {
	MAJOR,
	MINOR,
	PATCH,
}
