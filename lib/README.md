The Fa library is one `fa.hpp` file that can be directly imported into your C++ project.


## Usage

``` c++
#include "lib/fa.hpp"

int main() {
  // we pass a string to parse
  Fa::Parser parser("x = 12");

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

