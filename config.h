#ifndef CONFIG_H
#define CONFIG_H

#define DEBUG_MODE
#define TEST
// #define UNCHECKED_STACK_OVERFLOW
#define VM_DEBUG

#ifdef TEST
#define DEBUG_MODE
#endif

#ifdef VM_DEBUG
#define VM_LOG(s) puts(s);
#else
#define VM_LOG(s)
#endif

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include "stdbool.h"

typedef int64_t palint_t;
typedef intptr_t palbool_t;//must fit inside of a pointer

//this macro is used for allocating room on the stack for a var
#define CELLS(x) ((sizeof(x) + sizeof(void*) - 1) / sizeof(void*))


#if __STDC_VERSION__ >= 201112L
    #include <stdalign.h>
    #define ALIGN(type) alignof(type)
#else
    #define ALIGN(type) offsetof(struct { char c; type d; }, d)
#endif


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

