#ifndef CONFIG_H
#define CONFIG_H

#define DEBUG_MODE
// #define UNCHECKED_STACK_OVERFLOW

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
/**
 * this defines
 * ALOT + TO_ALOT
 */

#ifdef USE_ARENA
#include "arena.h"
#define ALOT(vm,len) arena_alloc(&(vm)->dict_mem,len)
#endif

#ifndef ALOT
#define ALOT(vm,len) malloc(len)
#define TO_ALOT(vm,mem,len) realloc(mem,len)
#endif

#ifndef TO_ALOT
#define TO_ALOT(vm,mem,len) to_alot(vm,mem,len)

#ifdef CONFIG_IMPLEMENTATION
#include "utils.h"
#include <string.h>
#include "vm.h"

void* palforth_to_alot (VM* vm,void* data,size_t len){
	void* ans = ALOT(vm,len);
	if(!ans)
		return ans;

	ASSERT(data);
	memcpy(ans,data,len);
	free(data);

	return ans;
}
#endif //CONFIG_IMPLEMENTATION
#endif //TO_ALOT

#endif // CONFIG_H

