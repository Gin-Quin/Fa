//! Strings and compile-time string constants. Well-formed text keeps UTF-8
//! storage and a cached UTF-16 length; only malformed text stores raw units.
//! No operation may use a lossy conversion except at an output boundary.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

/// A sequence of UTF-16 code units, including lone surrogates.
#[derive(Clone, Debug)]
pub struct Text(Repr);

#[derive(Clone, Debug)]
enum Repr {
	Utf8 { text: Arc<str>, length: usize },
	Units(Arc<[u16]>),
}

impl From<&str> for Text {
	fn from(text: &str) -> Self {
		Self::from(Arc::<str>::from(text))
	}
}

impl From<String> for Text {
	fn from(text: String) -> Self {
		Self::from(Arc::<str>::from(text))
	}
}

impl From<Arc<str>> for Text {
	fn from(text: Arc<str>) -> Self {
		let length = if text.is_ascii() {
			text.len()
		} else {
			text.encode_utf16().count()
		};
		Self(Repr::Utf8 { text, length })
	}
}

impl Text {
	/// Internal producers already know the length; keep ASCII copies and
	/// concatenations free of a second scan of their new buffer.
	pub(crate) fn from_utf8(text: Arc<str>, length: usize) -> Self {
		Self(Repr::Utf8 { text, length })
	}

	/// Canonicalizes a sequence: well-formed strings always take the UTF-8 path.
	pub fn from_units(units: &[u16]) -> Self {
		match String::from_utf16(units) {
			Ok(text) => text.into(),
			Err(_) => Self(Repr::Units(Arc::from(units))),
		}
	}

	/// Ordinary text, without copying. `None` means lone surrogates, never a
	/// replacement character. Emitters must handle this case explicitly.
	pub fn as_str(&self) -> Option<&str> {
		match &self.0 {
			Repr::Utf8 { text, .. } => Some(text),
			Repr::Units(_) => None,
		}
	}

	/// UTF-16 code-unit length, in constant time.
	pub fn len(&self) -> usize {
		match &self.0 {
			Repr::Utf8 { length, .. } => *length,
			Repr::Units(units) => units.len(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	pub fn is_ascii(&self) -> bool {
		matches!(&self.0, Repr::Utf8 { text, length } if text.len() == *length)
	}

	pub fn is_well_formed(&self) -> bool {
		self.as_str().is_some()
	}

	/// The UTF-8 encoding length, with U+FFFD for each lone surrogate.
	pub fn utf8_len(&self) -> usize {
		match &self.0 {
			Repr::Utf8 { text, .. } => text.len(),
			Repr::Units(units) => char::decode_utf16(units.iter().copied())
				.map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER).len_utf8())
				.sum(),
		}
	}

	/// Output/encoding only: never use this to emit a string constant.
	pub fn to_utf8_lossy(&self) -> Cow<'_, str> {
		match &self.0 {
			Repr::Utf8 { text, .. } => Cow::Borrowed(text),
			Repr::Units(units) => Cow::Owned(String::from_utf16_lossy(units)),
		}
	}

