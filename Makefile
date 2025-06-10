# Default: no arena
CFLAGS := -g2 -g -Wall -fsanitize=address
OBJ = header_impels.o vm.o arena.o

HEADERS := $(wildcard *.h)


# Optional: Enable arena support via `make USE_ARENA=1`
ifeq ($(USE_ARENA),1)
    CFLAGS += -DUSE_ARENA
endif

.PHONY: clean test check

all: test.out

check: clean all

test: test.out
	./test.out

test.out: test.c $(OBJ) $(HEADERS)
	$(CC) $(CFLAGS) $(OBJ) test.c -o $@

$(OBJ): %.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJ) test.out
