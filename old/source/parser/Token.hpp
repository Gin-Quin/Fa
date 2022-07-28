#include "Glue.hpp"
#include "TokenInfos.hpp"

struct Token {

	enum Type {  // List of all tokens
		${tokens.map(t => t.name).join(",\n\t\t")}
	};

	Type type { UnknownToken };
	int position { 0 };
	int length { 0 };


	void print() {
		cout
			<< " { "
			<< "type: " << type << ", "
			<< "position: " << position << ", "
			<< "length: " << length
			<< " } "
		;
	}

	inline bool isSymbol() {
		return type > SYMBOLS && type < KEYWORDS;
	}

	inline bool isWord() {
		return type == Identifier;
	}

	inline bool isKeyword() {
		return type > KEYWORDS;
	}

	inline int glue() {
		return tokenInfos[type].glue;
	}

	inline int priority() {
		return tokenInfos[type].priority;
	}

	void incrementType() {
		type = static_cast<Type>(static_cast<int>(type) + 1);
	}

	/**
	 * {"type": xx, "position": xxxx, "length": xx}
	 */
	inline string toJson() const {
		return '{' + jsonParameters() + '}';
	}

	inline string toVerboseJson() const {
		return '{' + jsonVerboseParameters() + '}';
	}

	string jsonParameters() const {
		string json;
		json += "\\"type\\":";
		json += to_string(type);

		json += ",\\"position\\":";
		json += to_string(position);

		json += ",\\"length\\":";
		json += to_string(length);
		return json;
	}

	inline string jsonVerboseParameters() const {
		return jsonParameters() + ",\\"name\\":\\"" + name() + "\\"";
	}

	string name() const {
		switch (type) {
			${tokens.map(({name}) => `case ${name}: return "${name}";`).join("\n\t\t\t")}
		}
	}
};

