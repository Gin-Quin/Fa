#include "../common.hpp"
#include "../walker/classes/Type.hpp"


int main() {
	Type t, t2;
	Type t3;
	Type t4 = t;
	Type t5;
	t3.subs = {t, t2};
	cout << t.id << endl;
	cout << t2.id << endl;
	cout << t3.id << endl;
	cout << t4.id << endl;
	cout << t5.id << endl;
	return 0;
}