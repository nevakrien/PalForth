#include "config.h"
#include "arena.h"
#include <stdlib.h>
#include <string.h>

void* arena_alloc(Arena* arena,size_t size){
	void* ans = malloc(size);
	if(!ans)
		return NULL;

	if(arena->len==arena->capacity){
		size_t new_cap=16+2*arena->len;
		void** new_mem = realloc(arena->mem,new_cap);
		if(!new_mem){
			free(ans);
			return NULL;
		}

		arena->capacity=new_cap;
	}

	arena->mem[arena->len++]=ans;
	return ans;
}

void arena_free(Arena* arena){
	for(size_t i=0;i<arena->len;i++){
		free(arena->mem[i]);
	}
	free(arena->mem);
	arena->mem=NULL;
}