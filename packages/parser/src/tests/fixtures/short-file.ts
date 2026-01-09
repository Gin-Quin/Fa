type User = {
	name: string;
	age: number;
};

const makeUser = (name: string, age: number): User => ({
	name,
	age,
});

const label = (user: User): string => {
	if (user.age > 17) {
		return "adult";
	}
	return "minor";
};

const alice = makeUser("Alice", 30);
const title = label(alice);

let counter = 0;
counter += 1;
