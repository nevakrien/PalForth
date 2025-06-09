# Default: no arena
CFLAGS := -O2 -Wall
OBJ = header_impels.o

HEADERS=arena.h config.h

# Optional: Enable arena support via `make USE_ARENA=1`
ifeq ($(USE_ARENA),1)
    CFLAGS += -DUSE_ARENA
endif



all: test.out

test.out: test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $^ -o $@

header_impels.o: header_impels.c
	$(CC) $(CFLAGS) $^ -c -o $@

clean:
	rm $(OBJ) test.out