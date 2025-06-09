# Default: no arena
CFLAGS := -O2 -Wall
OBJ = header_impels.o

HEADERS=arena.h config.h

# Optional: Enable arena support via `make USE_ARENA=1`
ifeq ($(USE_ARENA),1)
    CFLAGS += -DUSE_ARENA
endif



all: test.out

# Link final binary from .c + .o, but also recompile if headers change
test.out: test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $(OBJ) test.c -o $@

# Compile header_impels.o with macro flags and header deps
header_impels.o: header_impels.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm $(OBJ) test.out