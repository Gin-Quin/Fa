
namespace Validate {
   void This(Node* node);
   void Any(Node* node);
   ${generateRules()}

   void This(Node* node) {
      switch (node->type()) {
         ${ Object.keys(rules)
            .filter(key => !key.includes("__"))
            .map(key => `case Token::${key}: return Validate::${key}(node);`)
            .join('\n\t\t\t')
         }
         default: return Validate::Any(node);
      }
   }

   void Any(Node* node) {
      for (Node* node : node->children)
         Validate::This(node);
   }
}
