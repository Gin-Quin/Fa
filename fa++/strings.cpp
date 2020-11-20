#include <string>
#include "../String.hpp"
#include "../MyString.hpp"
#include "../Timer.hpp"
#include <iostream>


using namespace std;


int main() {
	// /*
	cout << string("Ç").size() << endl;

	return 1;

	//*/

	Timer timer;
	unsigned int iterations = 10000;


	// 1. Constant initialisation
	cout << endl << "------- Constant initialisation -------" << endl;

	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		string std = "Hello you";
	}
	timer.stop("Standard", iterations);

	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		String s = "Hello you";
	}
	timer.stop("Augmented", iterations);

	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		MyString s = "Hello you";
	}
	timer.stop("My", iterations);


	// 2. Copy
	// iterations = 3;

	cout << endl << "------- Copy -------" << endl;
	MyString s = "Hello";
	s += MyString(" you, how are you my dear fellow??");
	// cout << s.__capacity << endl;
	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		MyString s_copy = s;
		// s[0] = s[0] == 'Z' ? s[0] + 1 : 'A';
		// s_copy[0] = s_copy[0] == 'Z' ? s_copy[0] + 1 : 'A';
		// cout << s.__data.buffer << endl << s_copy.__data.buffer << endl;
		// cout << s.data() << endl << s_copy.data() << endl;
	}
	timer.stop("My", iterations);

	string std = "Hello you, how are you my dear fellow??";
	// cout << std.capacity() << endl;
	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		string std_copy = std;
		// std[0] = std[0] == 'Z' ? std[0] + 1 : 'A';
		// std_copy[0] = std_copy[0] == 'Z' ? std_copy[0] + 1 : 'A';
		// cout << (void*) std.data() << endl << (void*) std_copy.data() << endl;
	}
	timer.stop("Standard", iterations);

	String agm = "Hello you, how are you my dear fellow??";
	// cout << std.capacity() << endl;
	timer.start();
	for (unsigned int i=0; i < iterations; i++) {
		String agm_copy = agm;
		// agm[0] = agm[0] == 'Z' ? agm[0] + 1 : 'A';
		// agm_copy[0] = agm_copy[0] == 'Z' ? agm_copy[0] + 1 : 'A';
		// cout << (void*) std.data() << endl << (void*) std_copy.data() << endl;
	}
	timer.stop("Augmented", iterations);


	cout << endl;
	return 0;
}