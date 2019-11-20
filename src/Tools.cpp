
#include <iostream>
#include <fstream>
#include <string>

#include "Tools.hpp"
#include "Operators.hpp"

// extern ofstream faOutput;


using namespace std;
// using namespace FaTools;









/**************************************************

   ----  FONCTIONS DE GESTION DES STRINGS  ----

***************************************************/


/**
* Sépare le code en différents éléments
*/
CodeElements splitCodeString(string str) {
	const Operator *op;
	char c;
	unsigned int wordStart = 0;
	unsigned int wordLength = 0;
	CodeElements elements;


	while ((c = str[wordStart + wordLength])) {
		
		// si c'est un caractère d'espace, on termine l'élément actuel
		if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
			if (!wordLength) {
				wordStart++;
				continue;
			}

			elements.push_back({str.substr(wordStart, wordLength), NULL});
			wordStart += wordLength;
			wordLength = 0;
		}

		// si c'est un opérateur, on termine l'élément actuel et on ajoute l'opérateur
		else if ((op = matchOperator(str.substr(wordStart + wordLength, 3)))) {
			if (wordLength) {  // on ajoute l'élément actuel
				elements.push_back({str.substr(wordStart, wordLength), NULL});
				wordStart += wordLength;
			}

			std::cout << "OP : " << op->value << " " << op->connexion << std::endl;

			// on ajoute l'opérateur
			if (op->connexion & CONTAINER) {  // cas d'un container
				string content = fragment(str, wordStart);
				elements.push_back({content, op});
				wordStart += content.length();
			}
			else {  // cas d'un opérateur classique
				elements.push_back({op->value, op});
				wordStart += op->value.length();
			}
			wordLength = 0;
		}

		else {
			wordLength++;
		}
	}


	// on ajoute le dernier élément
	if (wordLength)
		elements.push_back({str.substr(wordStart, wordLength), NULL});

	return elements;
}



/**
* Retourne le prochain élément de code dans la chaîne de caractère
* Met également à jour l'offset
*/
CodeElement nextCodeElement(string str, unsigned int &offset) {
	const Operator *op;
	char c;
	bool trimmed = false;
	unsigned int wordStart = 0;
	// CodeElement elt;

	while ((c = str[offset])) {
		if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
			offset++;

			// si on est arrivé à la fin du mot
			if (trimmed)
				return {str.substr(wordStart, offset-1-wordStart), NULL};
		}


		// si on tombe sur un élément de ponctuation, on s'arrête là
		else if (isPunctuation(c)) {
			// si c'est la fin de l'expression
			if (trimmed)
				return {str.substr(wordStart, offset-wordStart), NULL};


			// sinon, c'est le début d'un opérateur
			op = matchOperator(str.substr(offset, 3));

			// si ce n'est pas un opérateur, il y a un symbole de ponctuation qui ne devrait pas être là
			if (!op) {
				string error = "Unrocognized punctuation character : ";
				throw error + c;
			}


			// si c'est un container
			if (op->connexion & CONTAINER) {
				string content = fragment(str, wordStart);
				offset += content.length();
				return {content, op};
			}


			// sinon, c'est un opérateur basique
			offset += op->value.size();
			return {op->value, op}
		}


		// sinon, c'est le début d'une expression
		else {
			wordStart = offset++;
			if (!trimmed) trimmed = true;
		}
	}


	// s'il y avait un mot, on le retourne
	if (trimmed)
		return {str.substr(wordStart, offset-wordStart), NULL}

	// sinon, on
	return {"", NULL};
}





/**
* Indique si la chaîne de caractères (ou ses premiers éléments) matchent un opérateur
*/
const Operator *matchOperator(string str) {
	int x;
	string op;

	for (x=0; x < NUM_OPERATORS; x++) {
		op = Operators[x].value;

		if (str[0] == op[0] && 
				(!op[1] || op[1] == str[1] &&
					(!op[2] || op[2] == str[2] && 
						(!op[3] || op[3] == str[3])))
			) return &(Operators[x]);
	}

	return NULL;
}




/**
* Analyse le type de ligne (leftValue | leftValue+rightValue | mot-clé | rightValue)
*/
// string evalFaCode(string faCode, string mode) {
// 	string faLine, faWord;
// 	faCode = trim(faCode);
// 	if (!faCode.length()) return "";


