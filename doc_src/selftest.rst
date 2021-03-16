Selftest
********

The kernel has the ability to run unit tests on its components when compiled in debug mode.
This feature uses the Rust language's tests feature.

Each test has to be placed in a private module with the following attribute:

.. code:: rust

    #[cfg(test)]

Each test is represented by a function in this module and must have the following attribute on it:

.. code:: rust

    #[test_case]

When compiled with environement variables ``KERNEL_MODE=debug`` and ``KERNEL_TEST=true``, the kernel will run all the tests automaticaly.
**Important note**: The tests are not isolated from each others. The kernel cannot reset the environement between each tests.
