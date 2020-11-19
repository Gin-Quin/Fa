
(LeftAtom)
'Identifier'
'['

(LeftValue)
LeftAtom
'.'::leftValue

[Let]
Identifier :identifier
':'::declare :identifier :type
'='::assign :identifier :type :value


['=']::assign
Identifier :identifier, Any :value
':'::declare :identifier :type, Any :value

[':']::declare
Identifier :identifier, Type :type

[Type]
CapitalIdentifier
'<>'
'::'
raise "Cannot deal with this shit!"

['<>']
CapitalIdentifier, Type
raise "Cannot deal with this shit!"

['::']
CapitalIdentifier*, Type
