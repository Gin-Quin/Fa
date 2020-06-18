#pragma once

#include "../../Walker.hpp"
// #include "emitters.hpp"



namespace Generate {
	struct Js : Walker {
		bool pretty { true };
		string emitted;

		// constructor
		Js(Parser& parser) : Walker(parser) {}

		// -- EMITTERS
		void Integer(Node* node) {
			emitted = value(node);
		}
	};
}

// Generate::Js.callback[Token::Type::Unknown] = JsEmit::Unknown;