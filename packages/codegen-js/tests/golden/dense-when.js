// helpers: 

function dense(value) {
	switch (value) {
		case -8: {
			return 1;
		}
		case -7: {
			return 2;
		}
		case -6: {
			return 3;
		}
		case -5: {
			return 4;
		}
		case -4: {
			return 5;
		}
		case -3: {
			return 6;
		}
		case -2: {
			return 7;
		}
		case -1: {
			return 8;
		}
		case 0: {
			return 9;
		}
		case 1: {
			return 10;
		}
		case 2: {
			return 11;
		}
		case 3: {
			return 12;
		}
		case 4: {
			return 13;
		}
		case 5: {
			return 14;
		}
		case 6: {
			return 15;
		}
		default: {
			return 16;
		}
	}
}

function sparse(value) {
	if (value === 0) {
		return 1;
	} else if (value === 100) {
		return 2;
	} else if (value === 200) {
		return 3;
	} else if (value === 300) {
		return 4;
	} else if (value === 400) {
		return 5;
	} else if (value === 500) {
		return 6;
	} else if (value === 600) {
		return 7;
	} else if (value === 700) {
		return 8;
	} else if (value === 800) {
		return 9;
	} else if (value === 900) {
		return 10;
	} else if (value === 1000) {
		return 11;
	} else if (value === 1100) {
		return 12;
	} else if (value === 1200) {
		return 13;
	} else if (value === 1300) {
		return 14;
	} else if (value === 1400) {
		return 15;
	} else {
		return 16;
	}
}

