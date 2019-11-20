
/*** COMPILATION ***

- Avec fenêtre :
g++ src/*.cpp -I src/include -L. -l:sfml-graphics-2.dll -l:sfml-window-2.dll -l:sfml-system-2.dll -o fa.exe

- Sans fenêtre :
g++ src/*.cpp -I src/include -o fa.exe

*/

#include <iostream>
#include <string>
#include <map>

using namespace std;

#include "var.h"

Null const null;


int main() {

	var x(12), y = 9;
	cout << x.getString() << endl;
	cout << y.getString() << endl;
	x = y = 11;
	cout << x.getString() << endl;
	cout << y.getString() << endl;
	// cout << x.type << endl;
	// cout << x.value.text << endl;

	return 0;
}







/** SFML starter !
#include <SFML/Graphics.hpp>

int main()
{
	sf::RenderWindow window(sf::VideoMode(200, 200), "SFML works!");
	sf::CircleShape shape(100.f);
	shape.setFillColor(sf::Color::Green);

	while (window.isOpen())
	{
		sf::Event event;
		while (window.pollEvent(event))
		{
			if (event.type == sf::Event::Closed)
				window.close();
		}

		window.clear();
		window.draw(shape);
		window.display();
	}

	return 0;
}
*/
