#include "Glue.hpp"

struct TokenInfo {
	int glue;
	int priority { 0 };  // 0 : not concerned
};

constexpr TokenInfo tokenInfos[] = {
	${tokens.map((t, i) => '/* '+ i +'. '+ t.name +' */'+ ' '.repeat((i < 10 ? 21 : i > 99 ? 19 : 20) - t.name.length) +'{ '+ (t.glue? (t.glue.map(g => 'Glue::' + g).join(' | ')) : '') + (t.priority?  ', ' + t.priority : '') +' }').join(',\n\t')}
};
