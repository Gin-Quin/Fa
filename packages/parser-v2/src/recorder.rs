pub struct Recorder<T> {
	data: Vec<T>,
}

#[derive(Copy, Clone)]
pub struct Record<T>(*const T);

impl<T> std::ops::Deref for Record<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0 }
	}
}

impl<T: std::fmt::Display> std::fmt::Display for Record<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Display::fmt(&**self, f)
	}
}

impl<T: std::fmt::Debug> std::fmt::Debug for Record<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(&**self, f)
	}
}

impl<T> Recorder<T> {
	pub fn new() -> Self {
		Self { data: Vec::new() }
	}

	pub fn at(&self, index: usize) -> Option<Record<T>> {
		if index > self.data.len() {
			None
		} else {
			Some(Record(&self.data[index] as *const T))
		}
	}

	pub fn push(&mut self, value: T) {
		self.data.push(value);
	}

	// Add return type hint here
	pub fn iter(&self) -> impl Iterator<Item = Record<T>> + ExactSizeIterator + '_ {
		self.data.iter().map(|item| Record(item as *const T))
	}
}
