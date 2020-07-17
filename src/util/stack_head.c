#include <util/util.h>

/*
 * A stack is a data structure which allows to access only the last inserted
 * element. Thus the only allowed operations are pushing and popping.
 */

/*
 * Returns the number of elements on the given `stack`.
 * Complexity: O(n)
 */
size_t stack_size(const stack_head_t *stack)
{
	size_t n = 0;

	debug_assert(sanity_check(stack), "stack: invalid argument");
	while(stack)
	{
		++n;
		stack = stack->next;
	}
	return n;
}

/*
 * Pushes an element `n` on top of the stack `stack`.
 * Complexity: O(1)
 */
void stack_push(stack_head_t **stack, stack_head_t *n)
{
	debug_assert(sanity_check(stack), "stack: invalid argument");
	if(unlikely(!n))
		return;
	n->next = *stack;
	*stack = n;
}

/*
 * Pops an element from the top of the stack `stack` and returns it. Returns
 * NULL if the stack is empty.
 * Complexity: O(1)
 */
stack_head_t *stack_pop(stack_head_t **stack)
{
	stack_head_t *s;

	debug_assert(sanity_check(stack), "stack: invalid argument");
	if(likely(s = *stack))
		*stack = s->next;
	return s;
}
