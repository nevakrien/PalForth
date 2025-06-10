#ifndef ERRORS_H
#define ERRORS_H

#include "config.h"
#include "ctypes.h"


typedef enum error {
	STACK_UNDERFLOW=2,
	STACK_OVERFLOW
} Error;

void panic(VM* vm,long code);
#define PANIC(code) panic(vm,code)


#endif // ERRORS_H