// 	// on trouve le premier caractère indicateur (espace, égal, ponctuation)
// 	while (faCode[0]) {
// 		faLine = getLine(faCode);
// 		faWord = getWord(faLine);
// 		cout << faLine << endl;

// 		// -- special keywords
// 		if (faWord == "return") {
// 			print("C'est un return !");
// 		}
// 		else if (faWord == "break") {
// 			print("C'est un break !");
// 		}
// 		else if (faWord == "continue") {
// 			print("C'est un continue !");
// 		}



// 		// -- conditionals and loops
// 		else if (faWord == "if") {
// 			print("C'est un if !");
// 		}
// 		else if (faWord == "else if") {
// 			print("C'est un else if !");
// 		}
// 		else if (faWord == "else") {
// 			print("C'est un else !");
// 		}
// 		else if (faWord == "while") {
// 			print("C'est un while !");
// 		}
// 		else if (faWord == "do") {
// 			print("C'est un do !");
// 		}
// 		else if (faWord == "loop") {
// 			print("C'est un loop !");
// 		}
// 		else if (faWord == "for") {
// 			print("C'est un for !");
// 		}
// 		else if (faWord == "match") {
// 			print("C'est un match !");
// 		}
// 		else if (faWord == "switch") {
// 			print("C'est un switch ! (non supporté, mais il faudrait pour la compatibilité javascript)");
// 		}



// 		// -- decorators
// 		else if (faWord[0] == '@' ||
// 				faWord == "private" ||
// 				faWord == "sealed" ||
// 				faWord == "static"
// 		) {
// 			print("C'est un decorator ! --> " + faWord);
// 			if (faWord[0] == '@')
// 				faWord = faWord.substr(1);

// 			// faOutput << "decorator(\""+faWord+"\");" << endl;
// 			faCode = trim(faCode.substr(faWord.length()));
// 			continue;
// 		}


// 		// classes and other special objects
// 		else if (faWord == "class") {
// 			print("C'est une class !");
// 		}
// 		else if (faWord == "unique") {
// 			print("C'est un unique !");
// 		}
// 		else if (faWord == "interface") {
// 			print("C'est un interface !");
// 		}



// 		// -- variables, objects, functions, and constants
// 		// -- ou assignement
// 		// -- ou commande
// 		else {
// 			string type;

// 			if (faWord[0] == '[' ||
// 				faWord == "var" ||
// 				faWord == "Object" ||
// 				faWord == "function" ||
// 				faWord == "let" ||
// 				faWord == "int" ||
// 				faWord == "string" ||
// 				faWord == "number"
// 			) type = "initialisation";

// 			else {
// 				type = getFaLineType(faLine);
// 			}

// 			cout << ">> C'est une " << type << " !" << endl;

// 			// on obtient la forme de l'initialisation :
// 			// *type* *var1* [= *expr*] [, *var2* [= *expr2*]] ...
// 			type = faWord;
// 			string var = getWord(trim(faLine.substr(faWord.length())));

// 			// faOutput << "new(\""+type+"\", \""+ var +"\");" << endl;
// 		}


// 		// on passe à l'instruction suivante
// 		cout << endl << endl;
// 		faCode = trim(faCode.substr(faLine.length()));
// 	}


// 	return "";
// }



/**
* Retourne la première ligne de la chaîne de caractère.
*/
string getLine(string str) {
	int x = 0;
	char c;

	while ((c = str[x]) && c != '\n' && c != '\r') {
		// si on arrive sur un opener (potentiellement polyline), on passe directement à son closer
		if (c == '{' || c == '[' || c == '"' || c == '\'' || c == '`' || c == '<' || c == '(') {
			x += fragment(str, x).length();
		}

		// commentaire de type '//' -> fin de ligne
		else if (c == '/' && str[x] == '/') {
			break;
		}

		else x++;
	}
	return str.substr(0, x);
}




/**
* Retourne le premier mot de la chaîne de caractère.
*/
string getWord(string str) {
	int x = 0;
	char c;

	while ((c = str[x]) && c != '\n' && c != '\r' && c != ' ' && c != '\t') {
		// si on arrive sur un opener (potentiellement polyline), on passe directement à son closer
		if (c == '{' || c == '[' || c == '"' || c == '\'' || c == '`' || c == '<' || c == '(') {
			x += fragment(str, x).length();
		}

		// commentaire de type '//' -> fin de ligne
		else if (c == '/' && str[x] == '/') {
			break;
		}

		else x++;
	}

	return str.substr(0, x);
}



