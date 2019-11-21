#include <string>
#include <iostream>
#include <fstream>
#include <streambuf>
#include <vector>
#include <cstdio>
#include <cstring>
#include <map>

#include "libraries/Clio.hpp"

using std::cout;
using std::cin;
using std::string;
using std::vector;
using std::to_string;
using std::endl;


inline string readFile(const char* name) {
	std::ifstream in(name);
	return string (
		std::istreambuf_iterator<char>(in), 
		std::istreambuf_iterator<char>()
	);
}

using namespace Clio;