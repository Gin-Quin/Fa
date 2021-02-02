/**
* Micro C++ library for colored output in terminal.
*/

#include <string>
#include <iostream>


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
			*overline = "\33[53m";
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
