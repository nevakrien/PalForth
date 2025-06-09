# Default: no arena
CFLAGS := -g2 -Wall -fsanitize=address
OBJ = header_impels.o vm.o arena.o

HEADERS=arena.h config.h cutf8.h vm.h

# Optional: Enable arena support via `make USE_ARENA=1`
ifeq ($(USE_ARENA),1)
    CFLAGS += -DUSE_ARENA
endif


.PHONY: clean test
all: test.out

check: clean all

test: test.out
	./test.out

# Link final binary from .c + .o, but also recompile if headers change
test.out: test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $(OBJ) test.c -o $@

# Compile header_impels.o with macro flags and header deps
header_impels.o: header_impels.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

# Compile header_impels.o with macro flags and header deps
vm.o: vm.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

# Compile header_impels.o with macro flags and header deps
arena.o: arena.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm $(OBJ) test.out