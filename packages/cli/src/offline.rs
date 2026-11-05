//! Resolving the dependencies without the network, for an editor: the
//! sources come from the lockfile and the cache only, and a source that
//! would be fetched (`git clone`, `git fetch`, `curl`, a `file://` read)
//! fails at once instead. The editor then resolves on another thread.

use std::cell::Cell;

thread_local! {
	/// `None` when this thread may use the network; inside [`offline`],
	/// whether a source had to be fetched.
	static OFFLINE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Runs `operation` on this thread without the network. Also returns
/// whether a source had to be fetched (its error is then not the answer:
/// the resolution needs the network).
pub(crate) fn offline<T>(operation: impl FnOnce() -> T) -> (T, bool) {
	/// Leaves the offline mode, even after a panic.
	struct Online;
	impl Drop for Online {
		fn drop(&mut self) {
			OFFLINE.set(None);
		}
	}
	OFFLINE.set(Some(false));
	let online = Online;
	let result = operation();
	let fetched = OFFLINE.get() == Some(true);
	drop(online);
	(result, fetched)
}

/// Fails when the network is refused on this thread ([`offline`]); called
/// before fetching a source.
pub(crate) fn network() -> Result<(), String> {
	if OFFLINE.get().is_none() {
		return Ok(());
	}
	OFFLINE.set(Some(true));
	Err("error: the source is not in the cache, and the network is not used here".to_string())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_network_is_refused_only_inside_offline() {
		assert!(network().is_ok());
		let (result, fetched) = offline(|| 1);
		assert_eq!((result, fetched), (1, false));
		let (result, fetched) = offline(network);
		assert!(result.is_err() && fetched);
		assert!(network().is_ok());
		// a panic leaves the offline mode
		let _ = std::panic::catch_unwind(|| offline(|| panic!("stop")));
		assert!(network().is_ok());
	}
}
