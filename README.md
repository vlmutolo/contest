# Contest

This is a small experiment to test the effects of unsynchronized,
non-atomic writes to a single `u64`.

## The Experiment

The following code is basically pseudocode in Rust syntax. It
won't compile because it's not safe.

Let's start with a single 64-bit unsigned integer.

```rust
let n: u64 = 0;
```

Then, if we generate a random number `r` and `xor` it with `n`
an even number of times, `n` should still be zero.

```rust
let r: u64 = rng.gen();
for _ in 0..1024 { // the upper bound is any even number
    n = n ^ r
}
```

Things get more interesting if we involve concurrency. The
tricky part is:

```rust
n = n ^ r
```

If the load on the right and the assignment on the left aren't
done atomically, it's possible that another thread will operate
on `n` in the middle of our "fetch-xor" operation. This will
break our invariant of `xor`ing an even number of times.

## Implementation

The code accomplishes unsynchronized data access with `UnsafeCell`: 

```rust
#[derive(Clone)]
pub struct SharedUnsync(Arc<UnsafeCell<u64>>);

impl SharedUnsync {
    fn fetch_xor(&self, other: u64) {
        // SAFETY: very unsafe.
        unsafe { *self.0.get() ^= other }
    }
}
```

We also have something like a control group using `AtomicU64`:

```rust
#[derive(Clone)]
pub struct SharedAtomic(Arc<AtomicU64>);

fn fetch_xor(&self, other: u64) {
    self.0.fetch_xor(other, Ordering::Relaxed);
}
```


### Rust Version
```bash
$ rustc --version
rustc 1.58.1 (db9d1b20b 2022-01-20)
```

## Results

### Both unsync and atomic

The two `fetch_xor` operations ("atomic" and "unsync") are executed
sequentially in the source. When both are enabled, we see that both
the atomics-synchronized and the unsynchronized values are corrupted.
Strangely enough, they're corrupted in the same way even though they're
two completely different locations in memory (or at least should be).

```
unsync: 1010101111011010001100111101101011001110001010001101010010100001
atomic: 1010101111011010001100111101101011001110001010001101010010100001
```

### Only unsync

When we comment out the atomic `fetch_xor` operation, we *still* get both
values back corrupted. This particularly surprised me since the atomics-
synchronized value is never even written to (or shouldn't be).

```
unsync: 1100110111110110011011000001100011111000101001111111010001111011
atomic: 1100110111110110011011000001100011111000101001111111010001111011
```

### Only atomic

Finally, if we only leave the atomic `fetch_add` operation in the source
and comment out the unsynchronized writes (effectively commenting out the UB)
we get zero back like we'd expect.

```
unsync: 0000000000000000000000000000000000000000000000000000000000000000
atomic: 0000000000000000000000000000000000000000000000000000000000000000
```

Note that we left both values initialized to zero in
each of the above variations.
