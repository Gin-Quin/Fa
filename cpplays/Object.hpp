


template <typename T>
struct CloudObject {
	T *trueObject;
	void **owners;
	unsigned int count;
	unsigned int currentOwners;
	unsigned int maxOwners;

	CloudObject(T *_trueObject=NULL) {
		trueObject = _trueObject;
		count = 1;
	}

	~CloudObject() {
		delete trueObject;
	}
};



template <typename T>
struct Object {
	CloudObject<T> *cloudObject;

	~Object() {
		print("One less object");
		if (--cloudObject->count == 0) {
			print("Deleted...");
			delete cloudObject;
		}
	}

	// create a fa object from a newly created object
	Object(T* newObject) {
		print("One more object !");
		cloudObject = new CloudObject<T>(newObject);
	}

	// create a fa object from another object
	// Object(Object<T>& object) {
		
	// }

	// create a fa object from another object
	Object<T> operator=(T* newObject) {
		print("One more object !");
		cloudObject = new CloudObject<T>(newObject);
	}


	T* operator->() {
		return cloudObject->trueObject;
	}
};

