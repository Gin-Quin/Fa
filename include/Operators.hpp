#pragma once

#include <vector>
#include <string>

enum Connexion {
	LEFT = 1,
	RIGHT = 2,
	LEFT_AND_RIGHT = 3,
	LEFT_OR_RIGHT = 7,
	CONTAINER = 8
};


struct Operator {
	std::string value;
	int priority;
	short connexion;
};



#define NUM_OPERATORS 34

const Operator Operators[] = {
	
	{"++",	100,	LEFT},
	{"+=",	-1,		LEFT + RIGHT},
	{"+",	10,		LEFT + RIGHT},
	{"--",	100,	LEFT_OR_RIGHT},
	{"-",	10,		LEFT + RIGHT},
	{"*=",	-1,		LEFT + RIGHT},
	{"**=",	-1,		LEFT + RIGHT},
	{"**",	30,		LEFT + RIGHT},  // puissance
	{"*",	20,		LEFT + RIGHT},
	{"//=",	-1,		LEFT + RIGHT},
	{"//",	20,		LEFT + RIGHT},  // division entière
	{"/=",	-1,		LEFT + RIGHT},
	{"/",	20,		LEFT + RIGHT},
	{"%=",	-1,		LEFT + RIGHT},
	{"%",	20,		LEFT + RIGHT},
	

	{"==",	5,		LEFT + RIGHT},
	{"!=",	5,		LEFT + RIGHT},
	{"<",	5,		LEFT + RIGHT},
	{"<=",	5,		LEFT + RIGHT},
	{">",	5,		LEFT + RIGHT},
	{">=",	5,		LEFT + RIGHT},
	

	{"=",	-1,		LEFT + RIGHT},
	

	{"(",	1000,	CONTAINER + LEFT},  // appel de fonction
	{"{",	999,	CONTAINER},
	{"[",	999,	CONTAINER + LEFT},
	{"'",	999,	CONTAINER},
	{"\"",	999,	CONTAINER},
	{"`",	999,	CONTAINER}
};
