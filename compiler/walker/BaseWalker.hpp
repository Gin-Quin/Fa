#pragma once
#include "../common.hpp"
#include "utils/NodeCallback.hpp"
#include "../parser/Parser.hpp"

namespace Walker {
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
		virtual void visitEND(Node* node) {std::cout << "Visit END from BaseWalker" << std::endl;}

		// node visiter
		virtual void visit(Node* node) {
			switch (node->token->type) {
				case Token::Type::UnknownToken: return visitUnknownToken(node);
				case Token::Type::Number: return visitNumber(node);
				case Token::Type::Comment: return visitComment(node);
				case Token::Type::SubComment: return visitSubComment(node);
				case Token::Type::Checkpoint: return visitCheckpoint(node);
				case Token::Type::Identifier: return visitIdentifier(node);
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
				case Token::Type::END: return visitEND(node);
			}
		}
	};
}