/**
* Vérifie si la string *s* commence par *starter*
*/
bool startsWith(string s, string starter) {
	return s.substr(0, starter.length()) == starter;
}




/**
* Vérifie si la string *s* commence par le mot *word*
*/
bool startsWithWord(string s, string starter) {
	int l = starter.length();
	return  startsWith(s, starter) && (!s[l] || s[l] == ' ' || s[l] == '\t');
}




/**
* Vérifie si le code donné dans la chaîne de caractère *s* est un début d'initialisation.
* Pour vérifier cela : il faut commencer par deux variables séparées par un espace.
*/
// string getFaLineType(string s) {
// 	if (isPunctuation(s[0]))
// 		return "command";

// 	int x = 0;
// 	char c;


// 	while ((c = s[x])) {

// 		if (c == ' ' || c == '\t') {
// 			// on a un un nom de variable suivi d'un espace
// 			// si le caractère suivant est un '=' simple (non suivi d'un autre égal), alors on a une assignation (ex: x = 125)
// 			// si le caractère suivant est une ponctuation, alors on a une commande (ex: zabu.call(this))
// 			// s'il n'y a pas de caractère suivant, c'est une commande (Peut être un getter. Ex: zabu.coco[3].call)
// 			// sinon, on a une initialisation (ex : int x = 125)

// 			string nextWord = trim(s.substr(x));
// 			c = nextWord[0];


// 			if (!c)
// 				return "command";
// 			else if (c == '=') {
// 				if (nextWord[1] != '=')  // si ce n'est pas un '=='
// 					return "assignation";
// 				return "command";
// 			}
// 			else if (isPunctuation(c))
// 				return "command";
// 			else return "initialisation";
// 		}

// 		if (c == '.') {  // cas spécial du point : autorisé
// 			x++;
// 		}
		
// 		else if (c == '[') {  // cas spécial du bracket : autorisé
// 			// ATTENTION : les symboles autres qu'opératoires (en particulier les virgules) ne sont pas autorisés
// 			x += fragment(s.substr(x), "hard").length() - 1;
// 		}

// 		else if (isPunctuation(c)) {
// 			cout << "ponctuation!!!" << endl;
// 			return "command";
// 		}

// 		else x++;
// 	}

// 	// pas d'espaces ni de ponctuation : c'est une commande (getter simple)
// 	return "command";
// }



/**
* Vérifie si le caractère donné est un caractère de ponctuation.
* Une variable peut être constituée de tous les caractères sauf les caractères de ponctuation (qui sont utilisés pour le calcul, etc...).
*/
bool isPunctuation(char c) {
	// static const char punctuations[] = "&~#\"'`{}()|<>\\^+-*/=%@:;,!?\n\r\t 0123456789.[]";
	static const char punctuations[] = "&~#\"'`{}()|<>\\^+-*/=%@:;,!?\n\r\t .[]";
	int x = -1;
	while (punctuations[++x]) {
		if (punctuations[x] == c) return true;  // c est une ponctuation
	}
	return false;
}




/**
* Ôte les caractères ' ', '\n', '\r' et '\t' en début de chaîne.
*/
string trim(string str) {
	int x = 0;
	while (str[x] == ' ' || str[x] == '\t' || str[x] == '\n' || str[x] == '\r')
		x++;
	return str.substr(x);
}