	/// The units of a constant, without constructing an intermediate buffer.
	pub fn units(&self) -> impl Iterator<Item = u16> + Clone + '_ {
		let (text, units) = match &self.0 {
			Repr::Utf8 { text, .. } => (Some(text.encode_utf16()), None),
			Repr::Units(units) => (None, Some(units.iter().copied())),
		};
		text.into_iter()
			.flatten()
			.chain(units.into_iter().flatten())
	}

	pub fn unit(&self, index: usize) -> Option<u16> {
		if self.is_ascii() {
			return self.as_str()?.as_bytes().get(index).map(|b| u16::from(*b));
		}
		match &self.0 {
			Repr::Utf8 { text, .. } => text.encode_utf16().nth(index),
			Repr::Units(units) => units.get(index).copied(),
		}
	}

	/// Bounds have already been checked by the evaluator.
	pub fn slice(&self, start: usize, end: usize) -> Self {
		if start == 0 && end == self.len() {
			return self.clone();
		}
		if self.is_ascii() {
			return Self::from_utf8(
				Arc::from(&self.as_str().expect("ASCII")[start..end]),
				end - start,
			);
		}
		match &self.0 {
			Repr::Utf8 { text, .. } => {
				let units: Vec<u16> = text.encode_utf16().skip(start).take(end - start).collect();
				Self::from_units(&units)
			}
			Repr::Units(units) => Self::from_units(&units[start..end]),
		}
	}

	/// A sequential code-point cursor. `offset` is a storage offset, while
	/// the returned width is the public index increment in code units.
	pub fn code_point_count(&self) -> usize {
		match &self.0 {
			Repr::Utf8 { text, .. } => {
				if self.is_ascii() {
					text.len()
				} else {
					text.chars().count()
				}
			}
			Repr::Units(units) => char::decode_utf16(units.iter().copied()).count(),
		}
	}

	/// A scalar cursor, retaining the numerical value of a lone surrogate.
	pub fn next_scalar(&self, offset: &mut usize) -> Option<u32> {
		match &self.0 {
			Repr::Utf8 { text, .. } => {
				let c = text.get(*offset..)?.chars().next()?;
				*offset += c.len_utf8();
				Some(u32::from(c))
			}
			Repr::Units(units) => {
				let first = *units.get(*offset)?;
				*offset += 1;
				if (0xD800..=0xDBFF).contains(&first)
					&& units
						.get(*offset)
						.is_some_and(|u| (0xDC00..=0xDFFF).contains(u))
				{
					let low = units[*offset];
					*offset += 1;
					Some(0x10000 + ((u32::from(first) - 0xD800) << 10) + u32::from(low) - 0xDC00)
				} else {
					Some(u32::from(first))
				}
			}
		}
	}

	pub fn next_point(&self, offset: &mut usize) -> Option<(Self, usize)> {
		match &self.0 {
			Repr::Utf8 { text, .. } => {
				let c = text.get(*offset..)?.chars().next()?;
				let start = *offset;
				*offset += c.len_utf8();
				Some((
					Self::from_utf8(Arc::from(&text[start..*offset]), c.len_utf16()),
					c.len_utf16(),
				))
			}
			Repr::Units(units) => {
				let first = *units.get(*offset)?;
				let width = usize::from(
					(0xD800..=0xDBFF).contains(&first)
						&& units
							.get(*offset + 1)
							.is_some_and(|u| (0xDC00..=0xDFFF).contains(u)),
				) + 1;
				let start = *offset;
				*offset += width;
				Some((Self::from_units(&units[start..*offset]), width))
			}
		}
	}

	pub fn concat(pieces: &[Self]) -> Self {
		if pieces.iter().all(Self::is_well_formed) {
			let size = pieces.iter().map(Self::utf8_len).sum();
			let mut text = String::with_capacity(size);
			for piece in pieces {
				text.push_str(piece.as_str().expect("well-formed"));
			}
			return Self::from_utf8(Arc::from(text), pieces.iter().map(Self::len).sum());
		}
		let units: Vec<u16> = pieces.iter().flat_map(Self::units).collect();
		Self::from_units(&units)
	}

	pub fn find(&self, needle: &Self, reverse: bool) -> Option<usize> {
		if let (Some(text), Some(needle)) = (self.as_str(), needle.as_str()) {
			let byte = if reverse {
				text.rfind(needle)
			} else {
				text.find(needle)
			}?;
			return Some(if self.is_ascii() {
				byte
			} else {
				text[..byte].encode_utf16().count()
			});
		}
		if needle.is_empty() {
			return Some(if reverse { self.len() } else { 0 });
		}
		let mut haystack = Vec::with_capacity(self.len());
		haystack.extend(self.units());
		let mut units = Vec::with_capacity(needle.len());
		units.extend(needle.units());
		let needle = units;
		if reverse {
			haystack.windows(needle.len()).rposition(|w| w == needle)
		} else {
			haystack.windows(needle.len()).position(|w| w == needle)
		}
	}
}

impl PartialEq for Text {
	fn eq(&self, other: &Self) -> bool {
		match (&self.0, &other.0) {
			(Repr::Utf8 { text: a, .. }, Repr::Utf8 { text: b, .. }) => a == b,
			(Repr::Units(a), Repr::Units(b)) => a == b,
			_ => false, // both representations are canonical
		}
	}
}
impl Eq for Text {}
impl PartialOrd for Text {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for Text {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.is_ascii() && other.is_ascii() {
			return self.as_str().cmp(&other.as_str());
		}
		self.units().cmp(other.units())
	}
}

impl fmt::Display for Text {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.to_utf8_lossy())
	}
}
