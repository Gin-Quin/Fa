/**
* A Box is simply an object container.
* Instead of pointing to the object itself, it's pointing to its associated cloud object.
*/
template<typename T>
struct Box {
	T* soul;

	Box() {
		soul = NULL;
	}

	/**
	* Construct from a pointer
	*/
	Box(T* _soul) {
		soul = _soul;
		Osiris::newSoul(soul);
		print("Box Soul :", soul);
	}

	/**
	* Construct from another Box
	*/
	Box(const Box<T>& otherBox) {
		soul = otherBox.soul;
		Osiris::newSoul(soul);
		print("Box Soul :", soul);
	}


	/**
	* Assign from a pointer
	*/
	Box& operator=(T* _soul) {
		Osiris::freeSoul(soul);
		soul = _soul;
		Osiris::newSoul(soul);
		print("Box Soul :", soul);
		return *this;
	}

	/**
	* Assign from another Box
	*/
	Box& operator=(const Box<T> otherBox) {
		Osiris::freeSoul(soul);
		soul = otherBox.soul;
		Osiris::newSoul(soul);
		print("Box Soul :", soul);
		return *this;
	}

	/**
	* Access the soul
	*/
	inline T* operator->() {
		return soul;
	}

	/**
	* Deleting the box will free the soul
	*/
	~Box() {
		Osiris::freeSoul(soul);
	}

	/**
	* Treat the box as a soul
	*/
	operator T*() {
		return soul;
	}
};