
/**
* Osiris is the entity that decides which souls (objects) deserve to be freed from living.
*/
namespace Osiris {
	struct SoulInfos {
		unsigned int count;
		SoulInfos() {
			count = 0;
		}
	};

	/**
	* The Soul book consign every soul informations.
	*/
	map<void*, SoulInfos> soulBook;

	void newSoul(void* soul) {
		soulBook[soul].count++;
	}

	/**
	* Free an object/soul.
	* Decrement its count in the Soul book, and if the total is 0, effectively free it.
	*/
	unsigned int freeSoul(void* soul) {
		if (!soul) {
			print("[Osiris] : Can't free the soul, it has not be defined");
			return 0;
		}
		unsigned int count = --(soulBook[soul].count);
		if (!count) {
			free(soul);
			print("Osiris : the soul", soul, "has been freed 😀");
		}
		else if (count < 0)
			print("[Osiris] : The soul is not on the book or has already been freed!");
		else
			print("Osiris : Soul", soul, "count :", count);
		return count;
	}
}

#include "Body.hpp"
