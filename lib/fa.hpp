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

namespace Fa {

// -- PARSER --
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
	/* 6. CapitalIdentifier */    { Glue::Assimilable },
	/* 7. String */               { Glue::Group },
	/* 8. RawString */            { Glue::Assimilable },
	/* 9. StringEnd */            {  },
	/* 10. SYMBOLS */             {  },
	/* 11. LeftParenthesis */     { Glue::Assimilable | Glue::Group },
	/* 12. RegexStart */          { Glue::Assimilable | Glue::Group },
	/* 13. GlobStart */           { Glue::Assimilable | Glue::Group },
	/* 14. LeftBrace */           { Glue::Assimilable | Glue::Group | Glue::WeakLeft },
	/* 15. LeftBraceNoLeft */     { Glue::Assimilable | Glue::Group },
	/* 16. Backslash */           { Glue::Left | Glue::Right, 30 },
	/* 17. DoubleBackslash */     { Glue::Left | Glue::Right, 30 },
	/* 18. Equal */               { Glue::Left | Glue::Right | Glue::Single, 6 },
	/* 19. Colon */               { Glue::Single | Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 7 },
	/* 20. ColonBody */           { Glue::Left | Glue::Body },
	/* 21. LeftCurlyBrace */      { Glue::Assimilable | Glue::Group },
	/* 22. Dot */                 { Glue::Left | Glue::Right, 40 },
	/* 23. StaticProperty */      { Glue::Left | Glue::Right, 40 },
	/* 24. Comma */               { Glue::Left | Glue::Right, 2 },
	/* 25. Apostrophe */          { Glue::Assimilable | Glue::Group },
	/* 26. Quote */               { Glue::Assimilable | Glue::Group },
	/* 27. Accent */              { Glue::Assimilable | Glue::Group },
	/* 28. Asterisk */            { Glue::Left | Glue::Right, 30 },
	/* 29. Divide */              { Glue::Left | Glue::Right, 30 },
	/* 30. Circumflex */          { Glue::Left | Glue::Right, 18 },
	/* 31. Plus */                { Glue::Left | Glue::Right | Glue::WeakLeft, 28 },
	/* 32. PlusRight */           { Glue::Right, 31 },
	/* 33. Minus */               { Glue::Left | Glue::Right | Glue::WeakLeft, 29 },
	/* 34. MinusRight */          { Glue::Right, 31 },
	/* 35. QuestionMark */        { Glue::Left },
	/* 36. Tilde */               { Glue::Right },
	/* 37. DoubleEqual */         { Glue::Left | Glue::Right, 22 },
	/* 38. NotEqual */            { Glue::Left | Glue::Right, 22 },
	/* 39. Equivalent */          { Glue::Left | Glue::Right, 22 },
	/* 40. LesserOrEqual */       { Glue::Left | Glue::Right, 22 },
	/* 41. GreaterOrEqual */      { Glue::Left | Glue::Right, 22 },
	/* 42. InputArrow */          { Glue::Left | Glue::Right, 8 },
	/* 43. OutputArrow */         { Glue::Left | Glue::Right, 8 },
	/* 44. Percent */             { Glue::Left | Glue::Right, 31 },
	/* 45. Extract */             { Glue::Left | Glue::Right, 6 },
	/* 46. Insert */              { Glue::Left | Glue::Right, 6 },
	/* 47. DoubleDot */           { Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement, 40 },
	/* 48. DoubleDotBody */       { Glue::Left | Glue::Body, 40 },
	/* 49. TripleDot */           { Glue::Right | Glue::TransformAtEndOfStatement },
	/* 50. TripleDotBody */       { Glue::Left | Glue::Body },
	/* 51. MultiLineString */     { Glue::Assimilable },
	/* 52. PlusPlus */            { Glue::Left, 38 },
	/* 53. MinusMinus */          { Glue::Left, 38 },
	/* 54. Power */               { Glue::Left | Glue::Right, 32 },
	/* 55. PlusEqual */           { Glue::Left | Glue::Right, 6 },
	/* 56. MinusEqual */          { Glue::Left | Glue::Right, 6 },
	/* 57. TimesEqual */          { Glue::Left | Glue::Right, 6 },
	/* 58. DivideEqual */         { Glue::Left | Glue::Right, 6 },
	/* 59. IntegerDivideEqual */  { Glue::Left | Glue::Right, 6 },
	/* 60. LesserThan */          { Glue::Left | Glue::Right, 24 },
	/* 61. GreaterThan */         { Glue::Left | Glue::Right, 24 },
	/* 62. SendTo */              { Glue::Left | Glue::Right },
	/* 63. Pipe */                { Glue::Left | Glue::Right },
	/* 64. At */                  { Glue::Right, 50 },
	/* 65. Semicolon */           {  },
	/* 66. RightParenthesis */    {  },
	/* 67. RegexOrGlobContent */  {  },
	/* 68. RegexOrGlobEnd */      {  },
	/* 69. RegexOrGlobOption */   {  },
	/* 70. RightBrace */          {  },
	/* 71. RightCurlyBrace */     {  },
	/* 72. UserDefinedSymbol */   {  },
	/* 73. Generic */             { Glue::WeakLeft | Glue::Group },
	/* 74. DefaultGeneric */      { Glue::Group },
	/* 75. KEYWORDS */            {  },
	/* 76. Let */                 { Glue::Right },
	/* 77. Super */               { Glue::Assimilable },
	/* 78. Print */               { Glue::Right },
	/* 79. Use */                 { Glue::Right },
	/* 80. Import */              { Glue::Right },
	/* 81. Export */              { Glue::Right },
	/* 82. From */                { Glue::Left | Glue::Right },
	/* 83. Extends */             { Glue::Left | Glue::Right },
	/* 84. IfComprehension */     { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 85. If */                  { Glue::Right | Glue::Body },
	/* 86. ElseComprehension */   { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 87. Else */                { Glue::Body },
	/* 88. ElseIf */              { Glue::Right | Glue::Body },
	/* 89. Then */                { Glue::Right },
	/* 90. Do */                  { Glue::Right },
	/* 91. WhileComprehension */  { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 92. While */               { Glue::Right | Glue::Body },
	/* 93. RepeatComprehension */ { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 94. Repeat */              { Glue::Right | Glue::Body },
	/* 95. ForComprehension */    { Glue::Left | Glue::Right | Glue::TransformAtStartOfStatement },
	/* 96. For */                 { Glue::Right | Glue::Body },
	/* 97. In */                  { Glue::Left | Glue::Right },
	/* 98. When */                { Glue::Left | Glue::Right },
	/* 99. And */                 { Glue::Left | Glue::Right, 12 },
	/* 100. Or */                 { Glue::Left | Glue::Right, 10 },
	/* 101. Xor */                { Glue::Left | Glue::Right },
	/* 102. Modulo */             { Glue::Left | Glue::Right },
	/* 103. Is */                 { Glue::Left | Glue::Right | Glue::TransformAtEndOfStatement },
	/* 104. IsStart */            { Glue::Right | Glue::OptionalBody },
	/* 105. To */                 { Glue::Left | Glue::Right },
	/* 106. Not */                { Glue::Right },
	/* 107. Isnt */               { Glue::Left | Glue::Right },
	/* 108. Return */             { Glue::Right },
	/* 109. Continue */           { Glue::Right },
	/* 110. Break */              { Glue::Right },
	/* 111. Try */                { Glue::Right },
	/* 112. Catch */              { Glue::Right },
	/* 113. Finally */            { Glue::Right },
	/* 114. Throw */              { Glue::Right },
	/* 115. Async */              { Glue::Right },
	/* 116. Await */              { Glue::Right },
	/* 117. Yield */              { Glue::Right },
	/* 118. Nil */                { Glue::Assimilable },
	/* 119. True */               { Glue::Assimilable },
	/* 120. False */              { Glue::Assimilable },
	/* 121. Infinity */           { Glue::Assimilable },
	/* 122. Global */             { Glue::Right },
	/* 123. Override */           { Glue::Right },
	/* 124. Final */              { Glue::Right },
	/* 125. Const */              { Glue::Right },
	/* 126. Private */            { Glue::Right },
	/* 127. Static */             { Glue::Right },
	/* 128. Class */              { Glue::Right | Glue::OptionalBody },
	/* 129. Enum */               { Glue::Right | Glue::OptionalBody },
	/* 130. Abstract */           { Glue::Right | Glue::OptionalBody },
	/* 131. Interface */          { Glue::Right | Glue::OptionalBody },
	/* 132. Structure */          { Glue::Right | Glue::OptionalBody },
	/* 133. Unique */             { Glue::Right | Glue::OptionalBody },
	/* 134. Exclamation */        { Glue::Right },
	/* 135. Self */               { Glue::Assimilable }
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
		CapitalIdentifier,
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
		StaticProperty,
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
		Generic,
		DefaultGeneric,
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
 - 'assimilate' : add a child node
 - 'cuckold' : a node cuckolds another node when :
	1. It becomes the father of the node's last child
	2. He then becomes the last child of the node
 * This algorithm makes Fa sort of a grammar-less language.
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
		return token->priority();
	}
	inline int glue() {
		return token->glue();
	}
	inline Token::Type type() {
		return token->type;
	}

	Node* lastChild() {
		return children.size() ? children.back() : NULL;
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

	static Node* from(Token* from);
};

struct LetNode : Node {
	struct {
		Node* identifier { NULL };
		Node* type { NULL };
		Node* value { NULL };
	} data;

	LetNode() : Node() {}
	LetNode(Token* from) : Node(from) {}
};


Node* Node::from(Token* from) {
	switch (from->type) {
		case Token::Let: return new LetNode(from);
		default: return new Node(from);
	}
}


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
		{ "::", Token::StaticProperty },
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
		{ "\"", Token::String },
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
	leftNode = Node::from(token);

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
		rightNode = Node::from(token);

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
			root->assimilate(Node::from(token));
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

// -- WALKERS --

using NodeCallback = void (*) (Node*);


namespace Walkers {
	/**
	* The abstract walker class every walker should inherit from
	*/
	struct BaseWalker {
		Parser* parser;

		BaseWalker(Parser& _parser) {
			parser = &_parser;
		}

		inline string value(Node* node) {
			return parser->extract(node);
		}

		virtual void start() {
			if (!parser->tree)
				throw "No syntax tree to walk through";
			walk(parser->tree);
		}


		virtual void walk(Node* node) {
			for (Node* child : parser->tree->children)
				visit(child);
		}

		// virtual methods
		virtual void visitUnknownToken(Node* node) {std::cout << "Visit UnknownToken from BaseWalker" << std::endl;}
		virtual void visitNumber(Node* node) {std::cout << "Visit Number from BaseWalker" << std::endl;}
		virtual void visitComment(Node* node) {std::cout << "Visit Comment from BaseWalker" << std::endl;}
		virtual void visitSubComment(Node* node) {std::cout << "Visit SubComment from BaseWalker" << std::endl;}
		virtual void visitCheckpoint(Node* node) {std::cout << "Visit Checkpoint from BaseWalker" << std::endl;}
		virtual void visitIdentifier(Node* node) {std::cout << "Visit Identifier from BaseWalker" << std::endl;}
		virtual void visitCapitalIdentifier(Node* node) {std::cout << "Visit CapitalIdentifier from BaseWalker" << std::endl;}
		virtual void visitString(Node* node) {std::cout << "Visit String from BaseWalker" << std::endl;}
		virtual void visitRawString(Node* node) {std::cout << "Visit RawString from BaseWalker" << std::endl;}
		virtual void visitStringEnd(Node* node) {std::cout << "Visit StringEnd from BaseWalker" << std::endl;}
		virtual void visitSYMBOLS(Node* node) {std::cout << "Visit SYMBOLS from BaseWalker" << std::endl;}
		virtual void visitLeftParenthesis(Node* node) {std::cout << "Visit LeftParenthesis from BaseWalker" << std::endl;}
		virtual void visitRegexStart(Node* node) {std::cout << "Visit RegexStart from BaseWalker" << std::endl;}
		virtual void visitGlobStart(Node* node) {std::cout << "Visit GlobStart from BaseWalker" << std::endl;}
		virtual void visitLeftBrace(Node* node) {std::cout << "Visit LeftBrace from BaseWalker" << std::endl;}
		virtual void visitLeftBraceNoLeft(Node* node) {std::cout << "Visit LeftBraceNoLeft from BaseWalker" << std::endl;}
		virtual void visitBackslash(Node* node) {std::cout << "Visit Backslash from BaseWalker" << std::endl;}
		virtual void visitDoubleBackslash(Node* node) {std::cout << "Visit DoubleBackslash from BaseWalker" << std::endl;}
		virtual void visitEqual(Node* node) {std::cout << "Visit Equal from BaseWalker" << std::endl;}
		virtual void visitColon(Node* node) {std::cout << "Visit Colon from BaseWalker" << std::endl;}
		virtual void visitColonBody(Node* node) {std::cout << "Visit ColonBody from BaseWalker" << std::endl;}
		virtual void visitLeftCurlyBrace(Node* node) {std::cout << "Visit LeftCurlyBrace from BaseWalker" << std::endl;}
		virtual void visitDot(Node* node) {std::cout << "Visit Dot from BaseWalker" << std::endl;}
		virtual void visitStaticProperty(Node* node) {std::cout << "Visit StaticProperty from BaseWalker" << std::endl;}
		virtual void visitComma(Node* node) {std::cout << "Visit Comma from BaseWalker" << std::endl;}
		virtual void visitApostrophe(Node* node) {std::cout << "Visit Apostrophe from BaseWalker" << std::endl;}
		virtual void visitQuote(Node* node) {std::cout << "Visit Quote from BaseWalker" << std::endl;}
		virtual void visitAccent(Node* node) {std::cout << "Visit Accent from BaseWalker" << std::endl;}
		virtual void visitAsterisk(Node* node) {std::cout << "Visit Asterisk from BaseWalker" << std::endl;}
		virtual void visitDivide(Node* node) {std::cout << "Visit Divide from BaseWalker" << std::endl;}
		virtual void visitCircumflex(Node* node) {std::cout << "Visit Circumflex from BaseWalker" << std::endl;}
		virtual void visitPlus(Node* node) {std::cout << "Visit Plus from BaseWalker" << std::endl;}
		virtual void visitPlusRight(Node* node) {std::cout << "Visit PlusRight from BaseWalker" << std::endl;}
		virtual void visitMinus(Node* node) {std::cout << "Visit Minus from BaseWalker" << std::endl;}
		virtual void visitMinusRight(Node* node) {std::cout << "Visit MinusRight from BaseWalker" << std::endl;}
		virtual void visitQuestionMark(Node* node) {std::cout << "Visit QuestionMark from BaseWalker" << std::endl;}
		virtual void visitTilde(Node* node) {std::cout << "Visit Tilde from BaseWalker" << std::endl;}
		virtual void visitDoubleEqual(Node* node) {std::cout << "Visit DoubleEqual from BaseWalker" << std::endl;}
		virtual void visitNotEqual(Node* node) {std::cout << "Visit NotEqual from BaseWalker" << std::endl;}
		virtual void visitEquivalent(Node* node) {std::cout << "Visit Equivalent from BaseWalker" << std::endl;}
		virtual void visitLesserOrEqual(Node* node) {std::cout << "Visit LesserOrEqual from BaseWalker" << std::endl;}
		virtual void visitGreaterOrEqual(Node* node) {std::cout << "Visit GreaterOrEqual from BaseWalker" << std::endl;}
		virtual void visitInputArrow(Node* node) {std::cout << "Visit InputArrow from BaseWalker" << std::endl;}
		virtual void visitOutputArrow(Node* node) {std::cout << "Visit OutputArrow from BaseWalker" << std::endl;}
		virtual void visitPercent(Node* node) {std::cout << "Visit Percent from BaseWalker" << std::endl;}
		virtual void visitExtract(Node* node) {std::cout << "Visit Extract from BaseWalker" << std::endl;}
		virtual void visitInsert(Node* node) {std::cout << "Visit Insert from BaseWalker" << std::endl;}
		virtual void visitDoubleDot(Node* node) {std::cout << "Visit DoubleDot from BaseWalker" << std::endl;}
		virtual void visitDoubleDotBody(Node* node) {std::cout << "Visit DoubleDotBody from BaseWalker" << std::endl;}
		virtual void visitTripleDot(Node* node) {std::cout << "Visit TripleDot from BaseWalker" << std::endl;}
		virtual void visitTripleDotBody(Node* node) {std::cout << "Visit TripleDotBody from BaseWalker" << std::endl;}
		virtual void visitMultiLineString(Node* node) {std::cout << "Visit MultiLineString from BaseWalker" << std::endl;}
		virtual void visitPlusPlus(Node* node) {std::cout << "Visit PlusPlus from BaseWalker" << std::endl;}
		virtual void visitMinusMinus(Node* node) {std::cout << "Visit MinusMinus from BaseWalker" << std::endl;}
		virtual void visitPower(Node* node) {std::cout << "Visit Power from BaseWalker" << std::endl;}
		virtual void visitPlusEqual(Node* node) {std::cout << "Visit PlusEqual from BaseWalker" << std::endl;}
		virtual void visitMinusEqual(Node* node) {std::cout << "Visit MinusEqual from BaseWalker" << std::endl;}
		virtual void visitTimesEqual(Node* node) {std::cout << "Visit TimesEqual from BaseWalker" << std::endl;}
		virtual void visitDivideEqual(Node* node) {std::cout << "Visit DivideEqual from BaseWalker" << std::endl;}
		virtual void visitIntegerDivideEqual(Node* node) {std::cout << "Visit IntegerDivideEqual from BaseWalker" << std::endl;}
		virtual void visitLesserThan(Node* node) {std::cout << "Visit LesserThan from BaseWalker" << std::endl;}
		virtual void visitGreaterThan(Node* node) {std::cout << "Visit GreaterThan from BaseWalker" << std::endl;}
		virtual void visitSendTo(Node* node) {std::cout << "Visit SendTo from BaseWalker" << std::endl;}
		virtual void visitPipe(Node* node) {std::cout << "Visit Pipe from BaseWalker" << std::endl;}
		virtual void visitAt(Node* node) {std::cout << "Visit At from BaseWalker" << std::endl;}
		virtual void visitSemicolon(Node* node) {std::cout << "Visit Semicolon from BaseWalker" << std::endl;}
		virtual void visitRightParenthesis(Node* node) {std::cout << "Visit RightParenthesis from BaseWalker" << std::endl;}
		virtual void visitRegexOrGlobContent(Node* node) {std::cout << "Visit RegexOrGlobContent from BaseWalker" << std::endl;}
		virtual void visitRegexOrGlobEnd(Node* node) {std::cout << "Visit RegexOrGlobEnd from BaseWalker" << std::endl;}
		virtual void visitRegexOrGlobOption(Node* node) {std::cout << "Visit RegexOrGlobOption from BaseWalker" << std::endl;}
		virtual void visitRightBrace(Node* node) {std::cout << "Visit RightBrace from BaseWalker" << std::endl;}
		virtual void visitRightCurlyBrace(Node* node) {std::cout << "Visit RightCurlyBrace from BaseWalker" << std::endl;}
		virtual void visitUserDefinedSymbol(Node* node) {std::cout << "Visit UserDefinedSymbol from BaseWalker" << std::endl;}
		virtual void visitGeneric(Node* node) {std::cout << "Visit Generic from BaseWalker" << std::endl;}
		virtual void visitDefaultGeneric(Node* node) {std::cout << "Visit DefaultGeneric from BaseWalker" << std::endl;}
		virtual void visitKEYWORDS(Node* node) {std::cout << "Visit KEYWORDS from BaseWalker" << std::endl;}
		virtual void visitLet(Node* node) {std::cout << "Visit Let from BaseWalker" << std::endl;}
		virtual void visitSuper(Node* node) {std::cout << "Visit Super from BaseWalker" << std::endl;}
		virtual void visitPrint(Node* node) {std::cout << "Visit Print from BaseWalker" << std::endl;}
		virtual void visitUse(Node* node) {std::cout << "Visit Use from BaseWalker" << std::endl;}
		virtual void visitImport(Node* node) {std::cout << "Visit Import from BaseWalker" << std::endl;}
		virtual void visitExport(Node* node) {std::cout << "Visit Export from BaseWalker" << std::endl;}
		virtual void visitFrom(Node* node) {std::cout << "Visit From from BaseWalker" << std::endl;}
		virtual void visitExtends(Node* node) {std::cout << "Visit Extends from BaseWalker" << std::endl;}
		virtual void visitIfComprehension(Node* node) {std::cout << "Visit IfComprehension from BaseWalker" << std::endl;}
		virtual void visitIf(Node* node) {std::cout << "Visit If from BaseWalker" << std::endl;}
		virtual void visitElseComprehension(Node* node) {std::cout << "Visit ElseComprehension from BaseWalker" << std::endl;}
		virtual void visitElse(Node* node) {std::cout << "Visit Else from BaseWalker" << std::endl;}
		virtual void visitElseIf(Node* node) {std::cout << "Visit ElseIf from BaseWalker" << std::endl;}
		virtual void visitThen(Node* node) {std::cout << "Visit Then from BaseWalker" << std::endl;}
		virtual void visitDo(Node* node) {std::cout << "Visit Do from BaseWalker" << std::endl;}
		virtual void visitWhileComprehension(Node* node) {std::cout << "Visit WhileComprehension from BaseWalker" << std::endl;}
		virtual void visitWhile(Node* node) {std::cout << "Visit While from BaseWalker" << std::endl;}
		virtual void visitRepeatComprehension(Node* node) {std::cout << "Visit RepeatComprehension from BaseWalker" << std::endl;}
		virtual void visitRepeat(Node* node) {std::cout << "Visit Repeat from BaseWalker" << std::endl;}
		virtual void visitForComprehension(Node* node) {std::cout << "Visit ForComprehension from BaseWalker" << std::endl;}
		virtual void visitFor(Node* node) {std::cout << "Visit For from BaseWalker" << std::endl;}
		virtual void visitIn(Node* node) {std::cout << "Visit In from BaseWalker" << std::endl;}
		virtual void visitWhen(Node* node) {std::cout << "Visit When from BaseWalker" << std::endl;}
		virtual void visitAnd(Node* node) {std::cout << "Visit And from BaseWalker" << std::endl;}
		virtual void visitOr(Node* node) {std::cout << "Visit Or from BaseWalker" << std::endl;}
		virtual void visitXor(Node* node) {std::cout << "Visit Xor from BaseWalker" << std::endl;}
		virtual void visitModulo(Node* node) {std::cout << "Visit Modulo from BaseWalker" << std::endl;}
		virtual void visitIs(Node* node) {std::cout << "Visit Is from BaseWalker" << std::endl;}
		virtual void visitIsStart(Node* node) {std::cout << "Visit IsStart from BaseWalker" << std::endl;}
		virtual void visitTo(Node* node) {std::cout << "Visit To from BaseWalker" << std::endl;}
		virtual void visitNot(Node* node) {std::cout << "Visit Not from BaseWalker" << std::endl;}
		virtual void visitIsnt(Node* node) {std::cout << "Visit Isnt from BaseWalker" << std::endl;}
		virtual void visitReturn(Node* node) {std::cout << "Visit Return from BaseWalker" << std::endl;}
		virtual void visitContinue(Node* node) {std::cout << "Visit Continue from BaseWalker" << std::endl;}
		virtual void visitBreak(Node* node) {std::cout << "Visit Break from BaseWalker" << std::endl;}
		virtual void visitTry(Node* node) {std::cout << "Visit Try from BaseWalker" << std::endl;}
		virtual void visitCatch(Node* node) {std::cout << "Visit Catch from BaseWalker" << std::endl;}
		virtual void visitFinally(Node* node) {std::cout << "Visit Finally from BaseWalker" << std::endl;}
		virtual void visitThrow(Node* node) {std::cout << "Visit Throw from BaseWalker" << std::endl;}
		virtual void visitAsync(Node* node) {std::cout << "Visit Async from BaseWalker" << std::endl;}
		virtual void visitAwait(Node* node) {std::cout << "Visit Await from BaseWalker" << std::endl;}
		virtual void visitYield(Node* node) {std::cout << "Visit Yield from BaseWalker" << std::endl;}
		virtual void visitNil(Node* node) {std::cout << "Visit Nil from BaseWalker" << std::endl;}
		virtual void visitTrue(Node* node) {std::cout << "Visit True from BaseWalker" << std::endl;}
		virtual void visitFalse(Node* node) {std::cout << "Visit False from BaseWalker" << std::endl;}
		virtual void visitInfinity(Node* node) {std::cout << "Visit Infinity from BaseWalker" << std::endl;}
		virtual void visitGlobal(Node* node) {std::cout << "Visit Global from BaseWalker" << std::endl;}
		virtual void visitOverride(Node* node) {std::cout << "Visit Override from BaseWalker" << std::endl;}
		virtual void visitFinal(Node* node) {std::cout << "Visit Final from BaseWalker" << std::endl;}
		virtual void visitConst(Node* node) {std::cout << "Visit Const from BaseWalker" << std::endl;}
		virtual void visitPrivate(Node* node) {std::cout << "Visit Private from BaseWalker" << std::endl;}
		virtual void visitStatic(Node* node) {std::cout << "Visit Static from BaseWalker" << std::endl;}
		virtual void visitClass(Node* node) {std::cout << "Visit Class from BaseWalker" << std::endl;}
		virtual void visitEnum(Node* node) {std::cout << "Visit Enum from BaseWalker" << std::endl;}
		virtual void visitAbstract(Node* node) {std::cout << "Visit Abstract from BaseWalker" << std::endl;}
		virtual void visitInterface(Node* node) {std::cout << "Visit Interface from BaseWalker" << std::endl;}
		virtual void visitStructure(Node* node) {std::cout << "Visit Structure from BaseWalker" << std::endl;}
		virtual void visitUnique(Node* node) {std::cout << "Visit Unique from BaseWalker" << std::endl;}
		virtual void visitExclamation(Node* node) {std::cout << "Visit Exclamation from BaseWalker" << std::endl;}
		virtual void visitSelf(Node* node) {std::cout << "Visit Self from BaseWalker" << std::endl;}

		// node visiter
		virtual void visit(Node* node) {
			switch (node->type()) {
				case Token::Type::UnknownToken: return visitUnknownToken(node);
				case Token::Type::Number: return visitNumber(node);
				case Token::Type::Comment: return visitComment(node);
				case Token::Type::SubComment: return visitSubComment(node);
				case Token::Type::Checkpoint: return visitCheckpoint(node);
				case Token::Type::Identifier: return visitIdentifier(node);
				case Token::Type::CapitalIdentifier: return visitCapitalIdentifier(node);
				case Token::Type::String: return visitString(node);
				case Token::Type::RawString: return visitRawString(node);
				case Token::Type::StringEnd: return visitStringEnd(node);
				case Token::Type::SYMBOLS: return visitSYMBOLS(node);
				case Token::Type::LeftParenthesis: return visitLeftParenthesis(node);
				case Token::Type::RegexStart: return visitRegexStart(node);
				case Token::Type::GlobStart: return visitGlobStart(node);
				case Token::Type::LeftBrace: return visitLeftBrace(node);
				case Token::Type::LeftBraceNoLeft: return visitLeftBraceNoLeft(node);
				case Token::Type::Backslash: return visitBackslash(node);
				case Token::Type::DoubleBackslash: return visitDoubleBackslash(node);
				case Token::Type::Equal: return visitEqual(node);
				case Token::Type::Colon: return visitColon(node);
				case Token::Type::ColonBody: return visitColonBody(node);
				case Token::Type::LeftCurlyBrace: return visitLeftCurlyBrace(node);
				case Token::Type::Dot: return visitDot(node);
				case Token::Type::StaticProperty: return visitStaticProperty(node);
				case Token::Type::Comma: return visitComma(node);
				case Token::Type::Apostrophe: return visitApostrophe(node);
				case Token::Type::Quote: return visitQuote(node);
				case Token::Type::Accent: return visitAccent(node);
				case Token::Type::Asterisk: return visitAsterisk(node);
				case Token::Type::Divide: return visitDivide(node);
				case Token::Type::Circumflex: return visitCircumflex(node);
				case Token::Type::Plus: return visitPlus(node);
				case Token::Type::PlusRight: return visitPlusRight(node);
				case Token::Type::Minus: return visitMinus(node);
				case Token::Type::MinusRight: return visitMinusRight(node);
				case Token::Type::QuestionMark: return visitQuestionMark(node);
				case Token::Type::Tilde: return visitTilde(node);
				case Token::Type::DoubleEqual: return visitDoubleEqual(node);
				case Token::Type::NotEqual: return visitNotEqual(node);
				case Token::Type::Equivalent: return visitEquivalent(node);
				case Token::Type::LesserOrEqual: return visitLesserOrEqual(node);
				case Token::Type::GreaterOrEqual: return visitGreaterOrEqual(node);
				case Token::Type::InputArrow: return visitInputArrow(node);
				case Token::Type::OutputArrow: return visitOutputArrow(node);
				case Token::Type::Percent: return visitPercent(node);
				case Token::Type::Extract: return visitExtract(node);
				case Token::Type::Insert: return visitInsert(node);
				case Token::Type::DoubleDot: return visitDoubleDot(node);
				case Token::Type::DoubleDotBody: return visitDoubleDotBody(node);
				case Token::Type::TripleDot: return visitTripleDot(node);
				case Token::Type::TripleDotBody: return visitTripleDotBody(node);
				case Token::Type::MultiLineString: return visitMultiLineString(node);
				case Token::Type::PlusPlus: return visitPlusPlus(node);
				case Token::Type::MinusMinus: return visitMinusMinus(node);
				case Token::Type::Power: return visitPower(node);
				case Token::Type::PlusEqual: return visitPlusEqual(node);
				case Token::Type::MinusEqual: return visitMinusEqual(node);
				case Token::Type::TimesEqual: return visitTimesEqual(node);
				case Token::Type::DivideEqual: return visitDivideEqual(node);
				case Token::Type::IntegerDivideEqual: return visitIntegerDivideEqual(node);
				case Token::Type::LesserThan: return visitLesserThan(node);
				case Token::Type::GreaterThan: return visitGreaterThan(node);
				case Token::Type::SendTo: return visitSendTo(node);
				case Token::Type::Pipe: return visitPipe(node);
				case Token::Type::At: return visitAt(node);
				case Token::Type::Semicolon: return visitSemicolon(node);
				case Token::Type::RightParenthesis: return visitRightParenthesis(node);
				case Token::Type::RegexOrGlobContent: return visitRegexOrGlobContent(node);
				case Token::Type::RegexOrGlobEnd: return visitRegexOrGlobEnd(node);
				case Token::Type::RegexOrGlobOption: return visitRegexOrGlobOption(node);
				case Token::Type::RightBrace: return visitRightBrace(node);
				case Token::Type::RightCurlyBrace: return visitRightCurlyBrace(node);
				case Token::Type::UserDefinedSymbol: return visitUserDefinedSymbol(node);
				case Token::Type::Generic: return visitGeneric(node);
				case Token::Type::DefaultGeneric: return visitDefaultGeneric(node);
				case Token::Type::KEYWORDS: return visitKEYWORDS(node);
				case Token::Type::Let: return visitLet(node);
				case Token::Type::Super: return visitSuper(node);
				case Token::Type::Print: return visitPrint(node);
				case Token::Type::Use: return visitUse(node);
				case Token::Type::Import: return visitImport(node);
				case Token::Type::Export: return visitExport(node);
				case Token::Type::From: return visitFrom(node);
				case Token::Type::Extends: return visitExtends(node);
				case Token::Type::IfComprehension: return visitIfComprehension(node);
				case Token::Type::If: return visitIf(node);
				case Token::Type::ElseComprehension: return visitElseComprehension(node);
				case Token::Type::Else: return visitElse(node);
				case Token::Type::ElseIf: return visitElseIf(node);
				case Token::Type::Then: return visitThen(node);
				case Token::Type::Do: return visitDo(node);
				case Token::Type::WhileComprehension: return visitWhileComprehension(node);
				case Token::Type::While: return visitWhile(node);
				case Token::Type::RepeatComprehension: return visitRepeatComprehension(node);
				case Token::Type::Repeat: return visitRepeat(node);
				case Token::Type::ForComprehension: return visitForComprehension(node);
				case Token::Type::For: return visitFor(node);
				case Token::Type::In: return visitIn(node);
				case Token::Type::When: return visitWhen(node);
				case Token::Type::And: return visitAnd(node);
				case Token::Type::Or: return visitOr(node);
				case Token::Type::Xor: return visitXor(node);
				case Token::Type::Modulo: return visitModulo(node);
				case Token::Type::Is: return visitIs(node);
				case Token::Type::IsStart: return visitIsStart(node);
				case Token::Type::To: return visitTo(node);
				case Token::Type::Not: return visitNot(node);
				case Token::Type::Isnt: return visitIsnt(node);
				case Token::Type::Return: return visitReturn(node);
				case Token::Type::Continue: return visitContinue(node);
				case Token::Type::Break: return visitBreak(node);
				case Token::Type::Try: return visitTry(node);
				case Token::Type::Catch: return visitCatch(node);
				case Token::Type::Finally: return visitFinally(node);
				case Token::Type::Throw: return visitThrow(node);
				case Token::Type::Async: return visitAsync(node);
				case Token::Type::Await: return visitAwait(node);
				case Token::Type::Yield: return visitYield(node);
				case Token::Type::Nil: return visitNil(node);
				case Token::Type::True: return visitTrue(node);
				case Token::Type::False: return visitFalse(node);
				case Token::Type::Infinity: return visitInfinity(node);
				case Token::Type::Global: return visitGlobal(node);
				case Token::Type::Override: return visitOverride(node);
				case Token::Type::Final: return visitFinal(node);
				case Token::Type::Const: return visitConst(node);
				case Token::Type::Private: return visitPrivate(node);
				case Token::Type::Static: return visitStatic(node);
				case Token::Type::Class: return visitClass(node);
				case Token::Type::Enum: return visitEnum(node);
				case Token::Type::Abstract: return visitAbstract(node);
				case Token::Type::Interface: return visitInterface(node);
				case Token::Type::Structure: return visitStructure(node);
				case Token::Type::Unique: return visitUnique(node);
				case Token::Type::Exclamation: return visitExclamation(node);
				case Token::Type::Self: return visitSelf(node);
			}
		}
	};
}


struct Definition {
	Node* node;
	Node* identifier;

	struct {
		bool shared { false };
		bool constant { false };
	} is;
};
/**
 * A Scope is a map of variable names and values.
 */
using Scope = map<string, Definition>;
namespace NodeData {

	struct Module {
		string name;
		Scope scope;
	};

	struct Let {

	};

	struct Class {
		Scope scope;
	};
}



namespace Validate {
   void This(Node* node);
   void Any(Node* node);
   void Let(Node* node);
	void Colon__declare(Node* node, Node*& identifier, Node*& type);
	void Equal__assign(Node* node, Node*& identifier, Node*& type, Node*& value);
	void Generic(Node* node);
	
	void Let(Node* node) {
		Node* _0 = node->children[0];
		Node*& identifier = ((LetNode*) node)->data.identifier;
		Node*& type = ((LetNode*) node)->data.type;
		Node*& value = ((LetNode*) node)->data.value;
		
		switch (_0->type()) {
			case Token::Identifier:
				Validate::Any(_0);
				identifier = _0;
				return;

			case Token::Colon:
				Validate::Colon__declare(_0, identifier, type);
				return;

			case Token::Equal:
				Validate::Equal__assign(_0, identifier, type, value);
				return;

			default:
				throw "Bad syntax";
		}
	}

	void Colon__declare(Node* node, Node*& identifier, Node*& type) {
		Node* _0 = node->children[0];
		Node* _1 = node->children[1];
		
		switch (_0->type()) {
			case Token::Identifier:
				Validate::Any(_0);
				identifier = _0;
				
				switch (_1->type()) {
					case Token::CapitalIdentifier:
					   Validate::Any(_1);
					   break;
					case Token::Generic:
					   Validate::Generic(_1);
					   break;
					default:
						throw "Invalid type";
				}
				type = _1;
				return;

			default:
				throw "Identifier expected";
		}
	}

	void Equal__assign(Node* node, Node*& identifier, Node*& type, Node*& value) {
		Node* _0 = node->children[0];
		Node* _1 = node->children[1];
		
		switch (_0->type()) {
			case Token::Identifier:
				Validate::Any(_0);
				value = _0;
				
				Validate::Any(_1);
				value = _1;
				return;

			case Token::Colon:
				Validate::Colon__declare(_0, identifier, type);
				
				Validate::Any(_1);
				value = _1;
				return;

			default:
				throw "Bad syntax";
		}
	}

	void Generic(Node* node) {
		Node* _0 = node->children[0];
		Node* _1 = node->children[1];
		
		switch (_0->type()) {
			case Token::CapitalIdentifier:
				Validate::Any(_0);
				
				switch (_1->type()) {
					case Token::CapitalIdentifier:
					   Validate::Any(_1);
					   break;
					case Token::Generic:
					   Validate::Generic(_1);
					   break;
					default:
						throw "Bad type";
				}
				return;

			default:
				throw "A type is expected";
		}
	}



   void This(Node* node) {
      switch (node->type()) {
         case Token::Let: return Validate::Let(node);
			case Token::Generic: return Validate::Generic(node);
         default: return Validate::Any(node);
      }
   }

   void Any(Node* node) {
      for (Node* node : node->children)
         Validate::This(node);
   }
}

namespace Walkers {

	/**
	 * This walker make sure the given tree is valid.
	 * Plus it feeds the `data` value of every node.
	 */
	struct Validator : BaseWalker {
		vector<Scope> scopes;
		inline const Scope& context() { return scopes.back(); }

		Validator(Parser& parser) : BaseWalker(parser) {
			auto moduleData = new NodeData::Module();
			parser.tree->data = (void*) moduleData;
			scopes.push_back(moduleData->scope);
		}


		// -- VISITERS
		// void visitLet(Node* node, bool shared=false, bool constant=false) {
		// 	// Syntax : a let expect a ':' or a '='
		// 	cout << "Visiting let" << endl;
		// 	auto child = node->children[0];
		// 	auto type = child->type();

		// 	// assignment
		// 	if (type == Token::Equal) {
		// 		auto [left, right] = child->children;

		// 		if (left->type() == Token::Colon) {
		// 			auto [identifier, type] = left->children;
		// 		}
		// 		else if (left->type() == Token::Identifier) {
		// 			context()[value(left)] = {
		// 				node, left,
		// 				shared, constant
		// 			};
		// 		}
		// 		else {
		// 			throw "Invalid identifier in let statement";
		// 		}
		// 	}

		// 	// declaration
		// 	else if (type == Token::Colon) {
		// 		auto [identifier] = child->children;
		// 		if (identifier->type() != Token::Identifier)
		// 			throw "A let statement expects an identifier";

		// 	}

		// 	// errors
		// 	else if (type == Token::Identifier) {
		// 		throw "A let statement expect a type or a value after the identifier";
		// 	}
		// 	else {
		// 		throw "Unexpected token after let statement";
		// 	}
		// }

	};
}

}
