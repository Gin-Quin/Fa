
namespace Fa {
	struct Object {
		int ___count { 0 };
	};


	template<typename T>
	struct Box {
		static_assert(std::is_base_of<Object, T>::value, "<Box> The type must inherits Fa::Object");
		Object* o { NULL };
	};
}

int main() {
	return Fa::main();
}