/**
* Récupère le fragment de la chaîne de caractères donnée.
* Les valeurs de départ possibles sont : [ ( ' " ` < {
* - Dans le cas non-chaîne de caractères, les chaînes de caractères à l'intérieur sont correctement ignorées dans le cas où elles possèdent le symbole de fermeture.
* - Dans le cas d'une entrée xml, l'élément fermant est la balise fermante.
*   (exemples de xml : <>Hello !!</> ou <Nav>Zabu</> ou <Header attr="7" />)
* À faire : gestion des erreurs ! La seule erreur qui peut arriver consiste en un opener sans closer [du même niveau]. Ou alors : type de fragment non reconnu [mais ne devrait pas arriver].
*/
string fragment(string str, int offset=0) {
	if (!str.length()) return "";

	int x = offset;
	char opener = str[x];
	char closer = 0;
	char c;
	string type;


	if (opener == '"' || opener == '\'' || opener == '`') {
		closer = opener;
		type = "string";
	}
	else if (opener == '{') {
		closer = '}';
		type = "expression";
	}
	else if (opener == '(') {
		closer = ')';
		type = "expression";
	}
	else if (opener == '[') {
		closer = ']';
		type = "expression";
	}
	else if (opener == '<') {
		closer = '/';
		type = "xml";
	}
	else if (opener == '/' && str[1] == '/') {
		type = "line-comment";
	}
	else if (opener == '/' && str[1] == '*') {
		type = "multi-line-comment";
	}


	// on cherche le closer en prêtant attention aux symboles escaped
	if (type == "string") {
		bool escaped = false;

		while ((c = str[++x]) && (c != closer || escaped)) {
			if (escaped)
				escaped = false;
			else if (c == '\\')
				escaped = true;
		}

		if (c) {
			return str.substr(offset, x+1);
		}
		return str.substr(offset);  // pas trouvé de closer ! La fin de chaîne est considéré comme un closer
	}

	// on cherche la fin de l'expression
	// (on utilise un système de 'profondeur' pour gérer les sous-expressions)
	else if (type == "expression") {
		int depth = 0;

		while ((c = str[++x])) {

			// si c'est un closer, on baisse d'un niveau ou on termine
			if (c == closer) {
				if (depth)
					depth--;
				else break;
			}

			// si c'est un opener, on monte d'un niveau
			else if (c == opener) {
				depth++;
			}

			// si c'est un autre élément, on effectue une étude récursive
			else if (c == '\'' || c == '"' || c == '`') {
				x += fragment(str, x).length() - 1;
			}
		}

		if (c) return str.substr(offset, x+1);
		return str.substr(offset);  // pas trouvé de closer ! La fin de chaîne est considérée comme un closer
	}


	// XML (cas spécial : départ textuel. Ex : < Yoyoyo, salut <b>toi !</b> >)
	else if (type == "xml" && (str[1] == ' ' || str[1] == '\t' || str[1] == '\n' || str[1] == '\r')) {
		int depth = 1;
		closer = '>';

		while ((c = str[++x])) {

			// si c'est un closer, on baisse d'un niveau ou on termine
			if (c == closer) {
				if (!(--depth)) break;
			}

			// si c'est un opener, on monte d'un niveau
			else if (c == opener) {
				depth++;
			}

			// si c'est une chaîne de caractères, on effectue une étude récursive
			else if (c == '\'' || c == '"' || c == '`') {
				x += fragment(str, x).length() - 1;
			}
		}

		if (c) return str.substr(offset, x+1);
		return str.substr(offset);  // pas trouvé de closer ! La fin de chaîne est considérée comme un closer
	}


	// XML (classique)
	else if (type == "xml") {
		int depth = 1;

		while ((c = str[++x])) {

			// si c'est un closer, on baisse d'un niveau ou on termine
			if (c == closer) {
				depth--;
				if (str[x-1] == '<')
					depth--;

				if (!depth) break;
			}

			// si c'est un opener, on monte d'un niveau
			else if (c == opener) {
				depth++;
			}

			// si c'est une chaîne de caractères, on effectue une étude récursive
			else if (c == '\'' || c == '"' || c == '`') {
				x += fragment(str, x).length() - 1;
			}
		}

		// on continue jusqu'au prochain '>'
		while ((c = str[x++]) && c != '>');

		if (c) return str.substr(offset, x+1);
		return str.substr(offset);  // pas trouvé de closer ! La fin de chaîne est considérée comme un closer
	}


	// commentaire (ligne)
	else if (type == "line-comment") {

		while (str[++x]) {
			if (str[x] == '\n' || str[x] == '\r')
				break;
		}

		if (str[x])
			return str.substr(offset, x+1);
		return str.substr(offset);
	}


	// commentaire (multi-ligne)
	else if (type == "multi-line-comment") {

		while (str[++x]) {
			if (str[x-1] == '*' && str[x] == '/')
				break;
		}

		if (str[x])
			return str.substr(offset, x+1);
		return str.substr(offset);
	}


	// sinon, ce n'est aucun type connu
	cout << "fragment() : type de fragment non reconnu !" << endl << "\t" << str << endl;
	return "";
}

