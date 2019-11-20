#include "main.hpp"
#include "Box.hpp"


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

struct SuperZabu : public Zabu {
	SuperZabu(string _name="coco") {
		name = _name;
		print("I'm super");
	}
};


// void operator delete(void *p) throw() {
	// print("!!!!");
	// if (ineffectiveDelete) {
	// 	print("ineffective delete !");
	// }
	// // else {
	// 	print("effective delete !");
	// 	free(p);
	// }
// }


int main()
{
	// Box<Zabu> z = new Zabu("yoto");  // constructor
	// z = new Zabu("caca");  // assignment
	// Box<Zabu> z2 = z;

	// SuperZabu z("yototo");
	// Zabu x = SuperZabu::Zabu("zinzin");

	vector<Zabu*> v;
	v.push_back(Box<Zabu>(new Zabu("Miii Zabu! :3")));
	v.push_back(Box<SuperZabu>(new SuperZabu("SuperMiii :D")));

	print("<<end>>");
	return 0;
}

