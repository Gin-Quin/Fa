#include <iostream>
#include <string>
#include <map>
#include <vector>
#include <chrono>
#include <ctime>
#include <memory>

using namespace std;

using Clock = std::chrono::high_resolution_clock;

struct Timer {
	chrono::_V2::system_clock::time_point startTime;

	Timer& print(string msg) {
		auto stopTime = Clock::now();
		cout
		<< msg
		<< " : "
		<< chrono::duration_cast<chrono::microseconds>(stopTime - startTime).count()
		<< " microseconds"
		<< endl;
		return *this;
	}

	Timer& start() {
		startTime = Clock::now();
		return *this;
	}
};

namespace Fa {};

template<typename T>
void print(T v) {
	cout << v << endl;
}

template<typename T, typename T2>
void print(T v, T2 v2) {
	cout << v << ' ' << v2 << endl;
}

template<typename T, typename T2, typename T3>
void print(T v, T2 v2, T3 v3) {
	cout << v << ' ' << v2 << ' ' << v3 << endl;
}

template<typename T, typename T2, typename T3, typename T4>
void print(T v, T2 v2, T3 v3, T4 v4) {
	cout << v << ' ' << v2 << ' ' << v3 << ' ' << v4 << endl;
}

