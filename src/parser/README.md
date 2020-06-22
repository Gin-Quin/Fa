All the code to parse some Fa code is defined here.

The main object is `Parser.hpp`. Its non-trivial methods are defined in the `Parser/` folder.

## Usage

Here is an example of usage :

``` c++
#include "your/path/to/Parser.hpp"

int main() {
  // we pass a string to parse
  Parser parser("x = 12");

  // we tokenize
  parser.tokenize();

  // parser.body();  // <-- the tokens are now stored in here

  // we grow the syntax tree
  parser.growTree();

  // parser.tree;  // <-- the AST is now stored in here

  // we generate json from the AST... 
  std::string json = parser.tree.toJson();
  std::cout << json << std::endl;
  
  return 0;
}
```

