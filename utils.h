#ifndef UTILS_H
#define UTILS_H

#include <stdlib.h>
#include <stdio.h>
#include <limits.h>
#include <errno.h>

//user optional definitions
#define CHECK_UNREACHABLE //puts an assert instead of ub 

// #define MUTATEBLE 

#ifndef __cplusplus
#include <assert.h>
#else
#include <cassert>
#endif //__cplusplus

#if defined(CHECK_UNREACHABLE)

#define UNREACHABLE() assert(0 && "Unreachable code reached")

#define ASSERT(x) assert(x) 

#elif defined(__GNUC__) || defined(__clang__)
#define UNREACHABLE() __builtin_unreachable()
#elif defined(_MSC_VER)

#define UNREACHABLE() __assume(0)
#define ASSERT(x) __assume(x)

#else
//null pointer dereference to signal unreachability
#define UNREACHABLE() (*(int*)0 = 0)
#endif

#ifndef ASSERT
#include <stdbool.h>
static inline void ASSERT(bool x){
    if(!x){UNREACHABLE();}
}
// #define ASSERT(x) if(!x){UNREACHABLE();}
#endif

#define TODO assert(0 && "TODO");

static inline void* null_check(void* p){
    if(p==NULL){
        perror("went oom\n");
        exit(1);
    }
    return p;
}

#define DEFINE_VECTOR_TYPE(T, Name) \
    typedef struct { \
        T* data; \
        size_t len, capacity; \
    } Name


#define ENSURE_CAPACITY(arr) do {\
    if((arr)->len >(arr)->capacity){\
        (arr) -> capacity = 1+2*(arr) -> len;\
        (arr) ->data = null_check(realloc((arr) ->data,sizeof(*(arr) ->data )* (arr) -> capacity));\
    }\
} while(0)\

#define APPEND(arr,elem) do{\
      (arr)->len++;\
      ENSURE_CAPACITY(arr);\
      (arr)->data[(arr)->len-1] = elem;\
}while(0)\

#define SHRINK_TO_FIT(arr) do{\
    void* new_data = realloc((arr) -> data,sizeof(*(arr) ->data )* (arr)-> len);\
    (arr) -> data = new_data? new_data : (arr) -> data;\
}while(0)\

#define SHRINK(arr) do{\
    if(2*(arr)->len < (arr)->capacity)\
        SHRINK_TO_FIT(arr);\
}while(0)\

#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))



#endif// UTILS_H