#include "config.h"
#include "arena.h"
#include <stdlib.h>
#include <string.h>

void* arena_alloc(Arena* arena,size_t size){
	void* ans = malloc(size);
	if(!ans)
		return ans;

	Arena* head = malloc(sizeof(Arena));
	if(!head){
		free(ans);
		return head;
	}

	memcpy(head,arena,sizeof(Arena));
	arena->next=head;
	arena->mem=ans;

	return ans;
}

void arena_free(Arena* arena){
	for(;arena;arena=arena->next)
		free(arena->mem);
}