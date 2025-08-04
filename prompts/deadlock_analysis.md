# Deadlock Analysis Prompt

## Objective
Analyze this code for potential deadlocks and circular dependencies.

## Analysis Steps

1. **Identify Lock Acquisition Points**
   - Locate all mutex locks, semaphores, and other synchronization primitives
   - Map the order of lock acquisition across different code paths

2. **Check for Circular Dependencies**
   - Look for scenarios where thread A waits for resource held by thread B, while thread B waits for resource held by thread A
   - Identify any chains of dependencies that could form a cycle

3. **Analyze Lock Ordering**
   - Verify that locks are always acquired in a consistent global order
   - Flag any code paths that acquire locks in different orders

4. **Resource Hierarchy Review**
   - Check if there's a clear hierarchy of resources
   - Ensure higher-level resources are acquired before lower-level ones

## Common Deadlock Patterns to Look For

- **Lock Ordering Deadlock**: Different threads acquire the same locks in different orders
- **Dynamic Lock Ordering Deadlock**: Lock order depends on runtime parameters
- **Resource Allocation Deadlock**: Multiple threads compete for multiple resources
- **Nested Lock Deadlock**: Acquiring locks within critical sections

## Resolution Strategies

1. **Lock Ordering**: Establish a global lock ordering protocol
2. **Timeout Mechanisms**: Use try-lock with timeouts
3. **Lock-Free Algorithms**: Replace locks with atomic operations where possible
4. **Resource Allocation Graphs**: Implement deadlock detection algorithms

## Code Review Checklist

- [ ] Are all locks acquired in a consistent order?
- [ ] Are there any nested lock acquisitions?
- [ ] Do error paths properly release all acquired locks?
- [ ] Are there any long-held locks that could cause contention?
- [ ] Could any conditional logic lead to different lock orderings?

Please analyze the provided code following these guidelines and provide specific recommendations for preventing deadlocks.