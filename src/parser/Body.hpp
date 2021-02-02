

struct Statement;
// using Body = vector<Statement*>;

struct Body : vector<Statement*> {
	string toJson() const;
	string toVerboseJson() const;
};