#include "main.hpp"

struct Zabu {
	string name;
	int value;
	static int count;

	Zabu() {
		name = "zabu-" + to_string(count);
		value = count++;
	}
	Zabu(string _name, int _value=0) {
		name = _name;
		value = _value;
	}
};

int Zabu::count = 0;


Zabu* find(vector<Zabu*> v, string name) {
	for (auto elt : v) if (elt->name == name)
		return elt;
	return NULL;
}


int main() {
	vector<Zabu*> v;
	map<string, Zabu*> m;
	auto timer = Timer();

	for (int x=0; x < 10; x++) {
		auto z = new Zabu();
		print(z->name, z->value);
		v.push_back(z);
		m[z->name] = z;
	}

	// print("zabu-2 :", find(v, "zabu-2")->value);

	// vector find
	timer.start();
	for (int x=0; x < 100000; x++) {
		find(v, "zabu-2");
	}
	timer.print("Vector");

	// map find
	timer.start();
	for (int x=0; x < 100000; x++) {
		m["zabu-2"];
	}
	timer.print("Map");



	v.clear();
	m.clear();
	return 0;
}