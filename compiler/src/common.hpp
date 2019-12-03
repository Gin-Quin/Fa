#include <string>
#include <iostream>
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

#ifdef __WINDOWS__
	#include <windows.h>
#else
	#include <sys/ioctl.h>
	#include <unistd.h>
#endif


#include "common/Clio.hpp"
using namespace Clio;

using std::cout;
using std::cin;
using std::string;
using std::vector;
using std::to_string;
using std::endl;

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

