#include "vm.h"
#include "cutf8.h"

int main(){
	(void)cutf8_get;
    (void)cutf8_put;
    (void)cutf8_copy;
    (void)cutf8_skip;
    (void)cutf8_valid_buff;

    test_vm();
	printf("!!!All Tests Passed!!!\n");
	return 0;
}