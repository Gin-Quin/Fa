#pragma once
#include <chrono>
#include <iostream>

using namespace std::chrono;

struct Timer {
	high_resolution_clock::time_point begin;
	
	void start() {
		begin = high_resolution_clock::now();
	}

	void stop(const char* msg, unsigned int iterations) {
		auto end = high_resolution_clock::now();
		auto duration = duration_cast<nanoseconds>(end-begin).count();

		std::cout
			<< "["
			<< msg
			<< "] Total : "
			<< duration
			<< "ns | Average : "
			<< duration / iterations
			<< "ns"
			<< std::endl;
	}
};