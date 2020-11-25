

// small utility to store growing code
module.exports = class Stack {
   value = ''
   indent = 0
   baseIndent = 0
   enders = []

   constructor(baseIndent) {
      this.baseIndent = baseIndent
   }

   push(value) {
      this.value += ('\t'.repeat(this.baseIndent + this.indent)) + value + '\n'
      return this
   }

   consume() {
      this.closeAllBlocks()
      let result = this.value + '\n'
      this.value = ''
      return result
   }

   newBlock(start, end) {
      this.push(start)
      this.enders.push(end)
      this.indent++
      return this
   }

   closeBlock() {
      if (this.indent) {
         this.indent--
         this.push(this.enders.pop())
      }
      return this
   }

   closeAllBlocks() {
      while (this.indent)
         this.closeBlock()
      return this
   }
}
