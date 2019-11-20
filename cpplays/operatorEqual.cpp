#include "main.hpp"


struct FaObject {
	static int instance;
	int id;

	int operator=(FaObject &b) {
		print("Assigning !", id, b.id);
		return 5;
	}
	FaObject() {
		id = instance++;
		print("Creating FaObject", id);
	}
	~FaObject() {
		print("Destructing FaObject", id);
	}
};


struct Zabu {
	FaObject f;
};

int FaObject::instance = 0;


int main() {
	print(sizeof(vector<FaObject>));
}