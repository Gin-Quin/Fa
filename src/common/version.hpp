


struct {
	int MAJOR { ${MAJOR} };
	int MINOR { ${MINOR} };
	int PATCH { ${PATCH} };

	const char* operator()() {
		return "${MAJOR}.${MINOR}.${PATCH}";
	}
} version;
