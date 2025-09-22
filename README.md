# Payments Engine

## LLM disclosure:

All of the code and this readme has been written by me.

## Architecture & Design Decisions

This has been optimized for throughput, not for latency.
This is because this feels like something that runs batch jobs.

In order to make this into a server that reads data from TCP connections, 
the change would be super simple, since this already uses a thread, adding more threads would be trivial.

### Error handling 

- Fails fast on bad input 
- Continues processing on "partner" mistakes.

### Performance

- Separates CSV parsing from transaction processing using a background thread. This is because the parsing is the slowest part. I played around with more threads but since the bottleneck is parsing they were not necessary so I removed them.
- Groups transactions into batches of 32 to reduce atomics overhead.
- Streams data rather than loading entire datasets into memory
- This approach minimizes useless memory clones by moving just the heap ptrs when data volume is high.


### Type safety

- Uses strong typing with custom types for Money, ClientId, and TxId.
- I specifically designed the data types so that they're impossible to misuse (see Raw* tys)
