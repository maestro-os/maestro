# Intrusive collections

Linked lists are used quite a lot in kernel development.

In Rust, the typical memory layout of a linked list node looks like this underneath:

```rust
struct Node<T> {
    prev: Option<NonNull<Self>>,
    next: Option<NonNull<Self>>,
    val: T,
}
```

This format allows convenient operations because the linked list **owns** the elements.

However, there is a catch: inserting an element will require allocating memory for `Node`.
Moreover, if `T` is an `Arc` (or another container requiring a memory allocation), we now need two memory allocations to store a simple value.

On top of that, kernel development often requires being able to insert an element in a linked list without making a memory allocation.

A simple example is the `setpgid` instruction, which inserts the process in the new group leader's list.
We don't want a memory allocation to fail at that moment, since `setpgid` returning a `ENOMEM` would be weird.

To solve this issue, one should use **intrusive linked lists**. In Maestro, this is the `List` and `ListNode` structures.

An intrusive linked list is rather easy to implement in C, but trickier in Rust.
The memory layout would look like:

```rust
struct Node {
    prev: Option<NonNull<Self>>,
    next: Option<NonNull<Self>>,
}

// Here, Foo is the element inserted in the list (it would be `T` in non-intrusive linked lists)
struct Foo {
    bar: u32,
    node: Node,
}
```

The fundamental difference is that the node is contained inside the value, instead of the opposite.

This layout also allows inserting the same element in several linked lists, without requiring more memory allocations:

```rust
struct Foo {
    bar: u32,
    node1: Node,
    node2: Node,
}
```

In Maestro, all the elements in an intrusive linked list is wrapped in an `Arc`, which brings several advantages:
- The value is immutable, preventing unsafe modifications of the inner `Node` (since `Node` is supposed to be a blackbox managed by `List`'s helpers)
- The memory allocation is managed by the `Arc`

An element that is inserted in a `List` has its reference counter incremented by one, to symbolize the ownership of the list over the element.

When going through the linked list, we may go to the next element by the `prev` or `next` pointers in `Node`.
However, they only give access to the `Node` that is contained in the value, not the value itself.

The pointer to the value can be retrieved simply by subtracting the offset of the `node` field in the value (which is known at compile-time) from the `Node`'s pointer.

## Unlinking is unsafe

Most operations on an intrusive linked list can be made safe (in the Rust sense), except unlinking: if a `ListNode` has been inserted in a `List`, it needs to be removed from that list.

The list points to its first element, which has no way of knowing its list (or it would require maintaining a field specifically for this, which would consume more memory and CPU time).

As such, we need to pass the list to the unlinking function in order to clear this pointer if the element being removed is the first in the list.
But there is no way to guarantee at compile time that the caller will pass the list in which the function is actually inserted and not another list.

Passing the wrong list results in the correct list pointing to an element that isn't in the list anymore, which is both incorrect and unsound (in the Rust sense).