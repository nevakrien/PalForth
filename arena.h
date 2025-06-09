#ifndef ARENA_H
#define ARENA_H

#include <stddef.h>

typedef struct Arena{
	void* mem;
	struct Arena* next;
}Arena;

void* arena_alloc(Arena* arena,size_t size);
void arena_free(Arena* arena);

#endif // ARENA_H

