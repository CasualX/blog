---
layout: post
title: "Why I forked rand"
author: "Casper"
categories: rust
---

Rust's `rand` crate is the default choice for randomness in the Rust ecosystem.
My experience with `rand` has frustrating papercuts: many everyday operations are hard to discover!

That led me to fork and build [`urandom`](https://crates.io/crates/urandom):
a randomness crate with a smaller public and implementation surface, organized around a specific user experience.

To quote:

> “Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away.” ― Antoine de Saint-Exupéry

## One place to look

With `rand`, useful operations come from several traits:

```rust
use rand::RngExt;
use rand::seq::{IndexedRandom, SliceRandom};

let mut rng = rand::rng();

let roll: u32 = rng.random_range(1..=6);
let color = ["red", "green", "blue"].choose(&mut rng);

let mut numbers: Vec<_> = (1..=10).collect();
numbers.shuffle(&mut rng);
```

Rand 0.10 also provides root-level helpers such as `rand::random_range` for one-off calls.
Once you keep an RNG handle, or need sequence operations such as
choosing and shuffling, the method API still comes from several traits.

The prelude shortens the imports, but you still need to know to which types the extension methods apply.
IDE completion does not help much when the method may belong to the RNG, the slice, the iterator or elsewhere.
Discovering the right method often requires already knowing which trait provides it on which type.

`urandom` puts its high-level consumer API on one `Random` wrapper struct:

```rust
let mut rand = urandom::new();
// rand: urandom::Random<urandom::rng::Xoshiro256Rng>

let roll: u32 = rand.uniform(1..=6);
let color = rand.choose(&["red", "green", "blue"]);

let mut numbers: Vec<_> = (1..=10).collect();
rand.shuffle(&mut numbers);
```

If you have a `Random`, editor completion shows you `random`, `uniform`, `chance`, `choose`, `shuffle`, `sample`, and more.
These are inherent methods, so there is no high-level extension trait to find or import.

## A sealed `Rng`

`rand` treats its low-level RNG traits as public extension points. `urandom` does not:
its `Rng` trait is sealed, and the supported generators are selected and implemented inside the crate.

- You cannot plug an arbitrary generator into `Random`.
- A new generator requires a change to `urandom`.

I kept coming back to a core question: what material benefit does customizing the generator provide?

If the goal is a better algorithm, Xoshiro256 and ChaCha are established
default choices for their roles today, and recommendations change slowly.
If that changes, `urandom` can adopt a better choice in a future major release.

If the goal is compatibility with another project, programming language, legacy
algorithm, specialized hardware, or simulation-specific generator, matching its
generator alone is not enough. Uniform sampling, shuffling, and other
algorithms must match too. A dedicated implementation of that complete contract
is a better fit.

Sealing `Rng` lets me shape it around my crate instead. I can add exactly the
primitives `urandom` needs without designing and documenting an implementation
contract for unknown generators and their special cases. This lets the generator
and algorithms specialize for each other, enabling niche optimizations not
available to `rand`.

For most applications, choosing the entropy is more useful than implementing a
new PRNG. The concrete generators expose their native `from_seed` constructors:

```rust
fn from_my_entropy(seed: [u32; 8]) -> urandom::Random<urandom::rng::ChaCha12Rng> {
    urandom::rng::ChaCha12Rng::from_seed(seed)
}
```

This is narrower than accepting any RNG implementation, but it preserves the
extension point I expect advanced users to need.

## Beating `rand` without a better algorithm

My fork is not based on novel random-number algorithms.
On 64-bit systems, `urandom::new()` and `rand::rngs::SmallRng` use the same
_Xoshiro256_ family for non-cryptographic use. `urandom::csprng()` and
`rand::rngs::StdRng` use _ChaCha12_ for cryptographic use. Rand's convenient
`rand::rng()` handle is backed by ChaCha12.

`rand`'s generator interface exposes integer words and byte filling.
A distribution that wants an `f64` starts by asking for a full `u64`.

`urandom::Rng` also has specialized `next_f32` and `next_f64`:

```rust
pub trait Rng: Sealed {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;

    fn next_f32(&mut self) -> f32;
    fn next_f64(&mut self) -> f64;

    // byte filling omitted
}
```

Random floats need fewer random bits than a full word. A generator can therefore override these methods with a cheaper output function.

The Xoshiro family of algorithms support exactly that.
The implementation retains Xoshiro256++ for `u64`, while `u32` and floats use the faster Xoshiro256+.
Both share the same state advance, but Xoshiro256+ has a cheaper output function whose upper bits are designed for this use case.

On my system, comparing the current `urandom` 1.0 implementation with `rand` 0.10.2,
the end-to-end Xoshiro `f64` benchmark measured about 31% higher throughput (24% faster) in this microbenchmark.
The `u64` paths, where both do the same work, were effectively tied.
The [full benchmark notes](https://github.com/CasualX/urandom/blob/master/docs/benchmarks-rand.md) for more.

| 1,000 random values | `rand` | `urandom` |
| --- | ---: | ---: |
| Xoshiro `u64` | 814 ns | 814 ns |
| Xoshiro `u32` | 836 ns | 788 ns |
| Xoshiro `f64` | 1,033 ns | 788 ns |
| ChaCha12 `f64` | 2,199 ns | 2,011 ns |

Exact timings depend on the machine and compiler. Here, the richer interface
lets Xoshiro select a cheaper output function for `u32` and floats.
ChaCha12 does not override `next_f64`; hence the equivalent performance.

## Unified uniform sampling

Uniform integer sampling hides a surprising amount of complexity. Reducing a
random integer modulo the length of a range introduces bias, so a correct
implementation must reject part of the generator's output. Finding the exact
rejection threshold requires an expensive modulo operation: reasonable setup
when a sampler will be reused, but expensive for a single value.

Rand exposes that distinction through its `UniformSampler` trait. A constructed
`UniformInt` calculates the threshold up front and samples without bias.
`Rng::random_range`, meanwhile, goes through the separate `sample_single` or
`sample_single_inclusive` hook to avoid that setup, extra methods which are easy
to overlook. With rand's default features, this shortcut uses a second
algorithm which is slightly biased; the optional `unbiased` feature substitutes
a more involved looping version.

My implementation avoids the split by calculating the threshold lazily, allowing
`UniformInt` to use one unbiased multiply-and-reject implementation everywhere exactly as described by Daniel Lemire (2018) in [*Fast Random Integer Generation in an Interval*](https://arxiv.org/abs/1805.10941):

```rust
let mut zone = range;

loop {
    let value = rand.next_u64();

    // Handle edge case when full range is requested
    if range == 0 {
        return value;
    }

    let (high, low) = widening_multiply(value, range);

    if low >= zone {
        return base.wrapping_add(high);
    }

    if zone == range {
        zone = range.wrapping_neg() % range;
        if low >= zone {
            return base.wrapping_add(high);
        }
    }
}
```

For most practical ranges such as the one benchmarked below, the first candidate almost
always returns before the division. Otherwise the code computes the exact
threshold and continues without introducing bias.

There is no special method to discover, no second algorithm, no eager setup
cost, and no biased fast path. The same implementation handles both reusable
distributions and one-off ranges and it was faster than both rand paths in my
benchmark of 1,000 samples from `500..20_000`:

| Sampling method | `rand` | `urandom` |
| --- | ---: | ---: |
| Reused `UniformInt` | 1,098 ns | 950 ns |
| One-off range | 1,079 ns | 942 ns |

These rand results use its default features, so the faster one-off row is the
slightly biased path. `urandom` is faster it while remaining unbiased!

## Reproducibility policy

`urandom` treats reproducibility as part of its public contract. Given the same
explicit seed and sequence of low-level rng calls, its
deterministic generators keep their raw output stable across supported
architectures and SemVer-compatible releases. This gives a 64-bit server and a
32-bit WebAssembly client the same generator foundation for replay.

This means a sacrifice of performance on 32-bit architectures in favor of compatibility!

This is a stronger promise than [`rand` makes](https://rust-random.github.io/book/crate-reprod.html),
where portable generators and sampling algorithms may change their output in a
minor release. Rand's `SmallRng` and `StdRng` are explicitly non-portable and
may also change across platforms or library releases.

## The trade-off

`urandom` is the randomness crate I want for my own projects:

- Common operations live on `Random`, where they are easy to find without importing extension traits.

- Its generators and distributions are designed together. That made cheaper
  Xoshiro output paths and one unbiased path for uniform sampling practical.
  The benchmarks above show the resulting end-to-end performance.

- Explicitly seeded generators produce stable raw streams across supported
  architectures and SemVer-compatible releases, useful for deterministic games and simulations.

The price is choice. You cannot bring an arbitrary generator, and the `rand`
ecosystem has a much larger catalogue of distributions and third-party integrations.

Use `rand` when you need that ecosystem.
Give my crate `urandom` a try if you find yourself aligned with my values!

You can find `urandom` on [crates.io](https://crates.io/crates/urandom/1.0.0-alpha.1),
read the [latest API documentation](https://docs.rs/urandom/1.0.0-alpha.1/urandom/),
or browse the [source on GitHub](https://github.com/CasualX/urandom).
