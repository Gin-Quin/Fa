const child_process = require('child_process')
const { performance } = require('perf_hooks')
const print = require('cute-print')

const isWindows = (process.platform == "win32")

exports.exec = (command, showStdout=true, showStderr=true) => new Promise((resolve, reject) => {
	child_process.exec(command, (error, stdout, stderr) => {
		if (error) return reject(error)
		stdout = stdout.trim()
		stderr = stderr.trim()
		if (showStdout && stdout) console.log(stdout.trim())
		if (showStderr && stderr) console.error(stderr.trim())
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
		let timerIcon = isWindows? '' : '⏱  '
		print `[white.italic]  [-- ${timerIcon}[reset.italic]${~~deltaTime}[white]  ms --]`
		deltaTime = -1
	}
}