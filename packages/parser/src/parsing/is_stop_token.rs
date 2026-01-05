use crate::tokens::TokenKind;

pub(crate) fn is_stop_token<const STOP_COUNT: usize>(
	stop_at: &[TokenKind; STOP_COUNT],
	kind: TokenKind,
) -> bool {
	stop_at.iter().any(|stop| *stop == kind)
}
