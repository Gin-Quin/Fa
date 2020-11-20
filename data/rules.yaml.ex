#

Let():
   - Identifier(identifier)
   - ':'_declare(identifier type)
   - '='_assign(identifier type value)


'='_assign(identifier type value):
   - [ Identifier_get(identifier), Any(value) ]
   - [ ':'_declare(identifier type), Any(value) ]

':'_declare():
   - [ Identifier(identifier), Type(type) ]

Type():
   - CapitalIdentifier()
   - Generic()
   - throw "Cannot deal with this shit!"

Generic():
   - [ CapitalIdentifier(), Type() ]
   - throw "Cannot deal with this shit!"
