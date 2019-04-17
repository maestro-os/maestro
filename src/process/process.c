#include "process.h"
#include "../util/btree.h"

static btree_t *processes;

void process_init()
{
	processes = NULL;
}

pid_t kfork()
{
	// TODO
	return 0;
}

process_t *get_process(const pid_t pid)
{
	if(pid == 0) return NULL;
	// TODO

	return NULL;
}
