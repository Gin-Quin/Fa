

:Type = CapitalIdentifier | Generic


Let
   case Identifier => identifier
   case Colon::declare(identifier, type)
   case Equal::assign(identifier, type, value)
   # Bad syntax

Colon::declare(identifier, type)
   case Identifier => identifier
      and :Type => type # Invalid type
   # Identifier expected


Equal::assign(identifier, type, value)
   case Identifier => value
      and Any => value
   case Colon::declare(identifier, type)
      and Any => value
   # Bad syntax

Generic
   case CapitalIdentifier
      and :Type # Bad type
   # A type is expected
