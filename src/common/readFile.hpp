
inline string readFile(const char* name) {
	std::ifstream in(name);
	return string (
		std::istreambuf_iterator<char>(in),
		std::istreambuf_iterator<char>()
	);
}

inline string readFile(string name) {
	std::ifstream in(name);
	return string (
		std::istreambuf_iterator<char>(in),
		std::istreambuf_iterator<char>()
	);
}
