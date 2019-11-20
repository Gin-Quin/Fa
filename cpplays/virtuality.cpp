#include "main.hpp"



struct Base
{
	virtual void sayHello() {
		print("Hello from Base");
	}
};

struct Derived : Base
{
	void sayHello() {
		print("Hello from Derived");
	}
};


int main()
{
	Base* b = new Base();
	Derived* d = new Derived();
	Base* b2 = d;
	b->sayHello();
	d->sayHello();
	b2->sayHello();

	delete b, d;
	return 0;
}