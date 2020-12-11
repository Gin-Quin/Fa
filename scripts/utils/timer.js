
const { performance } = require('perf_hooks')
const print = require('cute-print')
const isWindows = (process.platform == "win32")


let startTime
let deltaTime = -1

module.exports = {
	start() {
		startTime = performance.now()
	},

	stop() {
		deltaTime = performance.now() - startTime
	},

	print() {
		if (deltaTime = -1) this.stop()
		let timerIcon = isWindows? '' : '⏱  '
		print `[white]  ── ${timerIcon}[reset]${~~deltaTime}[white]  ms ──`
		deltaTime = -1
	}
}
