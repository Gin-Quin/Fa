
#include "Var.hpp"



using namespace std;


Array::~Array() {
	vec.clear();
}


int Array::length() {
	return vec.size();
}

Var& Array::operator[](int n) {
	if (n >= length())
		vec.resize(n+1);
	return vec[n];
}

void Array::push(Var &v) {
	vec.push_back(v);
}

Var Array::pop() {
	Var last;
	if (length())
		last = vec.back();
	vec.pop_back();
	return last;
}


void Array::clear() {
	vec.clear();
}