/**
* A statement is the abstract retpresentation of one line of code.
* It is a token list plus a possible body.
*/

#include "Token.hpp"
#include "Body.hpp"

struct Statement : public vector<Token> {
	Body body {};
	Token::Type lastType;

	~Statement() {
		for (Statement* child : body)
			delete child;
	}

	inline void push(Token token) {
		push_back(token);
		lastType = token.type;
	}

	inline void push(Token& token) {
		push_back(token);
		lastType = token.type;
	}

	inline bool hasBody() {
		return body.size();
	}

	string toJson() {
		int i = 0;
		string json = "{";
		for (auto& token : *this)
			json += '"' + to_string(i++) + "\":" + token.toJson() + ',';
		return json + "\"body\":" + body.toJson() + '}';
	}

	string toVerboseJson() {
		int i = 0;
		string json = "{";
		for (auto& token : *this)
			json += '"' + to_string(i++) + "\":" + token.toVerboseJson() + ',';
		return json + "\"body\":" + body.toVerboseJson() + '}';
	}
};

string Body::toJson() const {
	string json = "[";
	if (size()) {
		for (auto statement : *this)
			json += statement->toJson() + ',';
		json[json.length() - 1] = ']';
	}
	return json + ']';
}

string Body::toVerboseJson() const {
	string json = "[";
	if (size()) {
		for (auto statement : *this)
			json += statement->toVerboseJson() + ',';
		json[json.length() - 1] = ']';
	}
	return json + ']';
}
