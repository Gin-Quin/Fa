


struct {
	int MAJOR { ${MAJOR} };
	int MINOR { ${MAJOR} };
	int PATCH { ${MAJOR} };

	const char* operator()() {
		return "${MAJOR}.${MINOR}.${PATCH}";
	}
} version;


