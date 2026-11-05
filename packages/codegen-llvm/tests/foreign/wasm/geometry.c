/* C compiled to WebAssembly for the foreign-code tests (`tests/foreign.rs`):
   structs by value, linked with the program (no C library). */
typedef struct {
	int x;
	float y;
} point;

typedef struct {
	double a;
	double b;
	double c;
} triple;

typedef struct {
	unsigned char rgba[4];
	float weight;
} color;

point point_scale(point p, int factor) {
	point result = {p.x * factor, p.y * (float)factor};
	return result;
}

double triple_sum(triple t) {
	return t.a + t.b + t.c;
}

triple triple_make(double value) {
	triple t = {value, value * 2, value * 3};
	return t;
}

int color_sum(color c) {
	return c.rgba[0] + c.rgba[1] + c.rgba[2] + c.rgba[3] + (int)c.weight;
}
