# Default: no arena
SHARED_FLAGS := -g2 -g -Wall #-fsanitize=address -fsanitize=undefined
CFLAGS := -std=c99 $(SHARED_FLAGS)
OBJ = header_impels.o vm.o arena.o
CC = clang
CLANG = clang
CLANGXX = clang++

HEADERS := $(wildcard *.h)


# Optional: Enable arena support via `make USE_ARENA=1`
ifeq ($(USE_ARENA),1)
    CFLAGS += -DUSE_ARENA
endif

.PHONY: clean test check

all: test.out

check: clean all

test: test.out jiting_test.out jit_test.out
	./test.out
	./jiting_test.out
	./jit_test.out

test.out: test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $(OBJ) test.c -o $@

jiting_test.out: jiting_test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $(OBJ) jiting_test.c -o $@

$(OBJ): %.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

vm.ll: vm.c $(HEADERS)
	$(CLANG) -O3 -std=c99 -Wall -emit-llvm -S  vm.c -ovm.ll

jit_test.out: vm.ll jit_test.cpp $(OBJ)
	$(CLANGXX) -std=c++17 $(SHARED_FLAGS) jit_test.cpp -o jit_test.out $(OBJ) `llvm-config --cxxflags --ldflags --system-libs --libs orcjit native`


clean:
	rm -f $(OBJ) test.out vm.ll
