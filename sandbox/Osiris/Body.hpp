/**
* A Body is simply an object container.
*/
template<typename T>
struct Body {
	T* soul;

	Body() {
		soul = NULL;
	}

	/**
	* Construct from a pointer
	*/
	Body(T* _soul) {
		soul = _soul;
		Osiris::newSoul(soul);
		print("Body Soul :", soul);
	}

	/**
	* Construct from another Body
	*/
	Body(const Body<T>& otherBody) {
		soul = otherBody.soul;
		Osiris::newSoul(soul);
		print("Body Soul :", soul);
	}


	/**
	* Assign from a pointer
	*/
	Body& operator=(T* _soul) {
		Osiris::freeSoul(soul);
		soul = _soul;
		Osiris::newSoul(soul);
		print("Body Soul :", soul);
		return *this;
	}

	/**
	* Assign from another Body
	*/
	Body& operator=(const Body<T> otherBody) {
		Osiris::freeSoul(soul);
		soul = otherBody.soul;
		Osiris::newSoul(soul);
		print("Body Soul :", soul);
		return *this;
	}

	/**
	* Access the soul
	*/
	inline T* operator->() {
		return soul;
	}

	/**
	* Deleting the Body will free the soul
	*/
	~Body() {
		Osiris::freeSoul(soul);
	}

	/**
	* Treat the Body as a soul
	*/
	operator T*() {
		return soul;
	}
};