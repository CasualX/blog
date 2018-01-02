---
layout: post
title: "Analysis of base64 encoders and decoders"
categories: rust
---

# Analysis of base64 encoders and decoders

Some time ago I had the need for simple base64 encoding and decoding. So being a good Rustacean I looked them up on [crates.io](https://crates.io/search?q=base64).

Before I started looking I read up on the [base-N]() RFC.

Note that I didn't actually have a need for any of these features.

I had some requirements in mind of how an ideal base64 crate should work:

* _Does it support various base64 alphabets and quirks?_

  Specifically handle optional padding characters, ignoring invalid characters (instead of erroring) and various alphabets.

* _Does it support static dispatch for the alphabets?_

  This tests if the library allows abstraction without overhead. Optionally can it handle dynamic configuration?

* _Does it have a streaming API?_

  Specifically I had in mind an iterator adapter API where the encoder adapts a byte iterator into a char iterator and vica versa.

* _Does it strictly check for canonical input?_

  This is an interesting edge case I read about in the base-N RFC.

## Crate: base64

The first crate that shows up is [`base64`](https://crates.io/crates/base64) on [github](https://github.com/alicemaz/rust-base64).

The readme asks us "_What more could anyone want?_", quite a lot actually :)

Time to dig into the code itself:

The API does not support streaming and it looks like it dynamically dispatches the alphabet.

The alphabet is defined with the [`Base64Mode`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L31) enum, confirming the dynamic dispatch.

The [`Base64Error`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L38) is peculiar, there should be no need to check utf8 correctness in a base64 encoder or decoder. Either you accept valid utf8 input or you spit out base64 characters (which are all in ascii range) and unless you're doing something very weird you just shouldn't ever need to test this.

Looking deeper into the code the only place it could potentially have been used is in the [`decode_ws`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L135) function but this just unwraps. This should just use the `String::from_utf8_unchecked` function as removing valid ascii codes from a valid UTF8 string will always remaing a valid UTF8 string.

Also missing from the error case where the base64 encoded text is denormal. This happens when the end of the string has bits set that aren't used when decoded, the RFC4648 says such strings are ["not canonical"](https://tools.ietf.org/html/rfc4648#section-3.5).

We're getting side tracked here, let's dig into the meat following `decode` down the layers of wrappers:

[`decode`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L111) is a convenience helper calling [`decode_mode`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L259) with the default encoding which allocates the destination buffer and dispatches to [`decode_mode_buf`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L286) where the magic is happening.

It starts by reserving memory in the destination buffer (it already did this in `decode_mode` thus overallocating!).

It also assumes the only difference between base64 alphabets is just the alphabet! In fact, other differences is in the padding character (if the encoding has one at all or it is optional). This is where dynamic dispatching the encoding falls down on its face.

The inner loop looks ok, unrolled processing 8 bytes at the time. Followed by handling the last few bytes and padding.

The encoding is far more simple, it happens in [`encode_mode_buf`](https://github.com/alicemaz/rust-base64/blob/master/src/lib.rs#L198). It has the same defect of reserving twice the amount of required memory. Furthermore it pushes the individual utf8 bytes individually, which results in constant checking if the backing buffer needs to be resized somewhat defeating the purpose of reserving memory ahead of time. (This may be optimized by a sufficiently advanced compiler but I didn't check if that happens here).

Testing is fairly extensive.

Overall this library will work, but I think Rust deserves better.

## Crate: base64-rs

Following is [`base64-rs`](https://crates.io/crates/base64-rs) on [github](https://github.com/asukharev/base64-rs).

Zero documentation, and minimal API surface. There's something to be said for simplicity but I feel this is a little on the dry side.

Time to dig into the code itself:

The [`decode`](https://github.com/asukharev/base64-rs/blob/master/src/lib.rs#L57) API accepts only `&String` whereas the only thing it does with it is call `as_bytes()` on it. In this case it should just accept a `&str` or even `T: AsRef<[u8]>` but that may just be overkill.

The result buffers do not preallocate a capacity to avoid reallocations. There's only a one-way look up table for encoding, decoding iterates over the lookup table... Tsk.

I can only guess which base64 variant is implemented. Finding out would require decoding the lookup table manually looking up ascii codes. Probably the most popular encoding.

The implementation is very naive (not that there's anything wrong with that!).

Decoding will panic if the input string length isn't a multiple of 4 (incurring 4 bound checks, ☹), I don't know what happens to invalid characters...

Zero testing, even though Rust makes it so easy.

And the worst of all, lacking an extra newline at the end of the source file. Madness!

Overall... Not recommended.

## Crate: data-encoding

I was looking forward to reviewing this one! [`data-encoding`](https://crates.io/crates/data-encoding) on [github](https://github.com/ia0/data-encoding).

Lovely documentation, extensive feature set and looks professional.

So let's dig in the code and see how this works:

At the top level in `lib.rs` the implementation is defined through the [`base!`](https://github.com/ia0/data-encoding/blob/master/src/lib.rs#L123) macro. Supplied are all the bits required to define the encoding. These encoding details are then stored in an instance of `Opt<Static>` where each `Static` is unique for the encoding. I think the point is that this forces every 'configuration' struct to be a unique type allowing constant folding optimization.

The encoding and decoding can then be generic over the configuration.

I'm not sure why this simply couldn't have been a trait instead of an `Opt<T>` struct which will guarantee monomorphisation without having to trust constant folding will happen.

The generic [`encode`](https://github.com/ia0/data-encoding/blob/master/src/encode.rs#L80) and [`decode`](https://github.com/ia0/data-encoding/blob/master/src/decode.rs#L127) with allocation zero-initialize their buffers which isn't necessary before dispatching to [`encode_mut`](https://github.com/ia0/data-encoding/blob/master/src/encode.rs#L52) and [`decode_mut`](https://github.com/ia0/data-encoding/blob/master/src/decode.rs#L87) respectively.

These functions assert that the output length is exactly as expected, which I'm not sure is necessary. They could accept output buffers which are larger and just return the amount of data written.

The input is processed in chunks which are most natural for the encoding, the inner loop looks very clean and simple.


