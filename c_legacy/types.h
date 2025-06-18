#ifndef TYPES_H
#define TYPES_H

#include "config.h"

#define READ_FLAG 0x1
#define WRITE_FLAG 0x2
#define UNIQUE_FLAG 0x4
#define OUTPUT_FLAG 0x8

typedef char rw_t;

typedef struct{
	void* type;
	int32_t offset_from_start;//first local is +1
	int32_t num_borrowed; //unique borrow is -1
	rw_t permissions;
}BoxVar;

int use_box_as(BoxVar* box,rw_t sig);
void free_box_use(BoxVar* box,rw_t sig);

// #define BORROWED(x) (x!=0)
// #define BORROWED_UNIQUE(x) (x==-1)

#endif // TYPES_H

