#pragma once

#include "Statement.hpp"
#include "IsNumber.hpp"
#include "Node.hpp"
// #include "Validator.hpp"


struct Parser {
	const char* melody;
	vector<Body*> scope { new Body() };
	Statement *currentStatement { NULL };
	Node* tree { new Node() };  // the root node of the Abstract Syntax Tree
	int position { 0 };  // current position of the cursor
	int length { 0 };  // length of the current word/symbol
	IsNumber isNumber;

	// state of the parsing
	bool hasTokenized = false;
	bool hasGrownTree = false;

	// Configuration variables
	int indentUnit { 0 };  // number of spaces for one indentation unit
	char indentType { 0 };  // space or tab (if tabs, indentUnit must be 1)
	int indentLevel { 0 };  // current indent level
	string stringOpeners { "" };  // list of recursive string openers (' or ")
	int stringDepth { 0 };  // depth of string recursion (inside a template)
	int curlyBraceDepth { 0 };  // depth of curly braces (inside a template)
	bool startOfLine { true };  // true if we are at the beginning of a new line

	// locking
	enum LockType {
		Comment,
		Multiline
	};
	int indentLock { 0 };  // locked indentation (multiline strings or comments)
	LockType lockType;  // locked indentation (multiline strings or comments)

	// Constant messages
	static constexpr const char* forbiddenEolInString = "Missing end of string character before new line";
	static constexpr const char* forbiddenEolInRegex = "Missing end of regex or glob before new line";

	// Constructor / Deletor
	Parser(const char* _melody);
	Parser(const string& str) : Parser(str.data()) {}
	~Parser();


	// -- Tokenizing functions
	void tokenize();
	inline const Body& body() { return *scope[0]; }
	inline void push(Token::Type type);
	inline void pushStatement();
	inline void indent();
	inline void unindent();
	void parseString(char opener=0);  // parse string & push the right tokens
	void parseRegexOrGlob(Token::Type);  // parse regex & push the right tokens
	int getIndentLevel();  // skip all empty lines & return the number of spaces/tabs before the first non-empty line
	inline void pushRawLine(Token::Type);
	inline void pushLockedLine();



	// -- Tree-growing functions
	Node* growTree();
	Node* parseStatement(  // parse a statement
		Statement*,
		Token::Type groupType = Token::Type::UnknownToken
	);
	Node* parseBody(const Body&);  // parse multiple statements and return a new BodyNode
	Token::Type getStopToken(Token::Type type);
	void checkChaining(Token* left, Token* right);
	void parseTemplateString(Statement* statement, Node* root);




	// -- Printers
	inline void printTokens();  // print all tokens
	inline void printTree();  // print the tree
	inline string error(string msg);  // display a pretty error
	void print(Statement* statement, int depth=0);
	void print(Node* node, int depth=0);
	string coloredToken(const Token& token);
	string coloredToken(Token* token);
	string extract(const Token& token);  // extract a token's content from the melody

};