use crate::tokens::TokenKind;

#[inline]
pub(crate) fn is_stop_token(stop_at: &[TokenKind], kind: TokenKind) -> bool {
	stop_at.contains(&kind)
}

/// Return true if the token can start an expression.
pub(crate) fn can_start_expression(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::Integer
			| TokenKind::NegativeInteger
			| TokenKind::BinaryInteger
			| TokenKind::NegativeBinaryInteger
			| TokenKind::OctalInteger
			| TokenKind::NegativeOctalInteger
			| TokenKind::HexadecimalInteger
			| TokenKind::NegativeHexadecimalInteger
			| TokenKind::BigInteger
			| TokenKind::NegativeBigInteger
			| TokenKind::Number
			| TokenKind::True
			| TokenKind::False
			| TokenKind::String
			| TokenKind::None
			| TokenKind::Error
			| TokenKind::Identifier
			| TokenKind::Invalid
			| TokenKind::Return
			| TokenKind::Continue
			| TokenKind::Break
			| TokenKind::Start
			| TokenKind::Type
			| TokenKind::Reactive
			| TokenKind::Derived
			| TokenKind::Namespace
			| TokenKind::Minus
			| TokenKind::DoubleDot
			| TokenKind::DoubleDotEqual
			| TokenKind::TripleDot
			| TokenKind::If
			| TokenKind::For
			| TokenKind::While
			| TokenKind::Loop
			| TokenKind::Do
			| TokenKind::Watch
			| TokenKind::When
			| TokenKind::Export
			| TokenKind::Use
			| TokenKind::Reuse
			| TokenKind::Async
			| TokenKind::Await
			| TokenKind::Try
			| TokenKind::Throw
			| TokenKind::Provide
			| TokenKind::Assert
			| TokenKind::Not
			| TokenKind::Let
			| TokenKind::Function
			| TokenKind::Mutable
			| TokenKind::Static
			| TokenKind::AtFor
			| TokenKind::AtIf
			| TokenKind::AtTest
			| TokenKind::In
			| TokenKind::ParenthesisOpen
			| TokenKind::ParenthesisOpenFunctionDeclaration
			| TokenKind::BracesOpen
			| TokenKind::BracketsOpen
			| TokenKind::GenericOpen
	)
}

/// `(`, whether or not the tokenizer marked it as the start of the parameters of a function.
pub(crate) fn is_parenthesis_open(kind: TokenKind) -> bool {
	matches!(
		kind,
		TokenKind::ParenthesisOpen | TokenKind::ParenthesisOpenFunctionDeclaration
	)
}
