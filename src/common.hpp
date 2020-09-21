
#include <iostream>
#include <string>
#include <fstream>
#include <streambuf>
#include <vector>
#include <cstdio>
#include <cstring>
#include <map>


#ifdef _WIN32
	#define __WINDOWS__
#else
	#ifdef __WIN32__
		#define __WINDOWS__
	#else
		#define __UNIX__
	#endif
#endif


#include "common/Clio.hpp"
using namespace Clio;
#include "common/Timer.hpp"

using std::cout;
using std::cin;
using std::string;
using std::vector;
using std::to_string;
using std::endl;
using std::map;

#include "common/prettyError.hpp"
#include "common/StringUtilities.hpp"
using namespace StringUtilities;

inline string readFile(const char* name) {
	std::ifstream in(name);
	return string (
		std::istreambuf_iterator<char>(in),
		std::istreambuf_iterator<char>()
	);
}
