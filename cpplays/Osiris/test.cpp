#include "../main.hpp"
#include "Osiris.hpp"


struct Zabu {
	string name;
	// Box<Zabu> buddy;

	Zabu(string _name="coco") {
		name = _name;
		print("Zabu", name, "created");
	}
	~Zabu() {
		print("Zabu", name, "deleted");
	}
};


int main()
{
	Box<Zabu> z = new Zabu("Yoto");  // constructor
	z = new Zabu("Caca");  // assignment
	Box<Zabu> z2 = z;

	return 0;
}