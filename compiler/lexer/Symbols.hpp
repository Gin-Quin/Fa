#pragma once
/**
Vector of the different symbols.
Can be extended by user-defined symbols.
**/

#include "Token.hpp"

struct Symbol {
	const char* value;
	Token::Type type;
};


// we create the singleton
struct {
	vector<Symbol> table {
		// { "<<<" , Token::StreamFrom },
		// { ">>>" , Token::StreamTo },
		// { "<=>" , Token::Swap },
		{ "..." , Token::TripleDot },
		{ "==" ,  Token::DoubleEqual },
		{ "!=" ,  Token::NotEqual },
		{ "~=" ,  Token::Equivalent },
		{ "<=" ,  Token::LesserOrEqual },
		{ ">=" ,  Token::GreaterOrEqual },
		{ "->" ,  Token::OutputArrow },
		{ "<-" ,  Token::InputArrow },
		{ "<<" ,  Token::Insert },
		{ ">>" ,  Token::Extract },
		{ ".." ,  Token::DoubleDot },
		{ "++" ,  Token::PlusPlus },
		{ "--" ,  Token::MinusMinus },
		{ "**" ,  Token::Power },
		{ "//" ,  Token::RegexStart },
		{ "||" ,  Token::GlobStart },
		{ "+=" ,  Token::PlusEqual },
		{ "-=" ,  Token::MinusEqual },
		{ "*=" ,  Token::TimesEqual },
		{ "/=" ,  Token::DivideEqual },
		// { ".:" ,  Token::DotColon },
		{ "|>" ,  Token::SendTo },
		{ "\\\\" ,Token::DoubleBackslash },
		{ "(" ,   Token::LeftParenthesis },
		{ "[" ,   Token::LeftBrace },
		{ "{" ,   Token::LeftCurlyBrace },
		{ ")" ,   Token::RightParenthesis },
		{ "]" ,   Token::RightBrace },
		{ "}" ,   Token::RightCurlyBrace },
		{ "\\" ,  Token::Backslash },
		{ "=" ,   Token::Equal },
		{ "+" ,   Token::Plus },
		{ "-" ,   Token::Minus },
		{ "?" ,   Token::QuestionMark },
		{ "." ,   Token::Dot },
		{ "|" ,   Token::Pipe },
		{ "," ,   Token::Comma },
		{ ":" ,   Token::Colon },
		{ "'" ,   Token::StringStart },
		{ "\"" ,  Token::StringStart },
		{ "`" ,   Token::StringStart },
		{ "#" ,   Token::Comment },
		{ "*" ,   Token::Asterisk },
		{ "/" ,   Token::Divide },
		{ "@" ,   Token::At },
		{ "!" ,   Token::Exclamation },
		{ ">" ,   Token::GreaterThan },
		{ "<" ,   Token::LesserThan },
		{ "^" ,   Token::Circumflex },
		{ "~" ,   Token::Tilde },
		{ ";" ,   Token::Semicolon },
		{ "%" ,   Token::Percent },
	};

	Token::Type find(const char* in, int& length) {
		char c;
		int i;

		for (auto& symbol : table) {
			i = 0;
			while ((c = symbol.value[i]) == in[i] && c)
				i++;
			if (c == 0 || in[i] == 0) {  // they matched
				length = strlen(symbol.value);
				return symbol.type;
			}
		}

		length = 0;
		return Token::UnknownToken;
	}

} Symbols;
