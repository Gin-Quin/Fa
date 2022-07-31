type FaNodeKind* = enum
  # Literals
  Null
  BooleanLiteral
  IntegerLiteral
  NumberLiteral
  StringLiteral
  Identifier

  # Declarations
  VariableDeclaration
  # FunctionDeclaration
  # TypeDeclaration
  # TypeAliasDeclaration

  # Operations
  Operation
  RightOperation
  LeftOperation
