# Payments Engine

## LLM disclosure:

All of the code and this readme has been written by me.

## Architecture & Design Decisions

This has been optimized for throughput, not for latency. This is because this is a batch job.

In order to make this into a server that reads data from TCP connections, 
the change would be super simple, since this already uses a thread, adding more threads would be trivial.

### Error handling 

- Fails fast on bad input 
- Continues processing on "partner" mistakes.

### Performance

I've benchmarked, profiled and optimized this (in the limited time that I had), 
so I expect this to be faster than the average submission.
The flamegraph doesn't show any low hanging fruits, and most of the time is spent on CSV parsing, so this is as good as it gets.

If higher perf was required:
1. Look into other, more efficient formats than CSV
2. If 1. is not possible, look into faster deserialization techniques, parallel loading/parsing, etc.

I know that the instructions said that maintainability > performance, 
but I think that it's readable and maintainable enough, and since perf 
is one of the things that I enjoy the most I've taken the freedom to prioritize perf a bit more.

This approach separates CSV parsing from transaction processing using a background thread.
This is because the parsing is the slowest part, which is sequential, so this frees up valuable time from the parser thread.
I played around with more threads than 1 but since the bottleneck is parsing they were not necessary so I removed them.

Some other minor techniques:

- I've done my best to make 
- Groups transactions into batches to reduce the overhead of atomics in the channel.
- Streams data rather than loading the entire dataset into memory.
- This approach minimizes useless memory clones by moving just the heap ptrs when data volume is high.

I haven't bothered with other techniques like memory layout optimization because again, that wouldn't solve the bottleneck.

### Type safety

- Uses strong typing with custom types for Money, ClientId, and TxId.
- I specifically designed the data types so that they're impossible to misuse (see Raw* tys)

## Assumptions

I've assumed that only withdrawals can get disputed.
