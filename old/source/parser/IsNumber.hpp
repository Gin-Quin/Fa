/**
* Class used to detect if a character string is a number.
* It does not check directly a full string, but rather gets character after character.
*/

struct IsNumber {
	int length { 0 };
	int base { 10 };
	char lastInsertedCharacter { 0 };
	bool isNumber { true };
	bool exponential { false };  // true if an 'e' appeared
	bool decimal { false };  // true if a '.' appeared

	inline IsNumber& notNumber() {
		isNumber = false;
		return *this;
	}

	IsNumber& operator << (char c) {
		if (!isNumber)
			return *this;
		
		if (c == '.') {
			if (exponential || decimal)
				return notNumber();
			decimal = true;
		}

		else if (c == 'e') {
			if (exponential || length == 0)
				return notNumber();
			exponential = true;
		}

		else if (base == 10) {
			if (c == 'x') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 16;
			}
			else if (c == 'b') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 2;
			}
			else if (c == 'o') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 8;
			}
			else if (c < '0' || c > '9')
				return notNumber();
		}

		else if (base == 2) {
			if (c != '0' && c != '1')
				return notNumber();
		}

		else if (base == 8) {
			if (c < '0' || c > '7')
				return notNumber();
		}

		else if (base == 16) {
			if ((c < '0' || c > '9') && (c < 'A' || c > 'F') && (c < 'a' || c > 'f'))
				return notNumber();
		}

		lastInsertedCharacter = c;
		length++;
		return *this;
	}

	operator bool() {
		return isNumber;
	}

	void reset() {
		length = 0;
		base = 10;
		lastInsertedCharacter = 0;
		isNumber = true;
		exponential = false;
		decimal = false;
	}
};
