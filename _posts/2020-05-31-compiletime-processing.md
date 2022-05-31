---
layout: post
title: "Compiletime processing in Rust"
categories: rust
---

Ever since proc-macros were stabilized I explored their utility for compiletime processing. This approach using proc-macro has its drawbacks: it's an extra dependency which takes time to build and lack of sharing implementation used by the proc-macro and the main crate if you want both runtime and compiletime functionality.

With the recent progress on const and generic functions in Rust I decided to see if I can rewrite some of my proc-macros to use const generics instead.

## Part I: Compiletime string processing

The crate under inspection is my [`obfstr`](https://crates.io/crates/obfstr) crate which (among some other utilities) provides compiletime string obfuscation. I want to rewrite it using const fn instead of using proc-macros.

The idea of string obfuscation is to avoid baking the string literal as-is in the binary and instead store an obfuscated version of the string which gets deobfuscated as needed.

Let's define the desired interface:

```rust
// An instance of this struct is baked in the binary
struct ObfString<A> {
	key: u8,
	data: A,
}

// Given a string literal produces the desired obfuscated string at compiletime
pub const fn obfuscate<const LEN: usize>(s: &str) -> ObfString<[u8; LEN]>;
```

With this function signature we still have to specify the string length separately but fortunately this can be remedied using a `macro_rules` macro:

<!-- {% raw %} -->
```rust
macro_rules! obfuscate {
	($s:literal) => {{
		const STRING: ObfString<[u8; {$s.len()}]> = obfuscate::<{$s.len()}>(s);
		STRING
	}};
}
```
<!-- {% endraw %} -->

Note the use of `{$s.len()}` when passing the length as the const generic argument. It would be nice if there was some way for some kind of type inference to deduce the LEN parameter based on the input's length.

Putting it together: [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=ff10a24947a0c247396cca2e099ba445)

Of course this isn't yet very useful but we've managed to define the basic ingredients and the Rust compiler doesn't complain!

So let's implement the obfuscate function:

```rust
pub const fn obfuscate<const LEN: usize>(s: &str, key: u8) -> ObfString<[u8; LEN]> {
	let s = s.as_bytes();
	let mut data = [0u8; LEN];
	let mut i = 0usize;
	while i < s.len() {
		data[i] = s[i] ^ key as u8;
		i += 1;
	}
	ObfString { key, data }
}
```

To make this code work some magic is required. First of `for` loops are not supported but while loops are.

Inspect a complete example on the [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=f454e8966aadcc0372d4fc4ae3187c6c), it prints:

```
ObfString { key: ca, data: [a2, af, a6, a6, a5] }
```

Success!

Another example to blow your mind is a compiletime UTF-8 to UTF-16 converter: [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=3f65a51ef44e449f598016eb244a5291)

```rust
fn main() {
	let actual = wide!("🌍");
	let expected = &[0xd83c, 0xdf0d];
	println!("{:x?}", actual);
	assert_eq!(actual, expected);
}
```

No proc-macros required, just a const generic function :)

# Part II: Compiletime random numbers

In the previous examples the key `0xca` was hardcoded for the obfuscation. It would be much nicer if this key could be random and per invocation of the obfuscate macro without any input from the caller.

This may seem to be impossible as that would imply the source code is no longer deterministic but this isn't the case, welcome our good friends `file!()`, `line!()` and `column!()`. These macros return the file name, line and column number where they are invoked.

We can use these values as entropy to feed into a pseudo-random number generator (PRNG). Luckily for us these values are filled in the actual source location they're used (and not the macro location they're defined) so we can wrap them up in a helper.

In order to do this we'll need to process the `file!()` string into a number, but that's simple enough through our good ol' friend DJB2 compiletime string hash:

```rust
pub const fn hash(s: &str) -> u32 {
	let s = s.as_bytes();
	let mut hash = 3581u32;
	let mut i = 0usize;
	while i < s.len() {
		hash = hash.wrapping_mul(33).wrapping_add(s[i] as u32);
		i += 1;
	}
	return hash;
}
```

For the final mixing we'll use the SplitMix PRNG:

```rust
pub const fn splitmix(seed: u64) -> u64 {
	let next = seed.wrapping_add(0x9e3779b97f4a7c15);
	let mut z = next;
	z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
	z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
	return z ^ (z >> 31);
}
```

For fun let's mix in some external entropy defined by the user in the form of an environment variable (supported by a build script which defines a dummy value in case the env var is not defined):

```rust
pub const SEED: u64 = splitmix(hash(env!("OBFSTR_SEED")) as u64);
```

Putting it all together to turn `file!()`, `line!()` and `column!()` into pure entropy:

<!-- {% raw %} -->
```rust
pub const fn entropy(file: &str, line: u32, column: u32) -> u64 {
	splitmix(SEED ^ (hash(file) as u64 ^ (line as u64).rotate_left(32) ^ (column as u64).rotate_left(48)))
}

macro_rules! entropy {
	() => {{
		const ENTROPY: u64 = entropy(file!(), line!(), column!());
		ENTROPY
	}};
}
```
<!-- {% endraw %} -->

See it in action: [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=5d0ec37da9d8bbe4198d3c6370ff5a52)

It should reliably print `17854532005703890967, 8926035397751327455` for everyone. Tiny pertubations in the source code will produce different entropy, even shifting the line with the `entropy!()` macro with a single space character.

This keeps the whole thing random yet deterministic.

## Part III: Compiletime parsing

While I'm happy I was able to convert this crate to use const fn for its implementation, I have some other proc-macros that I cannot yet convert.

Let's say you've created some [Domain-specific language (DSL)](https://en.wikipedia.org/wiki/Domain-specific_language) and wrote a parser for that DSL in Rust which produces some bytecode in the form of `Vec<u8>`, eg:

```rust
pub fn parse(s: &str) -> Result<Vec<u8>, Error>;
```

Of course at runtime the result's length is derived from the contents of the input, but at compiletime the result has a deterministic length! It would be silly to force an allocation of `Vec<u8>` when the parser was const evaluated at compiletime. Ideally I would want to get back a `[u8; _]` to bake the result of the parser directly in my binary.

Reduced to its most essential it is the following idea; why can't this code be evaluated at compiletime?

```rust
const ANSWER: &str = &42.to_string();
```

To be more specific, the clever trick from earlier _should really look_ like this instead:

```rust
pub const fn wide(s: &str) -> &'static [u16] {
	&"🌍".encode_utf16().collect::<Vec<u16>>()
}

const STRING: &[u16] = wide("🌍");
```

Using a proc-macro this is all possible today but I hope some day in the future this could be replaced by simple const fn.

## Part IV: Compiletime all the things

Const functions are really nice and can replace some usage of proc-macros. I hope to see const functions further developed and stabilized in the Rust language.

Specifically when it comes to traits (const traits? const trait methods?) there is a huge world to explore.

Thanks for reading!
