/* A small C library for the foreign-code tests (`tests/foreign.rs`). */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
	int32_t x;
	float y;
} point;

typedef struct {
	double a;
	double b;
	double c;
} triple;

typedef struct farm {
	int32_t animals;
	char name[32];
} farm;

point point_scale(point p, int32_t factor) {
	point result = {p.x * factor, p.y * (float)factor};
	return result;
}

/* a struct with an array field, by value */
typedef struct {
	uint8_t rgba[4];
	float weight;
} color;

int32_t color_sum(color c) {
	return c.rgba[0] + c.rgba[1] + c.rgba[2] + c.rgba[3] + (int32_t)c.weight;
}

color color_make(uint8_t value) {
	color c = {{value, (uint8_t)(value + 1), (uint8_t)(value + 2), (uint8_t)(value + 3)}, 2.5f};
	return c;
}

double triple_sum(triple t) {
	return t.a + t.b + t.c;
}

triple triple_make(double value) {
	triple t = {value, value * 2, value * 3};
	return t;
}

/* writes through its parameter */
void counter_increment(int64_t *counter, int64_t step) {
	*counter += step;
}

/* an opaque resource */
static int32_t open_farms = 0;

farm *farm_open(const char *name) {
	farm *opened = calloc(1, sizeof(farm));
	strncpy(opened->name, name, sizeof(opened->name) - 1);
	open_farms += 1;
	return opened;
}

void farm_add(farm *opened, int32_t count) {
	opened->animals += count;
}

int32_t farm_animals(const farm *opened) {
	return opened->animals;
}

const char *farm_name(const farm *opened) {
	return opened->name;
}

void farm_close(farm *opened) {
	open_farms -= 1;
	free(opened);
}

int32_t farm_open_count(void) {
	return open_farms;
}

/* calls back into Fa */
int64_t apply_twice(int64_t (*function)(int64_t), int64_t value) {
	return function(function(value));
}
