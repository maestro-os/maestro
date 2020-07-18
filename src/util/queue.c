#include <util/util.h>

/*
 * A queue is a data structure which works in a FIFO fashion
 * (First In, First Out), which means that elements are removed in the same
 * order as they are inserted.
 *
 * This structure works with an input and an output. The input holds the last
 * inserted element and the output holds the next element to be removed.
 * Each element points to the next newer one.
 */

/*
 * Returns the number of elements from the given `queue` input.
 * Complexity: O(n)
 */
size_t queue_size(const queue_head_t *out)
{
	size_t n = 0;

	debug_assert(sanity_check(out), "queue: invalid argument");
	while(out)
	{
		++n;
		out = (const queue_head_t *) out->next;
	}
	return n;
}

/*
 * Enqueues element `n` in queue represented by `in` and `out`.
 * Complexity: O(1)
 */
void queue_enqueue(queue_head_t **in, queue_head_t **out, queue_head_t *n)
{
	debug_assert(sanity_check(in) && sanity_check(out),
		"queue: invalid arguments");
	if(!sanity_check(n))
		return;
	if(!(n->next = *in))
	{
		debug_assert(!sanity_check(*out), "queue: inconsistent state");
		*out = n;
	}
	*in = n;
}

/*
 * Dequeues the next element in queue represented by `in` and `out` and returns
 * it. Returns NULL if the queue is empty.
 * Complexity: O(1)
 */
queue_head_t *queue_dequeue(queue_head_t **in, queue_head_t **out)
{
	queue_head_t *n;

	debug_assert(sanity_check(in) && sanity_check(out),
		"queue: invalid arguments");
	n = *out;
	if(!(*out = (*out)->next))
	{
		debug_assert(!sanity_check(*in), "queue: inconsistent state");
		*in = NULL;
	}
	return n;
}
