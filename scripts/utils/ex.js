/**
* Replace all 'token shortcuts' in a string
* Ex : `'='` will be transform into `Equal`
*/

const safeString = str => str.split('').map(c => '\\' + c).join('')

const ex = ({symbols, keywords}, content) => {
	for (let {key, token} of symbols)
		content = content.replace(RegExp(`'${safeString(key)}'`, 'g'), token)
	for (let {key, token} of keywords)
		content = content.replace(RegExp(`'${safeString(key)}'`, 'g'), token)
	return content
}


export default ex