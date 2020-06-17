#pragma once
#include <vector>

using namespace std;


struct Snippet {
	unsigned int count;
	size_t size;
	size_t length;
	const char *data;

	Snippet(const char*)
};


struct SnippetString {
	vector<Snippet*> snippets;



}



