#include <iostream>
#include <string>
#include <fstream>
#include <streambuf>
#include <vector>
#include <cstdio>
#include <cstring>
#include <map>
#include <chrono>

#ifdef _WIN32
	#define __WINDOWS__
#else
	#ifdef __WIN32__
		#define __WINDOWS__
	#else
		#define __UNIX__
	#endif
#endif
/**
* Micro C++ library for colored output in terminal.
*/

namespace Clio {

	namespace Font {
		const char
			*reset = "\33[0m",
			*bold = "\33[1m",
			*faint = "\33[2m",
			*italic = "\33[3m",
			*underline = "\33[4m",
			*blink = "\33[5m",
			*fastBlink = "\33[6m",
			*inverse = "\33[7m",
			*strike = "\33[9m",
			*framed = "\33[51m",
			*encircled = "\33[52m",
			*overlined = "\33[53m";
	}

	namespace Ink {
		const char
			// ink colors
			*red = "\33[31m",
			*green = "\33[32m",
			*yellow = "\33[33m",
			*blue = "\33[34m",
			*purple = "\33[35m",
			*magenta = "\33[35m",
			*cyan = "\33[36m",
			*white = "\33[37m",
			*brightBlack = "\33[90m",
			*brightRed = "\33[91m",
			*brightGreen = "\33[92m",
			*brightYellow = "\33[93m",
			*brightBlue = "\33[94m",
			*brightPurple = "\33[95m",
			*brightMagenta = "\33[95m",
			*brightCyan = "\33[96m",
			*brightWhite = "\33[97m";
	}

	namespace Paper {
		const char
			*black = "\27[40m",
			*red = "\33[41m",
			*green = "\33[42m",
			*yellow = "\33[43m",
			*blue = "\33[44m",
			*purple = "\33[45m",
			*magenta = "\33[45m",
			*cyan = "\33[46m",
			*white = "\33[47m",
			*brightBlack = "\33[100m",
			*brightRed = "\33[101m",
			*brightGreen = "\33[102m",
			*brightYellow = "\33[103m",
			*brightBlue = "\33[104m",
			*brightPurple = "\33[105m",
			*brightMagenta = "\33[105m",
			*brightCyan = "\33[106m",
			*brightWhite = "\33[107m";
	}


	namespace Screen {
		struct Size {
			unsigned int rows;
			unsigned int columns;
		};

		const char
			*clearAll = "\33[2J",
			*clearFromCursorToEnd = "\33[J",
			*clearFromCursorToStart = "\33[1J",
			*clearLine = "\33[2K",
			*clearFromCursorToEndOfLine = "\33[K",
			*clearFromCursorToStartOfLine = "\33[1K";

		inline std::string scrollUp(int n=1) {
			return "\33["+ std::to_string(n) +'S';
		}
		inline std::string scrollDown(int n=1) {
			return "\33["+ std::to_string(n) +'T';
		}

		// Size getConsoleSize() {
		// 	Size size;

		// 	#ifdef __WINDOWS__
		// 		CONSOLE_SCREEN_BUFFER_INFO csbi;
		// 		GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
		// 		size.rows = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
		// 		size.columns = csbi.srWindow.Right - csbi.srWindow.Left + 1;
		// 	#else
		// 		struct winsize w;
		// 		ioctl(STDOUT_FILENO, TIOCGWINSZ, &w);
		// 		size.rows = w.ws_row;
		// 		size.columns = w.ws_col;
		// 	#endif

		// 	return size;
		// }
	}


	namespace Cursor {
		inline std::string up(int n=1) {
			return "\33["+ std::to_string(n) +'A';
		}

		inline std::string down(int n=1) {
			return "\33["+ std::to_string(n) +'B';
		}

		inline std::string forward(int n=1) {
			return "\33["+ std::to_string(n) +'C';
		}
		
		inline std::string backward(int n=1) {
			return "\33["+ std::to_string(n) +'D';
		}

		inline std::string toNextLine(int n=1) {
			return "\33["+ std::to_string(n) +'E';
		}

		inline std::string toPreviousLine(int n=1) {
			return "\33["+ std::to_string(n) +'F';
		}

		inline std::string toLine(int n=1) {
			return "\33["+ std::to_string(n) +'G';
		}

		inline std::string moveTo(int line=1, int col=1) {
			return "\33["+ std::to_string(line) + ";"+ std::to_string(col) +"H";
		}

		const char
			*report = "\33[6n",
			*save = "\33[s",
			*reset = "\33[u";
	}
}

using namespace Clio;

using namespace std::chrono;

struct Timer {
	high_resolution_clock::time_point begin;
	
	void start() {
		begin = high_resolution_clock::now();
	}

	void stop(const char* msg, unsigned int iterations) {
		auto end = high_resolution_clock::now();
		auto duration = duration_cast<nanoseconds>(end-begin).count();

		std::cout
			<< "["
			<< msg
			<< "] Total : "
			<< duration
			<< "ns | Average : "
			<< duration / iterations
			<< "ns"
			<< std::endl;
	}
};
using std::cout;
using std::cin;
using std::string;
using std::vector;
using std::to_string;
using std::endl;
using std::map;

namespace StringUtilities {

	/**
	* Character check functions
	*/
	inline bool isSpace(char c) {
		return c == ' ' || c == '\t';
	}
	inline bool isBlank(char c) {  // ignored characters
		return c == '\r';
	}
	inline bool isEndOfLine(char c) {
		return c == '\n';
	}
	inline bool isControlCharacter(char c) {
		// (valid control characters for Fa files are : 0, \t, \n, \r)
		return c > 0 && c < 32;
	}


	/**
	* Check if the given character is a word-breaker (and is a symbol too)
	*/
	inline bool doBreakWord(char c) {
		return (
			!(128 & c) && !(
				c == '$'
				|| c == '_'
				|| (c >= 'a' && c <= 'z')
				|| (c >= 'A' && c <= 'Z')
			)
		);
	}

	inline bool isWordCharacter(char c) {
		return (128 & c)
			|| c == '$'
			|| c == '_'
			|| (c >= 'a' && c <= 'z')
			|| (c >= 'A' && c <= 'Z')
			|| (c >= '0' && c <= '9')
		;
	}

	
	/**
	* Return the number of characters before the end of line
	*/
	int getLineLength(const char* str, int offset) {
		char c;
		int length = 0;
		while ((c = str[offset+length]) && !isEndOfLine(c))
			length++;
		return length;
	}





	
	/**
	* When given a string with an offset pointing to a group character like '[', '(', '{', '\' or '"',
	* return the length until the group close.
	*/
	// int fragment(const char* src, int offset=0) {
	// 	int cursor = offset;
	// 	char c;
	// 	char starter = src[offset], ender;

	// 	switch (starter) {
	// 		case 0: return 0;  // empty string

