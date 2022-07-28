
import child_process from 'child_process'


export default (command, showStdout=true, showStderr=true) => new Promise((resolve, reject) => {
	child_process.exec(command, (error, stdout, stderr) => {
		if (error) return reject(error)
		stdout = stdout.trim()
		stderr = stderr.trim()
		if (showStdout && stdout) console.log(stdout.trim())
		if (showStderr && stderr) console.error(stderr.trim())
		resolve()
	})
})


