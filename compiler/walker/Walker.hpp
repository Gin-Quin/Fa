#pragma once
#include "../common.hpp"
#include "classes/NodeCallback.hpp"
#include "../parser/Parser.hpp"


/**
 * A walker is a structure used to walk an AST.
 */
struct Walker {
	Parser* parser;

	Walker(Parser& _parser) {
		parser = &_parser;
	}

	inline string value(Node* node) {
		return parser->extract(node);
	}

	void start() {
		if (!parser->tree)
			throw "No syntax tree to walk through";
		walk(parser->tree);
	}


	void walk(Node* node) {
		for (Node* child : parser->tree->children)
			visit(child);
	}

	// virtual methods
	virtual void UnknownToken(Node*) {}
	virtual void Number(Node*) {}
	virtual void Comment(Node*) {}
	virtual void SubComment(Node*) {}
	virtual void Checkpoint(Node*) {}
	virtual void Identifier(Node*) {}
	virtual void String(Node*) {}
	virtual void RawString(Node*) {}
	virtual void StringEnd(Node*) {}
	virtual void SYMBOLS(Node*) {}
	virtual void LeftParenthesis(Node*) {}
	virtual void RegexStart(Node*) {}
	virtual void GlobStart(Node*) {}
	virtual void LeftBrace(Node*) {}
	virtual void LeftBraceNoLeft(Node*) {}
	virtual void Backslash(Node*) {}
	virtual void DoubleBackslash(Node*) {}
	virtual void Equal(Node*) {}
	virtual void Colon(Node*) {}
	virtual void ColonBody(Node*) {}
	virtual void LeftCurlyBrace(Node*) {}
	virtual void Dot(Node*) {}
	virtual void Comma(Node*) {}
	virtual void Apostrophe(Node*) {}
	virtual void Quote(Node*) {}
	virtual void Accent(Node*) {}
	virtual void Asterisk(Node*) {}
	virtual void Divide(Node*) {}
	virtual void Circumflex(Node*) {}
	virtual void Plus(Node*) {}
	virtual void PlusRight(Node*) {}
	virtual void Minus(Node*) {}
	virtual void MinusRight(Node*) {}
	virtual void QuestionMark(Node*) {}
	virtual void Tilde(Node*) {}
	virtual void DoubleEqual(Node*) {}
	virtual void NotEqual(Node*) {}
	virtual void Equivalent(Node*) {}
	virtual void LesserOrEqual(Node*) {}
	virtual void GreaterOrEqual(Node*) {}
	virtual void InputArrow(Node*) {}
	virtual void OutputArrow(Node*) {}
	virtual void Percent(Node*) {}
	virtual void Extract(Node*) {}
	virtual void Insert(Node*) {}
	virtual void DoubleDot(Node*) {}
	virtual void DoubleDotBody(Node*) {}
	virtual void TripleDot(Node*) {}
	virtual void TripleDotBody(Node*) {}
	virtual void MultiLineString(Node*) {}
	virtual void PlusPlus(Node*) {}
	virtual void MinusMinus(Node*) {}
	virtual void Power(Node*) {}
	virtual void PlusEqual(Node*) {}
	virtual void MinusEqual(Node*) {}
	virtual void TimesEqual(Node*) {}
	virtual void DivideEqual(Node*) {}
	virtual void IntegerDivideEqual(Node*) {}
	virtual void LesserThan(Node*) {}
	virtual void GreaterThan(Node*) {}
	virtual void SendTo(Node*) {}
	virtual void Pipe(Node*) {}
	virtual void At(Node*) {}
	virtual void Semicolon(Node*) {}
	virtual void RightParenthesis(Node*) {}
	virtual void RegexOrGlobContent(Node*) {}
	virtual void RegexOrGlobEnd(Node*) {}
	virtual void RegexOrGlobOption(Node*) {}
	virtual void RightBrace(Node*) {}
	virtual void RightCurlyBrace(Node*) {}
	virtual void UserDefinedSymbol(Node*) {}
	virtual void KEYWORDS(Node*) {}
	virtual void Let(Node*) {}
	virtual void Super(Node*) {}
	virtual void Print(Node*) {}
	virtual void Use(Node*) {}
	virtual void Import(Node*) {}
	virtual void Export(Node*) {}
	virtual void From(Node*) {}
	virtual void Extends(Node*) {}
	virtual void IfComprehension(Node*) {}
	virtual void If(Node*) {}
	virtual void ElseComprehension(Node*) {}
	virtual void Else(Node*) {}
	virtual void ElseIf(Node*) {}
	virtual void Then(Node*) {}
	virtual void Do(Node*) {}
	virtual void WhileComprehension(Node*) {}
	virtual void While(Node*) {}
	virtual void RepeatComprehension(Node*) {}
	virtual void Repeat(Node*) {}
	virtual void ForComprehension(Node*) {}
	virtual void For(Node*) {}
	virtual void In(Node*) {}
	virtual void When(Node*) {}
	virtual void And(Node*) {}
	virtual void Or(Node*) {}
	virtual void Xor(Node*) {}
	virtual void Modulo(Node*) {}
	virtual void Is(Node*) {}
	virtual void IsStart(Node*) {}
	virtual void To(Node*) {}
	virtual void Not(Node*) {}
	virtual void Isnt(Node*) {}
	virtual void Return(Node*) {}
	virtual void Continue(Node*) {}
	virtual void Break(Node*) {}
	virtual void Try(Node*) {}
	virtual void Catch(Node*) {}
	virtual void Finally(Node*) {}
	virtual void Throw(Node*) {}
	virtual void Async(Node*) {}
	virtual void Await(Node*) {}
	virtual void Yield(Node*) {}
	virtual void Nil(Node*) {}
	virtual void True(Node*) {}
	virtual void False(Node*) {}
	virtual void Infinity(Node*) {}
	virtual void Global(Node*) {}
	virtual void Override(Node*) {}
	virtual void Final(Node*) {}
	virtual void Const(Node*) {}
	virtual void Private(Node*) {}
	virtual void Static(Node*) {}
	virtual void Class(Node*) {}
	virtual void Enum(Node*) {}
	virtual void Abstract(Node*) {}
	virtual void Interface(Node*) {}
	virtual void Structure(Node*) {}
	virtual void Unique(Node*) {}
	virtual void Exclamation(Node*) {}
	virtual void Self(Node*) {}
	virtual void END(Node*) {}

