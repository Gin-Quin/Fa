#pragma once

#include <string>
#include <vector>


class FaEvaluator {
private:
	std::string code;
	std::string mode;  // "compile" | "execute"
	std::vector<std::string> dependencies;  // liste des modules

public:
	// créé un nouvel évaluateur
	FaEvaluator();
	
	// exécute/compile du code fa
	std::string run(std::string code);
	std::string compile(std::string code);
	std::string compileAndRun(std::string code);

	// exécute/compile du code fa depuis un fichier
	std::string runFile(std::string fileName);
	std::string compileFile(std::string fileName);
	std::string compileFileAndRun(std::string fileName);

	// exécute/compile un projet fa
	std::string runProject(std::string projectName="");
	std::string compileProject(std::string projectName="");
	std::string compileProjectAndRun(std::string projectName="");


// private:


};