	// 		case '"': case '\'': case '`': case '/':  // string / regular expression
	// 			while ((c = src[++cursor])) switch (c) {
	// 				case starter: return cursor - offset;
	// 				case '\\': if (src[cursor+1]) cursor++;  // next character is escaped
	// 			}
	// 			throw ((string) "Missing ") + starter;

	// 		case '(': ender = ')'; break;
	// 		case '[': ender = ']'; break;
	// 		case '{': ender = '}'; break;
	// 	}

	// 	// groups who can contain sub-groups and strings
	// 	int level = 0;
	// 	while ((c = src[++cursor])) switch (c) {
	// 		case starter: level++; continue;
	// 		case ender:
	// 			if (level) level--;
	// 			else return cursor - offset;
	// 		continue;

	// 		case '"': case '\'': case '`': case '/':  // we enter a string or regex
	// 			char subStarter = c;
	// 			while ((c = src[++cursor]) && c != subStarter)
	// 				if (c == '\\' && src[cursor+1]) cursor++;  // next character is escaped
	// 			if (!c) throw ((string) "Missing ") + subStarter;
	// 	}

	// 	throw ((string) "Missing ") + ender;
	// }
}
/**
* Structure returned by the getLocation method.
*/
struct Location {
	int line       { 1 };  // line number
	int column     { 1 };  // column number
	int lineStart  { 0 };  // position of the first non-space character of the line
	int lineLength { 0 };  // number of characters before the next end-of-line character

	Location() {}
	Location(const char* str, int position, int length=0) {
		char c;
		int padding = 0, offset = 0;
		int end = position + length;

		while (offset < end && (c = str[offset++])) {
			if (StringUtilities::isEndOfLine(c)) {
				line++;
				lineStart = offset;
				column = 1;
				padding = 0;
			}
			else {
				if (padding == column && StringUtilities::isBlank(c))
					lineStart++, padding++;
				column++;
			}
		}

		lineLength = StringUtilities::getLineLength(str, lineStart);
	}
};



/**
* Display the error msg with position information
*/
string prettyError (
	const char* source,
	string message,
	int position,
	int length=0,
	const char* filename=""
) {
	auto [line, column, lineStart, lineLength] = Location(source, position, length);
	string lineContent(source + lineStart, lineLength);

	cout
	<< Ink::brightRed
	<< Font::bold
	<< "[!] "
	<< message
	<< Font::reset
	<< Font::italic
	// << endl
	
	<< " (at position "
	<< Ink::white
	<< position + length
	
	<< Font::reset
	<< Font::italic
	<< ", line "
	<< Ink::brightYellow
	<< line

	<< Font::reset
	<< Font::italic
	<< ", column "
	<< Ink::brightPurple
	<< column

	<< Font::reset
	<< Font::italic
	<< ")"
	<< endl
	<< endl
	<< Font::reset
	<< Ink::brightYellow
	<< Font::bold
	<< line
	<< " |  "
	<< Font::reset
	<< Font::italic
	<< Ink::white
	<< lineContent
	<< endl
	<< "    ";

	for (int i=0; i < to_string(line).length(); i++)
		cout << ' ';

	for (int i=0; i < column; i++)
		cout << (lineContent[i] == '\t' ? '\t' : ' ');

	cout
	<< Font::reset
	<< Font::bold
	<< Ink::brightPurple
	<< '^'
	<< Font::reset
	<< endl
	<< endl;

	return message;
}


using namespace StringUtilities;

inline string readFile(const char* name) {
	std::ifstream in(name);
	return string (
		std::istreambuf_iterator<char>(in),
		std::istreambuf_iterator<char>()
	);
}

// the parser
namespace Fa {
/**
* A statement is the abstract retpresentation of one line of code.
* It is a token list plus a possible body.
*/
/**
 * Describe how a token should glue to the others.
 * One token can 
 */

namespace Glue {
	constexpr unsigned int
		NotConcerned = 0,  // Not concerned by gluing (for special cases)
		Assimilable = 1,  // Can be glued but do not glue others
		Group = 2,  // Glue inside (Parenthesis, brackets, ...)
		Left = 4,  // Glue a left token
		Right = 8,  // Glue a right token
		Body = 16,  // Glue a following body

