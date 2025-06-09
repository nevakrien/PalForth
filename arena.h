#ifndef ARENA_H
#define ARENA_H

#include <stddef.h>
#include "utils.h"

//this should be rewritten to dynamic arrays later

typedef struct Arena Arena;

struct Arena{
	void** mem;
	size_t len;
	size_t capacity;
};

void* arena_alloc(Arena* arena,size_t size);
void arena_free(Arena* arena);

#endif // ARENA_H