	// node visiter
	void visit(Node* node) {
		switch (node->token->type) {
			case Token::Type::UnknownToken: return UnknownToken(node);
			case Token::Type::Number: return Number(node);
			case Token::Type::Comment: return Comment(node);
			case Token::Type::SubComment: return SubComment(node);
			case Token::Type::Checkpoint: return Checkpoint(node);
			case Token::Type::Identifier: return Identifier(node);
			case Token::Type::String: return String(node);
			case Token::Type::RawString: return RawString(node);
			case Token::Type::StringEnd: return StringEnd(node);
			case Token::Type::SYMBOLS: return SYMBOLS(node);
			case Token::Type::LeftParenthesis: return LeftParenthesis(node);
			case Token::Type::RegexStart: return RegexStart(node);
			case Token::Type::GlobStart: return GlobStart(node);
			case Token::Type::LeftBrace: return LeftBrace(node);
			case Token::Type::LeftBraceNoLeft: return LeftBraceNoLeft(node);
			case Token::Type::Backslash: return Backslash(node);
			case Token::Type::DoubleBackslash: return DoubleBackslash(node);
			case Token::Type::Equal: return Equal(node);
			case Token::Type::Colon: return Colon(node);
			case Token::Type::ColonBody: return ColonBody(node);
			case Token::Type::LeftCurlyBrace: return LeftCurlyBrace(node);
			case Token::Type::Dot: return Dot(node);
			case Token::Type::Comma: return Comma(node);
			case Token::Type::Apostrophe: return Apostrophe(node);
			case Token::Type::Quote: return Quote(node);
			case Token::Type::Accent: return Accent(node);
			case Token::Type::Asterisk: return Asterisk(node);
			case Token::Type::Divide: return Divide(node);
			case Token::Type::Circumflex: return Circumflex(node);
			case Token::Type::Plus: return Plus(node);
			case Token::Type::PlusRight: return PlusRight(node);
			case Token::Type::Minus: return Minus(node);
			case Token::Type::MinusRight: return MinusRight(node);
			case Token::Type::QuestionMark: return QuestionMark(node);
			case Token::Type::Tilde: return Tilde(node);
			case Token::Type::DoubleEqual: return DoubleEqual(node);
			case Token::Type::NotEqual: return NotEqual(node);
			case Token::Type::Equivalent: return Equivalent(node);
			case Token::Type::LesserOrEqual: return LesserOrEqual(node);
			case Token::Type::GreaterOrEqual: return GreaterOrEqual(node);
			case Token::Type::InputArrow: return InputArrow(node);
			case Token::Type::OutputArrow: return OutputArrow(node);
			case Token::Type::Percent: return Percent(node);
			case Token::Type::Extract: return Extract(node);
			case Token::Type::Insert: return Insert(node);
			case Token::Type::DoubleDot: return DoubleDot(node);
			case Token::Type::DoubleDotBody: return DoubleDotBody(node);
			case Token::Type::TripleDot: return TripleDot(node);
			case Token::Type::TripleDotBody: return TripleDotBody(node);
			case Token::Type::MultiLineString: return MultiLineString(node);
			case Token::Type::PlusPlus: return PlusPlus(node);
			case Token::Type::MinusMinus: return MinusMinus(node);
			case Token::Type::Power: return Power(node);
			case Token::Type::PlusEqual: return PlusEqual(node);
			case Token::Type::MinusEqual: return MinusEqual(node);
			case Token::Type::TimesEqual: return TimesEqual(node);
			case Token::Type::DivideEqual: return DivideEqual(node);
			case Token::Type::IntegerDivideEqual: return IntegerDivideEqual(node);
			case Token::Type::LesserThan: return LesserThan(node);
			case Token::Type::GreaterThan: return GreaterThan(node);
			case Token::Type::SendTo: return SendTo(node);
			case Token::Type::Pipe: return Pipe(node);
			case Token::Type::At: return At(node);
			case Token::Type::Semicolon: return Semicolon(node);
			case Token::Type::RightParenthesis: return RightParenthesis(node);
			case Token::Type::RegexOrGlobContent: return RegexOrGlobContent(node);
			case Token::Type::RegexOrGlobEnd: return RegexOrGlobEnd(node);
			case Token::Type::RegexOrGlobOption: return RegexOrGlobOption(node);
			case Token::Type::RightBrace: return RightBrace(node);
			case Token::Type::RightCurlyBrace: return RightCurlyBrace(node);
			case Token::Type::UserDefinedSymbol: return UserDefinedSymbol(node);
			case Token::Type::KEYWORDS: return KEYWORDS(node);
			case Token::Type::Let: return Let(node);
			case Token::Type::Super: return Super(node);
			case Token::Type::Print: return Print(node);
			case Token::Type::Use: return Use(node);
			case Token::Type::Import: return Import(node);
			case Token::Type::Export: return Export(node);
			case Token::Type::From: return From(node);
			case Token::Type::Extends: return Extends(node);
			case Token::Type::IfComprehension: return IfComprehension(node);
			case Token::Type::If: return If(node);
			case Token::Type::ElseComprehension: return ElseComprehension(node);
			case Token::Type::Else: return Else(node);
			case Token::Type::ElseIf: return ElseIf(node);
			case Token::Type::Then: return Then(node);
			case Token::Type::Do: return Do(node);
			case Token::Type::WhileComprehension: return WhileComprehension(node);
			case Token::Type::While: return While(node);
			case Token::Type::RepeatComprehension: return RepeatComprehension(node);
			case Token::Type::Repeat: return Repeat(node);
			case Token::Type::ForComprehension: return ForComprehension(node);
			case Token::Type::For: return For(node);
			case Token::Type::In: return In(node);
			case Token::Type::When: return When(node);
			case Token::Type::And: return And(node);
			case Token::Type::Or: return Or(node);
			case Token::Type::Xor: return Xor(node);
			case Token::Type::Modulo: return Modulo(node);
			case Token::Type::Is: return Is(node);
			case Token::Type::IsStart: return IsStart(node);
			case Token::Type::To: return To(node);
			case Token::Type::Not: return Not(node);
			case Token::Type::Isnt: return Isnt(node);
			case Token::Type::Return: return Return(node);
			case Token::Type::Continue: return Continue(node);
			case Token::Type::Break: return Break(node);
			case Token::Type::Try: return Try(node);
			case Token::Type::Catch: return Catch(node);
			case Token::Type::Finally: return Finally(node);
			case Token::Type::Throw: return Throw(node);
			case Token::Type::Async: return Async(node);
			case Token::Type::Await: return Await(node);
			case Token::Type::Yield: return Yield(node);
			case Token::Type::Nil: return Nil(node);
			case Token::Type::True: return True(node);
			case Token::Type::False: return False(node);
			case Token::Type::Infinity: return Infinity(node);
			case Token::Type::Global: return Global(node);
			case Token::Type::Override: return Override(node);
			case Token::Type::Final: return Final(node);
			case Token::Type::Const: return Const(node);
			case Token::Type::Private: return Private(node);
			case Token::Type::Static: return Static(node);
			case Token::Type::Class: return Class(node);
			case Token::Type::Enum: return Enum(node);
			case Token::Type::Abstract: return Abstract(node);
			case Token::Type::Interface: return Interface(node);
			case Token::Type::Structure: return Structure(node);
			case Token::Type::Unique: return Unique(node);
			case Token::Type::Exclamation: return Exclamation(node);
			case Token::Type::Self: return Self(node);
			case Token::Type::END: return END(node);
		}
	}
};