		// -- modifiers
		Single = 32,  // When glue left and right, cannot chain gluing
		WeakLeft = 64,  // When left is not possible, increment token type by 1
		WeakRight = 128,  // No error thrown when right is not possible
		OptionalBody = 256 + 16,  // The token can glue a body but not mandatory (ex: comments)
		TransformAtStartOfStatement = 512,  // When at start of statement, increment token type
		TransformAtEndOfStatement = 1024;  // When at end of statement, increment token type by 1
}


struct TokenInfo {
	int glue;
	int priority { 0 };  // 0 : not concerned
};

constexpr TokenInfo tokenInfos[] = {
	/* 0. UnknownToken */         {  },
	/* 1. Number */               { Glue::Assimilable },
	/* 2. Comment */              { Glue::OptionalBody },
	/* 3. SubComment */           {  },
	/* 4. Checkpoint */           {  },
	/* 5. Identifier */           { Glue::Assimilable },
	/* 6. String */               { Glue::Group },
	/* 7. RawString */            { Glue::Assimilable },
	/* 8. StringEnd */            {  },
	/* 9. SYMBOLS */              {  },
	/* 10. LeftParenthesis */     { Glue::Assimilable | Glue::Group },
	/* 11. RegexStart */          { Glue::Assimilable | Glue::Group },
	/* 12. GlobStart */           { Glue::Assimilable | Glue::Group },
	/* 13. LeftBrace */           { Glue::Assimilable | Glue::Group | Glue::WeakLeft },
	/* 14. LeftBraceNoLeft */     { Glue::Assimilable | Glue::Group },
	/* 15. Backslash */           { Glue::Left | Glue::Right, 30 },
	/* 16. DoubleBackslash */     { Glue::Left | Glue::Right, 30 },
	/* 17. Equal */               { Glue::Left | Glue::Right, 6 },
	/* 18. Colon */               { Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 7 },
	/* 19. ColonBody */           { Glue::Left | Glue::Body },
	/* 20. LeftCurlyBrace */      { Glue::Assimilable | Glue::Group },
	/* 21. Dot */                 { Glue::Left | Glue::Right, 40 },
	/* 22. Comma */               { Glue::Left | Glue::Right, 2 },
	/* 23. Apostrophe */          { Glue::Assimilable | Glue::Group },
	/* 24. Quote */               { Glue::Assimilable | Glue::Group },
	/* 25. Accent */              { Glue::Assimilable | Glue::Group },
	/* 26. Asterisk */            { Glue::Left | Glue::Right, 30 },
	/* 27. Divide */              { Glue::Left | Glue::Right, 30 },
	/* 28. Circumflex */          { Glue::Left | Glue::Right, 18 },
	/* 29. Plus */                { Glue::Left | Glue::Right | Glue::WeakLeft, 28 },
	/* 30. PlusRight */           { Glue::Right, 31 },
	/* 31. Minus */               { Glue::Left | Glue::Right | Glue::WeakLeft, 29 },
	/* 32. MinusRight */          { Glue::Right, 31 },
	/* 33. QuestionMark */        { Glue::Left },
	/* 34. Tilde */               { Glue::Right },
	/* 35. DoubleEqual */         { Glue::Left | Glue::Right, 22 },
	/* 36. NotEqual */            { Glue::Left | Glue::Right, 22 },
	/* 37. Equivalent */          { Glue::Left | Glue::Right, 22 },
	/* 38. LesserOrEqual */       { Glue::Left | Glue::Right, 22 },
	/* 39. GreaterOrEqual */      { Glue::Left | Glue::Right, 22 },
	/* 40. InputArrow */          { Glue::Left | Glue::Right, 8 },
	/* 41. OutputArrow */         { Glue::Left | Glue::Right, 8 },
	/* 42. Percent */             { Glue::Left | Glue::Right, 31 },
	/* 43. Extract */             { Glue::Left | Glue::Right, 6 },
	/* 44. Insert */              { Glue::Left | Glue::Right, 6 },
	/* 45. DoubleDot */           { Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 40 },
	/* 46. DoubleDotBody */       { Glue::Left | Glue::Body, 40 },
	/* 47. TripleDot */           { Glue::Right | Glue::TransformAtEndOfStatement },
	/* 48. TripleDotBody */       { Glue::Left | Glue::Body },
	/* 49. MultiLineString */     { Glue::Assimilable },
	/* 50. PlusPlus */            { Glue::Left, 38 },
	/* 51. MinusMinus */          { Glue::Left, 38 },
	/* 52. Power */               { Glue::Left | Glue::Right, 32 },
	/* 53. PlusEqual */           { Glue::Left | Glue::Right, 6 },
	/* 54. MinusEqual */          { Glue::Left | Glue::Right, 6 },
	/* 55. TimesEqual */          { Glue::Left | Glue::Right, 6 },
	/* 56. DivideEqual */         { Glue::Left | Glue::Right, 6 },
	/* 57. IntegerDivideEqual */  { Glue::Left | Glue::Right, 6 },
	/* 58. LesserThan */          { Glue::Left | Glue::Right, 24 },
	/* 59. GreaterThan */         { Glue::Left | Glue::Right, 24 },
	/* 60. SendTo */              { Glue::Left | Glue::Right },
	/* 61. Pipe */                { Glue::Left | Glue::Right },
	/* 62. At */                  { Glue::Right, 50 },
	/* 63. Semicolon */           {  },
	/* 64. RightParenthesis */    {  },
	/* 65. RegexOrGlobContent */  {  },
	/* 66. RegexOrGlobEnd */      {  },
	/* 67. RegexOrGlobOption */   {  },
	/* 68. RightBrace */          {  },
	/* 69. RightCurlyBrace */     {  },
	/* 70. UserDefinedSymbol */   {  },
	/* 71. KEYWORDS */            {  },
	/* 72. Let */                 { Glue::Right },
	/* 73. Super */               { Glue::Assimilable },
	/* 74. Print */               { Glue::Right },
	/* 75. Use */                 { Glue::Right },
	/* 76. Import */              { Glue::Right },
	/* 77. Export */              { Glue::Right },
	/* 78. From */                { Glue::Left | Glue::Right },
	/* 79. Extends */             { Glue::Left | Glue::Right },
	/* 80. IfComprehension */     { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 81. If */                  { Glue::Right | Glue::Body },
	/* 82. ElseComprehension */   { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 83. Else */                { Glue::Body },
	/* 84. ElseIf */              { Glue::Right | Glue::Body },
	/* 85. Then */                { Glue::Right },
	/* 86. Do */                  { Glue::Right },
	/* 87. WhileComprehension */  { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 88. While */               { Glue::Right | Glue::Body },
	/* 89. RepeatComprehension */ { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 90. Repeat */              { Glue::Right | Glue::Body },
	/* 91. ForComprehension */    { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 92. For */                 { Glue::Right | Glue::Body },
	/* 93. In */                  { Glue::Left | Glue::Right },
	/* 94. When */                { Glue::Left | Glue::Right },
	/* 95. And */                 { Glue::Left | Glue::Right, 12 },
	/* 96. Or */                  { Glue::Left | Glue::Right, 10 },
	/* 97. Xor */                 { Glue::Left | Glue::Right },
	/* 98. Modulo */              { Glue::Left | Glue::Right },
	/* 99. Is */                  { Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement },
	/* 100. IsStart */            { Glue::Right | Glue::OptionalBody },
	/* 101. To */                 { Glue::Left | Glue::Right },
	/* 102. Not */                { Glue::Right },
	/* 103. Isnt */               { Glue::Left | Glue::Right },
	/* 104. Return */             { Glue::Right },
	/* 105. Continue */           { Glue::Right },
	/* 106. Break */              { Glue::Right },
	/* 107. Try */                { Glue::Right },
	/* 108. Catch */              { Glue::Right },
	/* 109. Finally */            { Glue::Right },
	/* 110. Throw */              { Glue::Right },
	/* 111. Async */              { Glue::Right },
	/* 112. Await */              { Glue::Right },
	/* 113. Yield */              { Glue::Right },
	/* 114. Nil */                { Glue::Assimilable },
	/* 115. True */               { Glue::Assimilable },
	/* 116. False */              { Glue::Assimilable },
	/* 117. Infinity */           { Glue::Assimilable },
	/* 118. Global */             { Glue::Right },
	/* 119. Override */           { Glue::Right },
	/* 120. Final */              { Glue::Right },
	/* 121. Const */              { Glue::Right },
	/* 122. Private */            { Glue::Right },
	/* 123. Static */             { Glue::Right },
	/* 124. Class */              { Glue::Right | Glue::OptionalBody },
	/* 125. Enum */               { Glue::Right | Glue::OptionalBody },
	/* 126. Abstract */           { Glue::Right | Glue::OptionalBody },
	/* 127. Interface */          { Glue::Right | Glue::OptionalBody },
	/* 128. Structure */          { Glue::Right | Glue::OptionalBody },
	/* 129. Unique */             { Glue::Right | Glue::OptionalBody },
	/* 130. Exclamation */        { Glue::Right },
	/* 131. Self */               { Glue::Assimilable }
};

/**
 * List of all tokens
 */
struct Token {
	enum Type {
		UnknownToken,
		Number,
		Comment,
		SubComment,
		Checkpoint,
		Identifier,
		String,
		RawString,
		StringEnd,
		SYMBOLS,
		LeftParenthesis,
		RegexStart,
		GlobStart,
		LeftBrace,
		LeftBraceNoLeft,
		Backslash,
		DoubleBackslash,
		Equal,
		Colon,
		ColonBody,
		LeftCurlyBrace,
		Dot,
		Comma,
		Apostrophe,
		Quote,
		Accent,
		Asterisk,
		Divide,
		Circumflex,
		Plus,
		PlusRight,
		Minus,
		MinusRight,
		QuestionMark,
		Tilde,
		DoubleEqual,
		NotEqual,
		Equivalent,
		LesserOrEqual,
		GreaterOrEqual,
		InputArrow,
		OutputArrow,
		Percent,
		Extract,
		Insert,
		DoubleDot,
		DoubleDotBody,
		TripleDot,
		TripleDotBody,
		MultiLineString,
		PlusPlus,
		MinusMinus,
		Power,
		PlusEqual,
		MinusEqual,
		TimesEqual,
		DivideEqual,
		IntegerDivideEqual,
		LesserThan,
		GreaterThan,
		SendTo,
		Pipe,
		At,
		Semicolon,
		RightParenthesis,
		RegexOrGlobContent,
		RegexOrGlobEnd,
		RegexOrGlobOption,
		RightBrace,
		RightCurlyBrace,
		UserDefinedSymbol,
		KEYWORDS,
		Let,
		Super,
		Print,
		Use,
		Import,
		Export,
		From,
		Extends,
		IfComprehension,
		If,
		ElseComprehension,
		Else,
		ElseIf,
		Then,
		Do,
		WhileComprehension,
		While,
		RepeatComprehension,
		Repeat,
		ForComprehension,
		For,
		In,
		When,
		And,
		Or,
		Xor,
		Modulo,
		Is,
		IsStart,
		To,
		Not,
		Isnt,
		Return,
		Continue,
		Break,
		Try,
		Catch,
		Finally,
		Throw,
		Async,
		Await,
		Yield,
		Nil,
		True,
		False,
		Infinity,
		Global,
		Override,
		Final,
		Const,
		Private,
		Static,
		Class,
		Enum,
		Abstract,
		Interface,
		Structure,
		Unique,
		Exclamation,
		Self
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
	inline string toJson() {
		string json;
		json += '{';
			json += "\"type\":";
			json += to_string(type);

			json += ",\"position\":";
			json += to_string(position);

			json += ",\"length\":";
			json += to_string(length);
		json += '}';
		return json;
	}
};


struct Statement;
using Body = vector<Statement*>;

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
};

/**
* Class used to detect if a character string is a number.
* It does not check directly a full string, but rather gets character after character.
*/

struct IsNumber {
	int length { 0 };
	int base { 10 };
	char lastInsertedCharacter { 0 };
	bool isNumber { true };
	bool exponential { false };  // true if an 'e' appeared
	bool decimal { false };  // true if a '.' appeared

	inline IsNumber& notNumber() {
		isNumber = false;
		return *this;
	}

	IsNumber& operator << (char c) {
		if (!isNumber)
			return *this;
		
		if (c == '.') {
			if (exponential || decimal)
				return notNumber();
			decimal = true;
		}

		else if (c == 'e') {
			if (exponential || length == 0)
				return notNumber();
			exponential = true;
		}

		else if (base == 10) {
			if (c == 'x') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 16;
			}
			else if (c == 'b') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 2;
			}
			else if (c == 'o') {
				if (length != 1 || lastInsertedCharacter != '0')
					return notNumber();
				base = 8;
			}
			else if (c < '0' || c > '9')
				return notNumber();
		}

		else if (base == 2) {
			if (c != '0' && c != '1')
				return notNumber();
		}

		else if (base == 8) {
			if (c < '0' || c > '7')
				return notNumber();
		}

		else if (base == 16) {
			if ((c < '0' || c > '9') && (c < 'A' || c > 'F') && (c < 'a' || c > 'f'))
				return notNumber();
		}

		lastInsertedCharacter = c;
		length++;
		return *this;
	}

	operator bool() {
		return isNumber;
	}

	void reset() {
		length = 0;
		base = 10;
		lastInsertedCharacter = 0;
		isNumber = true;
		exponential = false;
		decimal = false;
	}
};


/**
 * The base structure for a Node Fa's syntax tree.
 * The Fa parser uses its own algorithm to generate the tree.
 * This algorithm is only based on two operations :
 - `assimilate` : add a child node
 - `cuckold` : a node cuckolds another node when :
	1. It becomes the father of the node's last child
	2. He then becomes the last child of the node
 * This algorithm makes Fa sort of a `grammar-less` language.
 * The grammar is checked during the validation phase, each node having its own validation rules.
 */
struct Node {
	Token* token;
	vector<Node*> children;
	void* data { NULL };  // the arbitrary data that brings the node. Used and defined by tree walkers.

	Node() {
		token = NULL;
	}
	Node(Token* from) {
		token = from;
	}
	~Node() {
		for (auto child : children)
			delete child;
	}

	inline int priority() {
		return token ? token->priority() : 0;
	}
	inline int glue() {
		return token ? token->glue() : 0;
	}

	Node* lastChild() {
		return children.size()? children.back() : NULL;
	}

	Node* assimilate(Node* node) {
		children.push_back(node);
		return this;
	}

	Node* cuckoldedBy(Node* node) {
		if (children.size()) {
			node->assimilate(children[children.size() - 1]);
			children[children.size() - 1] = node;
		}
		else
			children.push_back(node);
		return node;
	}

	/**
	 * {"type": xx, "position": xxxx, "length": xx}
	 */
	inline string toJson() {
		string json;
		json += '{';
			json += "\"token\":";
			json += token ? token->toJson() : "null,";

			json += "\"children\":[";
				for (int i=0; i < children.size(); i++) {
					if (i) json += ',';
					json += children[i]->toJson();
				}
			json += ']';
		json += '}';
		return json;
	}
};





/**
 * A symbol is a combination of punctuation characters.
 * They are separated from keywords because they have a different matching algorithm.
 */
struct Symbol {
	const char* value;
	Token::Type type;
};


/*
 * The list of the different symbols.
 * Can be extended by user-defined symbols.
**/
struct {
	vector<Symbol> table {
		{ "...", Token::TripleDot },
		{ "==", Token::DoubleEqual },
		{ "!=", Token::NotEqual },
		{ "~=", Token::Equivalent },
		{ "<=", Token::LesserOrEqual },
		{ ">=", Token::GreaterOrEqual },
		{ "->", Token::OutputArrow },
		{ "<-", Token::InputArrow },
		{ "<<", Token::Insert },
		{ ">>", Token::Extract },
		{ "..", Token::DoubleDot },
		{ "++", Token::PlusPlus },
		{ "--", Token::MinusMinus },
		{ "**", Token::Power },
		{ "//", Token::RegexStart },
		{ "||", Token::GlobStart },
		{ "+=", Token::PlusEqual },
		{ "-=", Token::MinusEqual },
		{ "*=", Token::TimesEqual },
		{ "/=", Token::DivideEqual },
		{ "|>", Token::SendTo },
		{ "\\\\", Token::DoubleBackslash },
		{ "(", Token::LeftParenthesis },
		{ "[", Token::LeftBrace },
		{ "{", Token::LeftCurlyBrace },
		{ ")", Token::RightParenthesis },
		{ "]", Token::RightBrace },
		{ "}", Token::RightCurlyBrace },
		{ "\\", Token::Backslash },
		{ "=", Token::Equal },
		{ "+", Token::Plus },
		{ "-", Token::Minus },
		{ "?", Token::QuestionMark },
		{ ".", Token::Dot },
		{ "|", Token::Pipe },
		{ ",", Token::Comma },
		{ ":", Token::Colon },
		{ "'", Token::String },
		{ """, Token::String },
		{ "`", Token::String },
		{ "#", Token::Comment },
		{ "*", Token::Asterisk },
		{ "/", Token::Divide },
		{ "@", Token::At },
		{ "!", Token::Exclamation },
		{ ">", Token::GreaterThan },
		{ "<", Token::LesserThan },
		{ "^", Token::Circumflex },
		{ "~", Token::Tilde },
		{ ";", Token::Semicolon },
		{ "%", Token::Percent }
	};


	Token::Type find(const char* in, int& length) {
		char c;
		int i;

		for (auto& symbol : table) {
			i = 0;
			while ((c = symbol.value[i]) == in[i] && c) i++;

			if (c == 0) {  // they matched
				length = strlen(symbol.value);
				return symbol.type;
			}
		}

		length = 0;
		return Token::UnknownToken;
	}

} Symbols;


/**
 * A keyword is made of alphanumeric characters
 * and has a special meaning in Fa's language.
 */
struct Keyword {
	const char* value;
	Token::Type type;
};


/**
 * The list of the different keywords.
 * Keywords are sorted by size.
 * Maximum size for a keyword is 32 characters.
 * Minimum size is 2 characters.
 * Can be extended by user-defined keywords.
 */
struct {
	vector<Keyword> table[32] {
		{  // 2 characters
			{ "if", Token::IfComprehension },
			{ "in", Token::In },
			{ "or", Token::Or },
			{ "is", Token::Is },
			{ "to", Token::To },
			{ "do", Token::Do }
		},
		{  // 3 characters
			{ "let", Token::Let },
			{ "use", Token::Use },
			{ "for", Token::ForComprehension },
			{ "and", Token::And },
			{ "xor", Token::Xor },
			{ "not", Token::Not },
			{ "try", Token::Try },
			{ "nil", Token::Nil }
		},
		{  // 4 characters
			{ "else", Token::ElseComprehension },
			{ "from", Token::From },
			{ "self", Token::Self },
			{ "then", Token::Then },
			{ "when", Token::When },
			{ "isnt", Token::Isnt },
			{ "true", Token::True },
			{ "enum", Token::Enum }
		},
		{  // 5 characters
			{ "while", Token::WhileComprehension },
			{ "super", Token::Super },
			{ "break", Token::Break },
			{ "catch", Token::Catch },
			{ "throw", Token::Throw },
			{ "async", Token::Async },
			{ "await", Token::Await },
			{ "yield", Token::Yield },
			{ "false", Token::False },
			{ "final", Token::Final },
			{ "const", Token::Const },
			{ "class", Token::Class },
			{ "print", Token::Print }
		},
		{  // 6 characters
			{ "import", Token::Import },
			{ "export", Token::Export },
			{ "modulo", Token::Modulo },
			{ "return", Token::Return },
			{ "elseif", Token::ElseIf },
			{ "repeat", Token::RepeatComprehension },
			{ "global", Token::Global },
			{ "static", Token::Static },
			{ "unique", Token::Unique }
		},
		{  // 7 characters
			{ "extends", Token::Extends },
			{ "finally", Token::Finally },
			{ "private", Token::Private }
		},
		{  // 8 characters
			{ "continue", Token::Continue },
			{ "infinity", Token::Infinity },
			{ "override", Token::Override },
			{ "abstract", Token::Abstract }
		},
		{  // 9 characters
			{ "interface", Token::Interface },
			{ "structure", Token::Structure }
		},
		{  // 10 characters
			
		},
		{  // 11 characters
			
		},
		{  // 12 characters
			
		},
		{  // 13 characters
			
		},
		{  // 14 characters
			
		},
		{  // 15 characters
			
		},
		{  // 16 characters
			
		},
		{  // 17 characters
			
		},
		{  // 18 characters
			
		},
		{  // 19 characters
			
		},
		{  // 20 characters
			
		},
		{  // 21 characters
			
		},
		{  // 22 characters
			
		},
		{  // 23 characters
			
		},
		{  // 24 characters
			
		},
		{  // 25 characters
			
		},
		{  // 26 characters
			
		},
		{  // 27 characters
			
		},
		{  // 28 characters
			
		},
		{  // 29 characters
			
		},
		{  // 30 characters
			
		},
		{  // 31 characters
			
		}

		//
		// {	// 2 characters
		// 	{ "if", Token::IfComprehension },
		// 	{ "in", Token::In },
		// 	{ "or", Token::Or },
		// 	{ "is", Token::Is },
		// 	{ "to", Token::To },
		// 	{ "do", Token::Do }
		// },
		//
		// {	// 3 characters
		// 	{ "let", Token::Let },
		// 	{ "use", Token::Use },
		// 	{ "for", Token::ForComprehension },
		// 	{ "and", Token::And },
		// 	{ "xor", Token::Xor },
		// 	{ "not", Token::Not },
		// 	{ "try", Token::Try },
		// 	{ "nil", Token::Nil }
		// },
		//
		//
		// {	// 4 characters
		// 	{ "else", Token::ElseComprehension },
		// 	{ "from", Token::From },
		// 	{ "self", Token::Self },
		// 	{ "then", Token::Then },
		// 	{ "when", Token::When },
		// 	{ "isnt", Token::Isnt },
		// 	{ "true", Token::True },
		// 	{ "enum", Token::Enum }
		// },
		//
		// {	// 5 characters
		// 	{ "while", Token::WhileComprehension },
		// 	{ "super", Token::Super },
		// 	{ "break", Token::Break },
		// 	{ "catch", Token::Catch },
		// 	{ "throw", Token::Throw },
		// 	{ "async", Token::Async },
		// 	{ "await", Token::Await },
		// 	{ "yield", Token::Yield },
		// 	{ "false", Token::False },
		// 	{ "final", Token::Final },
		// 	{ "const", Token::Const },
		// 	{ "class", Token::Class },
		// 	{ "print", Token::Print },
		// },
		//
		// {	// 6 characters
		// 	{ "import", Token::Import },
		// 	{ "export", Token::Export },
		// 	{ "modulo", Token::Modulo },
		// 	{ "return", Token::Return },
		// 	{ "elseif", Token::ElseIf },
		// 	{ "repeat", Token::RepeatComprehension },
		// 	{ "global", Token::Global },
		// 	{ "static", Token::Static },
		// 	{ "unique", Token::Unique }
		// },
		//
		// {	// 7 characters
		// 	{ "extends", Token::Extends },
		// 	{ "finally", Token::Finally },
		// 	{ "private", Token::Private }
		// },
		//
		// {	// 8 characters
		// 	{ "continue", Token::Continue },
		// 	{ "infinity", Token::Infinity },
		// 	{ "override", Token::Override },
		// 	{ "abstract", Token::Abstract }
		// },
		//
		// {	// 9 characters
		// 	{ "interface", Token::Interface },
		// 	{ "structure", Token::Structure },
		// },

		// 10-32 characters (may be defined by the user)
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}, {{}},
		// {{}}, {{}}, {{}}
	};

	Token::Type find(const char* in, const int length) {
		if (length == 1 || length > 32)
			return Token::Identifier;

		int i;
		char c;

		for (auto& keyword : table[length-2]) {
			i = -1;
			while (++i < length && keyword.value[i] == in[i]);
			if (i == length)
				return keyword.type;
		}

		return Token::Identifier;
	}

} Keywords;

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
	Parser(const char* _melody) : melody(_melody) {}
	Parser(const string& str) : Parser(str.data()) {}
	~Parser() {
		delete scope[0];
		delete tree;
	}


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
	inline string extract(const Token& token);  // extract a token's content from the melody
	inline string extract(Token* token);  // extract a token's content from the melody
	inline string extract(Node* node);  // extract a token's content from the melody
};

// push a token to the current statement
void Parser::push(Token::Type type) {
	currentStatement->push({ type, position, length });
	position += length;
	length = 0;
}

// push the current statement to the current scope
void Parser::pushStatement() {
	if (currentStatement->size())
		scope.back()->push_back(currentStatement);
}

// indent by one
void Parser::indent() {
	indentLevel++;
	scope.push_back( &(scope.back()->back()->body) );
}

// unindent by one
void Parser::unindent() {
	indentLevel--;
	scope.pop_back();
}


/**
* Transform some fa code into a list of tokens.
* The token list will be parsed by the parser.
*/
void Parser::tokenize() {
	char c;
	position = 0;
	currentStatement = new Statement();

	if (getIndentLevel())
		throw error("Indentation at the beginning of the file");

	do {
		c = melody[position + length];

		if (isWordCharacter(c)) {
			if (isNumber) {
				isNumber << c;
				if (length && !isNumber) push(Token::Number);
			}
			length++;
		}

		else {  // special symbol
			// if there was a word before
			if (length) {
				if (isNumber)  push(Token::Number);
				else           push(Keywords.find(melody + position, length));
				startOfLine = false;
			}

			// new line
			if (!c || isEndOfLine(c)) {
				if (stringDepth)
					throw error(forbiddenEolInString);

				// we add the new line token
				position++;
				pushStatement();
				startOfLine = true;

				// if the line ends with ... it's a multiline string
				if (currentStatement->lastType == Token::TripleDot) {
					currentStatement->back().type = Token::MultiLineString;
					lockType = LockType::Multiline;
					indent();
					indentLock = indentLevel;
				}

				if (c) getIndentLevel();
				else currentStatement = NULL;
			}

			// space
			else if (isSpace(c) || isBlank(c)) {
				position++;
				length = 0;
			}

			// forbidden control character
			else if (isControlCharacter(c)) {
				cout << "zabu ??" << endl;
				throw error("Forbidden control character");
			}


			// other symbol
			else {
				auto type = Symbols.find(melody + position, length);

				if (type == Token::UnknownToken)
					throw error("Unexpected symbol");

				// comment
				if (type == Token::Comment) {
					pushRawLine(Token::Comment);
					length = 0;
					if (startOfLine) {
						pushStatement();
						indent();
						indentLock = indentLevel;
						lockType = LockType::Comment;
						getIndentLevel();
					}
				}

				// checkpoint
				else if (startOfLine && type == Token::GreaterThan) {
					pushRawLine(Token::Checkpoint);
					length = 0;
				}

				// regular symbol
				else {
					push(type);
					startOfLine = false;

					switch (type) {
						// start of string
						case Token::String:
							parseString(c);
						break;

						// maybe end of template
						case Token::RightCurlyBrace:
							if (stringDepth) {
								if (curlyBraceDepth)
									curlyBraceDepth--;
								else {  // end of template
									stringDepth--;
									char opener = stringOpeners.back();
									stringOpeners.pop_back();
									parseString(opener);
								}
							}
						break;

						// curly brace : increment depth
						case Token::LeftCurlyBrace:
							if (stringDepth)
								curlyBraceDepth++;
						break;

						// regex / glob
						case Token::RegexStart:
						case Token::GlobStart:
							parseRegexOrGlob(type);
						break;

						default:
						break;
					}
				}
			}

			isNumber.reset();
		}
	} while (c);
	// we arrived at the end of the string

	// if everything is not closed
	if (stringDepth)
		throw error(forbiddenEolInString);

	if (currentStatement)
		pushStatement();
	
	hasTokenized = true;
}


/**
* Parse the string starting at the given location and push the right tokens
*/
void Parser::parseString(char opener) {
	bool isTemplate = (opener == '"' || opener == '\'');
	char c;
	while ((c = melody[position+length])) {
		if (isEndOfLine(c))
			throw error(forbiddenEolInString);
		else if (c == '\\') {
			if (isEndOfLine(melody[position+length+1]))
				throw error(forbiddenEolInString);
			length++;
		}
		else if (c == opener) {  // end of string
			if (length)
				push(Token::RawString);
			length = 1;
			push(Token::StringEnd);
			return;
		}
		else if (isTemplate && c == '{') {  // start of template
			push(Token::RawString);
			length = 1;
			push(Token::LeftCurlyBrace);

			stringOpeners += opener;
			stringDepth++;
			return;
		}
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in string");
		length++;
	}

	throw error(forbiddenEolInString);
}







/**
* Skip all empty lines and return the number of spaces/tabs before the first non-empty line
* The position will be automatically updated by this function
* This function is called after every end of line
*/
int Parser::getIndentLevel() {
	currentStatement = new Statement();
	int indentValue = 0;
	char c;

	while ((c = melody[position])) {
		if (isSpace(c)) {
			if (!indentType) {
				indentType = c;
				if (c == '\t')
					indentUnit = 1;
			}
			else if (c != indentType) {
				if (c == ' ')
					throw error("Unexpected use of space as indentation character. The file previously used tabulations.");
				else
					throw error("Unexpected use of tabulation as indentation character. The file previously used spaces.");
			}
			indentValue++;

			// if indent is locked we get the line and continue
			if (indentLock && indentUnit && indentLock == indentValue / indentUnit) {
				position++;
				pushLockedLine();
				indentValue = 0;
				continue;
			}
		}
		else if (isBlank(c)) {}
		else if (isEndOfLine(c)) indentValue = 0;
		else if (isControlCharacter(c)) {
			cout << "coco ??" << endl;
			throw error("Forbidden control character!");
		}
		else {
			// first character met : we return the calculated indent
			if (indentUnit == 0) {
				indentUnit = indentValue;
				if (indentLock == 1) {  // if locked indent at 1 unit
					position++;
					pushLockedLine();
					indentValue = 0;
					continue;
				}
			}
			break;
		}

		position++;
	}

	// we check space indent is correct
	if (indentValue && (indentValue % indentUnit)) {
		string msg = "Incorrect number of spaces for indentation. The file previously used ";

		if (indentUnit == 1) msg += "1 space";
		else msg += to_string(indentUnit) + " spaces";

		msg += " as an indentation unit. You indented with ";

		if (indentValue == 1) msg += "1 space";
		else msg += to_string(indentValue) + " spaces";

		msg += ".";
		throw error(msg);
	}

	// we update indent
	if (indentValue)
		indentValue /= indentUnit;
	if (indentLock)
		indentLock = 0;

	if (indentValue > indentLevel) {
		if (indentValue > indentLevel + 1) {  // we check the indentation-up is correct
			position--;
			throw error("Too strong indentation. You indented by "
				+ to_string(indentValue)
				+" levels."
			);
		}

		indent();
	}
	else while (indentValue < indentLevel)
		unindent();
	
	return indentValue;
}



/**
* Push from the current position to the end of line
*/
void Parser::pushRawLine(Token::Type tokenType) {
	length = getLineLength(melody, position);
	push(tokenType);
}


/**
* Push from the current position to the end of line
*/
void Parser::pushLockedLine() {
	pushRawLine(lockType == LockType::Comment ? Token::SubComment : Token::RawString);
	pushStatement();
	currentStatement = new Statement();
	position++;
}


/**
* Parse a glob or a regular expression.
* Example of regex : //.../opt?/
* Example of glob : ||...|opt?|
*/
void Parser::parseRegexOrGlob(Token::Type type) {
	const bool isRegex = (type == Token::RegexStart);
	const char closer = (isRegex ? '/' : '|');
	char c;

	// main regex/glob content
	while ((c = melody[position+length])) {
		if (c == closer)
			break;
		else if (isEndOfLine(c))
			throw error(forbiddenEolInRegex);
		else if (c == '\\') {
			if (isEndOfLine(melody[position+length+1]))
				throw error(forbiddenEolInRegex);
			length++;
		}
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in regex or glob");
		length++;
	}
	if (!c) throw error(forbiddenEolInRegex);
	push(Token::RegexOrGlobContent);
	length = 1;
	push(Token::RegexOrGlobEnd);

	// regex/glob option
	while ((c = melody[position+length])) {
		if (c == closer) break;
		else if (isEndOfLine(c))
			throw error(forbiddenEolInRegex);
		else if (!isBlank(c) && isControlCharacter(c))
			throw error("Forbidden control character in regex or glob");
		length++;
	}
	if (!c) throw error(forbiddenEolInRegex);
	push(Token::RegexOrGlobOption);
	length = 1;
	push(Token::LeftParenthesis);
}



/**
 * Parse a body (vector of statements) and return a BodyNode
 */
Node* Parser::parseBody(const Body& body) {
	auto root = new Node();
	for (auto statement : body) {
		position = 0;
		root->assimilate(parseStatement(statement));
	}
	return root;
}


/**
 * Parse the current statement at the current position
 * Return a new node, the root of the statement
 */
Node* Parser::parseStatement(Statement* statement, Token::Type groupType) {
	if (position >= statement->size())
		return NULL;
	
	Node* leftNode;
	Node* rightNode = NULL;
	Node* bodyNode = NULL;
	Token* token;
	Token* lastToken;
	int leftGlue;
	int rightGlue;
	Token::Type stopAtToken = getStopToken(groupType);
	vector<Node*> stack;

	// we check if the last token needs to be transformed
	lastToken = &statement->back();
	if (lastToken->glue() & Glue::TransformAtEndOfStatement)
		lastToken->incrementType();

	// we create the left node from the first token
	token = &statement->at(position++);

	// we check if the first token needs to be transformed
	if (position == 1 && (token->glue() & Glue::TransformAtStartOfStatement))
		token->incrementType();

	checkChaining(NULL, token);
	leftGlue = token->glue();
	leftNode = new Node(token);

	if (leftGlue & Glue::Body)  // if the node can have a body
		bodyNode = leftNode;

	if (token->type == Token::Type::String)  // if the node is a template string
		parseTemplateString(statement, leftNode);
	else if (leftGlue & Glue::Group)  // if the first node is a group...
		leftNode->assimilate(parseStatement(statement, token->type));

	rightGlue = leftGlue;

	// we loop through all the tokens
	while (position < statement->size()) {
		lastToken = token;
		token = &statement->at(position++);
		checkChaining(lastToken, token);

		// we stop here if we have reached the end of a group
		if (stopAtToken == token->type) {
			checkChaining(lastToken, NULL);
			return stack.size()? stack[0] : leftNode;
		}
		
		rightGlue = token->glue();
		rightNode = new Node(token);

		// if the node can have a body
		if (rightGlue & Glue::Body) {
			if (!bodyNode)
				bodyNode = rightNode;
			else {
				if (bodyNode->glue() & Glue::OptionalBody)
					bodyNode = rightNode;
				else if (!(rightGlue & Glue::OptionalBody))
					throw "Two tokens in the same statement need a body";
			}
		}

		// if we have a group we find its content
		if (token->type == Token::Type::String)  // if the node is a template string
			parseTemplateString(statement, rightNode);
		else if (rightGlue & Glue::Group)
			rightNode->assimilate(parseStatement(statement, token->type));

		// both left and right glue to each other...
		if ((rightGlue & Glue::Left) && (leftGlue & Glue::Right)) {
			int rightPriority = token->priority();
			Node* parent = leftNode;
			stack.push_back(leftNode);
			bool isSingle = rightGlue & Glue::Single;

			while (rightPriority <= parent->priority()) {
				if (rightPriority == parent->priority()) {
					if (isSingle || token->type != parent->token->type)
						break;

					// fusion
					stack.pop_back();
					delete rightNode;
					leftNode = parent;
					goto next;
				}

				stack.pop_back();
				if (!stack.size()) {  // lowest priority of all stack
					leftNode = rightNode->assimilate(parent);
					goto next;
				}
				parent = stack.back();
			}

			leftNode = parent->cuckoldedBy(rightNode);
		}

		// only right is gluing
		else if (rightGlue & Glue::Left) {
			leftNode = rightNode->assimilate(leftNode);
		}

		// only left is gluing
		else if (leftGlue & Glue::Right) {
			leftNode->assimilate(rightNode);
			if (!(rightGlue & Glue::Assimilable)) {
				stack.push_back(leftNode);
				leftNode = rightNode;
			}
		}

		// none of them glue to each other : error
		else {
			cout << "Left token : " << leftNode->token->type << endl;
			cout << "Right token : " << rightNode->token->type << endl;
			throw "The left and right nodes don't glue";
		}

		next:
		leftGlue = leftNode->glue();
	}

	checkChaining(token, NULL);

	// we check if there is a body and if it can be consumed
	if (statement->hasBody()) {
		if (!bodyNode)
			throw "The body cannot be consumed";
		bodyNode->assimilate(parseBody(statement->body));
	}
	else if (bodyNode && !(bodyNode->glue() & Glue::OptionalBody)) {
		throw "A body is expected!";
	}

	return stack.size()? stack[0] : leftNode;
}


/**
 * Parse a template string
 * Return a single node containing the string and all its children
 */
void Parser::parseTemplateString(Statement* statement, Node* root) {
	Token* token;

	// the tokenization ensured there is a StringEnd token
	while ((token = &statement->at(position++))) switch (token->type) {
		case Token::Type::StringEnd:
			return;
		
		case Token::Type::RawString:
			root->assimilate(new Node(token));
			break;
		
		case Token::Type::LeftCurlyBrace:
			root->assimilate(parseStatement(statement, Token::Type::LeftCurlyBrace));
			break;
		
		default:
			throw "Unexpected token in string template";
	}
}



/**
 * Parse the whole melody
 */
Node* Parser::growTree() {
	if (!hasTokenized)
		tokenize();
	tree = parseBody(body());
	hasGrownTree = true;
	return tree;
}



/**
 * Return the stop token corresponding to a token group
 */
Token::Type Parser::getStopToken(Token::Type type) {
	switch (type) {
		case Token::Type::UnknownToken :
			return Token::Type::UnknownToken;

		case Token::Type::LeftParenthesis :  // (...)
			return Token::Type::RightParenthesis;

		case Token::Type::LeftCurlyBrace :  // {...}
			return Token::Type::RightCurlyBrace;

		case Token::Type::LeftBrace :  // [...]
		case Token::Type::LeftBraceNoLeft :
			return Token::Type::RightBrace;
		
		case Token::Type::String :  // "..." or '...'
			return Token::Type::StringEnd;

		case Token::Type::RegexStart :  // //...//
		case Token::Type::GlobStart :   // ||...||
			return Token::Type::RegexOrGlobEnd;
		
		default:
			return Token::Type::UnknownToken;
	}
}



/**
 * Check if two consecutive tokens can be next to each other
 */
void Parser::checkChaining(Token* left, Token* right) {
	if (!left) {
		int glue = right->glue();
		if (glue & Glue::Left) {
			if (glue & Glue::WeakLeft)
				right->incrementType();
			else
				throw "Missing expression before token";
		}
	}
	else if (!right) {
		int glue = left->glue();
		if ((glue & Glue::Right) && !(glue & Glue::WeakRight)) {
			cout << "Token : " << left->type << endl;
			throw "Missing expression after token";
		}
	}
	else {
		int leftGlue = left->glue();
		int rightGlue = right->glue();
		if ((leftGlue & Glue::Right) && (rightGlue & Glue::Left)) {
			if (rightGlue & Glue::WeakLeft) 
				right->incrementType();
			else
				throw "Glue conflict : both tokens glue to each other";
		}
	}
}




// print a statement
void Parser::print(Statement* statement, int depth) {
	// we print all the tokens of the statement
	for (auto& token : *statement) {
		cout
		<< string(depth * 3, ' ')  // indentation
		<< Ink::cyan            << "type  "
		<< Ink::brightCyan      << token.type
		<< Ink::green           << string(10 - to_string(token.type).length(), ' ')
		                        << " position  "
		<< Ink::brightGreen     << token.position
		<< Ink::magenta         << string(10 - to_string(token.position).length(), ' ')
		                        << " length  "
		<< Ink::brightMagenta   << token.length

		<< string(10 - to_string(token.length).length(), ' ')
		<< coloredToken(token)
		<< Font::reset
		<< endl;
	}

	cout << endl;

	// and we print the body
	for (auto statement : statement->body)
		print(statement, depth + 1);
}


// print a node (and all its subnodes if it has)
void Parser::print(Node* node, int depth) {
	if (!node) return;
	
	if (node->token)
		cout << coloredToken(node->token) << endl;
	else
		cout << Ink::purple << Font::italic << "[Body]" << Font::reset << endl;

	for (Node* child : node->children) {
		cout << string(depth * 2, ' ') << "| ";
		print(child, depth + 1);
	}
}


// print a colored token
string Parser::coloredToken(const Token& token) {
	string content = extract(token);

	if (token.type > Token::KEYWORDS)  // keyword
		return Ink::red + content + Font::reset;

	if (token.type > Token::SYMBOLS)
		return Ink::yellow + content + Font::reset;

	switch (token.type) {
		case Token::Number :
			return Ink::white + content + Font::reset;

		case Token::Identifier :
			return Ink::white + content + Font::reset;

		case Token::String :
		case Token::RawString :
		case Token::StringEnd :
			return Ink::green + content + Font::reset;

		case Token::Comment :
		case Token::SubComment :
		case Token::Checkpoint :
			return Ink::cyan + content + Font::reset;
		
		default:
			return content;
	}
}


string Parser::coloredToken(Token* token) {
	return coloredToken(*token);
}


void Parser::printTokens() {  // print all tokens
	for (Statement* statement : *scope[0])
		print(statement);
}

void Parser::printTree() {  // print all tokens
	print(tree);
}

string Parser::error(string msg) {
	return prettyError(melody, msg, position, length);
};

string Parser::extract(const Token& token) {
	return string(melody + token.position, token.length);
}

string Parser::extract(Token* token) {
	return string(melody + token->position, token->length);
}

string Parser::extract(Node* node) {
	return string(melody + node->token->position, node->token->length);
}

}

// the walkers
// #include "walker/Validate.hpp"