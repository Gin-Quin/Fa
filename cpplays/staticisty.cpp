#include "main.hpp"


namespace Static_Zabu {
	const int x = 12;
}

struct Zabu {
	int x;
	// static const int x = 12;
};



int main(int argc, char const *argv[])
{
	// Static_Zabu::x++;
	Zabu z;
	print(Static_Zabu::x, z.x);
	return 0;
}
