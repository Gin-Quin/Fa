const child_process = require('child_process')
const { performance } = require('perf_hooks')
const print = require('cute-print')

exports.exec = command => new Promise((resolve, reject) => {
	child_process.exec(command, (error, stdout, stderr) => {
		if (error) return reject(error)
		stdout = stdout.trim()
		stderr = stderr.trim()
		if (stdout) console.log(stdout.trim())
		if (stderr) console.error(stderr.trim())
		resolve()
	})
})


let startTime, deltaTime = -1
exports.timer = {
	start() {
		startTime = performance.now()
	},

	stop() {
		deltaTime = performance.now() - startTime
	},

	print() {
		if (deltaTime = -1) this.stop()
		print `[white.italic]  [-- ⏱  [reset.italic]${~~deltaTime}[white]  ms --]`
		deltaTime = -1
	}
}