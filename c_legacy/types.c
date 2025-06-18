#include "types.h"
#include "errors.h"

static bool is_a_subset(rw_t box,rw_t sig){
	if(sig&UNIQUE_FLAG && !(box&UNIQUE_FLAG))
		return false;
	if(sig&WRITE_FLAG && !(box&WRITE_FLAG))
		return false;
	if(sig&READ_FLAG && !(box&READ_FLAG))
		return false;
	if((sig&OUTPUT_FLAG) != (box&OUTPUT_FLAG))
		return false;

	return true;
}

int use_box_as(BoxVar* box,rw_t sig){
	if(!is_a_subset(box->permissions,sig))
		return BAD_SIG;

	if(box->num_borrowed==-1){
		return ALREADY_BORROWED;
	}
	if(sig&UNIQUE_FLAG && box->num_borrowed!=0){
		return ALREADY_BORROWED;
	}

	if(sig&UNIQUE_FLAG){
		box->num_borrowed=-1;
	}else{
		box->num_borrowed++;
	}

	return 0;
}

void free_box_use(BoxVar* box,rw_t sig){
	if(sig&UNIQUE_FLAG){
		box->num_borrowed=0;
	}else{
		box->num_borrowed--;
	}
}