
/**
* A CloudObject is a wrapper around a regular object, which keeps the count of variables keeping the reference of that object.
* 1 object = 1 cloud object
*/
template<typename T>
struct CloudObject {
	T* skyObject;
	unsigned long count;

	CloudObject() {
		skyObject = NULL;
		count = 0;
	}

	CloudObject(T *newObject) {
		skyObject = newObject;
		count = 1;
	}

	~CloudObject() {
		print("CloudObject::delete, count = ", count);

		if (--count) {
			// ineffectiveDelete = true;
			// delete skyObject;
			// ineffectiveDelete = false;
		}

		if (!count) {
			delete skyObject;
			// delete this;
		}
	}
};


/**
* A Box is simply an object container.
* Instead of pointing to the object itself, it's pointing to its associated cloud object.
*/
template<typename T>
struct Box {
	CloudObject<T>* cloudObject;

	Box() {
		cloudObject = new CloudObject<T>();
	}

	Box(T* newObject) {
		print("Box::new(T*)");
		cloudObject = new CloudObject<T>(newObject);
	}

	Box(const Box<T>& otherBox) {
		print("Box::new(Box<T>)");
		cloudObject = otherBox.cloudObject;
		otherBox.cloudObject->count++;
	}

	Box& operator=(T* newObject) {
		print("Box::=(T*)");
		delete cloudObject;
		cloudObject = new CloudObject<T>(newObject);
		return *this;
	}

	Box& operator=(Box<T> otherBox) {
		print("Box::=(<box<T>)");
		// delete cloudObject;
		// cloudObject = otherBox.cloudObject;
		// cloudObject->count++;
		// return *this;
	}

	T* operator->() {
		return cloudObject->skyObject;
	}

	~Box() {
		delete cloudObject;
	}

	operator T*() {
		return cloudObject->skyObject;
	}
	// explicit operator T*() {
	// 	return cloudObject->skyObject;
	// }
};