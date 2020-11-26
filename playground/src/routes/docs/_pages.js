const fs = require('fs')
const frontMatter = require('front-matter')
const marked = require('marked')

const pages = fs.readdirSync('../documentation')
   .filter(file => file.endsWith('.md'))
   .map(file => {
      let path = `../documentation/${file}`
      const content = fs.readFileSync(path, 'utf8')
      // const pageFrontMatter = frontMatter(postContent)

      return {
         title: file.slice(0, -3),
         html: marked(content)
      }
   })


export default pages
