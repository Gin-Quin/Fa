#include "main.hpp"
#include "Object.hpp"

struct Zabu {
	string name;
	int x, y;

	Zabu(string _name="coco") {
		name = _name;
		print("Zabu", name, "created");
	}
	~Zabu() {
		print("Zabu", name, " deleted");
	}
};

int main() {
	Object<Zabu> z = new Zabu();
	z->x = 1321;
	return 0;
}