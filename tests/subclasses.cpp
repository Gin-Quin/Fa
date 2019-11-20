
#include <iostream>
#include <string>

using namespace std;

struct Zabu {
	Zabu() {
		cout << "Zabu created" << endl;
	}
	~Zabu() {
		cout << "Zabu deleted" << endl;
	}

	struct Coco {
		Coco() {
			cout << "Coco created" << endl;
		}
		~Coco() {
			cout << "Coco deleted" << endl;
		}
	};
};


int main() {
	Zabu z;
	Zabu::Coco c;
	return 1;
}
