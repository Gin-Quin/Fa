#include "../main.hpp"
#include "Osiris.hpp"


struct Zabu {
	string name;
	// Body<Zabu> buddy;

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
	Body<Zabu> z = new Zabu("Yoto");  // constructor
	z = new Zabu("Caca");  // assignment
	Body<Zabu> z2 = z;

	return 0;
}