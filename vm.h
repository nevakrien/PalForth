#ifndef VM_H
#define VM_H

#include "config.h"

typedef struct vm {

#ifdef USE_ARENA
	Arena dict_mem;
#endif

} VM;

#endif // VM_H

