---
layout: post
title: "My Personal Rust 2019"
categories: rust
---

Many people have already posted their thoughts on the future of Rust. There are some _excellent_ ideas in there that are probably more important than what I have to say.

On the other hand my blog post covers my personal experiences I've had in the few years I've been writing Rust.

## Customizable prelude

The prelude are a list of symbols available without having to import them. This is why you can use `String`, `Vec`, `Iterator`, etc. without explicit use statements greatly improving usability.

With the recent release of Rust Edition 2018, the prelude was extended to include all the extern crates, allowing them to be referenced in any submodule without an import statement.

Eg. `impl serde::Serialize for ...` without a `use serde;` statement.

Sometimes I'm working with a crate which is fundamental to the code I'm writing. The crate provides a set of types and traits that I want to be universally available in my code. Eg. vector and matrix types in video game code.

This becomes especially more powerful with traits, which can add methods anywhere. Eg. some helper methods that are universally used in your code. It becomes a real chore to have to import these all the time.

To accomodate this friction, the ability to customize the prelude (only for _your_ code) would be really sweet. Some bikeshedding syntax could be an attribute which when applied to a type, trait, fn or use statememnt would make that item available in your code's custom prelude:

```rust
#[prelude]
use rand::Rng;

#[prelude]
use cgmath::prelude::*;

#[prelude]
trait Foo {
	fn helper_foo(&self) -> i32;
}

#[prelude]
fn helper_foo() -> i32 { 42 }
```

All these items marked with this attribute would be available in the prelude of _your entire_ crate.

The specific details are up for bikeshedding of course, but the underlying idea feels attractive to me. It allows to define a custom set of 'primitive' items specific to the domain of your code compared to the more general purpose primitives defined by the Rust langauge's standard prelude.

## The Rust Language Server experience

I know this topic has been beaten to death but...

At $dayjob I work with C++, C# and recently some python. My main drivers are Visual Studio and VS Code. I get things done in these languages, maybe not the prettiest ways but work done is work done.

I find it harder to be productive in Rust.

I've struggeled to explain why this is the case, but I think I have found some factors. When working in these other languages I enjoy *excellent* IDE support. At every occasion I just type `.` or press F12 and I get basically instant, high quality feedback.

Even with lacking documentation, exploratory programming with just seeing the names and parameters of the methods can already fill a huge gap.

The experience with RLS... just doesn't compare. Yes Rust's documentation is excellent, but when RLS dies I am left thinking 'what method appends to a String again?', 'what was the key value pair type for iterating a HashMap again?' When switching between languages these things slip my mind, and I just draw a blank. This just happens too often.

I sigh, alt-tab, go open docs.rust-lang.org and type in the name of the type I want to look at. This just completely kills my momentum and takes me out of my zone. The fact that I have to type the name of the type in Rust's documentation search is already too much (instead of just being one press of F12 away).

On top of that the RLS was just broken for version 1.30.0, I thought I was going crazy that RLS was even less reliable than usual. [Turns out RLS was just broken for 2 weeks](https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1311-2018-12-20).

Don't get me wrong, I understand making an language service is extremely difficult and time consuming work (for _any_ language, but perhaps Rust more so than others). But a barely functioning RLS has a huge impact on productivity and satisfaction from developing in Rust.

I don't care that it is slow, my projects aren't massive codebases; but when autocomplete fails for simple std library types forcing me to lookup the documentation elsewhere, it just hurts.

## UTF-16 string literals

Working with the Windows APIs which talk in 'nul terminated wide strings' (basically unchecked UTF-16 strings) is very cumbersome.

When the input is dynamic, it isn't so bad with the [`std::os::windows::ffi::OsStrExt` trait](https://doc.rust-lang.org/std/os/windows/ffi/trait.OsStrExt.html). However when the string is a constant then this overhead of dynamic memory allocation is not acceptable.

I've made [a tool to encode strings as UTF-16](https://casualhacks.net/rustvc.html) generating Rust array declarations but this is far from convenient.

This could be solved by custom literals in Rust (but I understand the design space here is huge, making it less likely to succeed). The use case for this syntax is probably not big enough to warrant dedicated syntax.

Perhaps with the proc macros being stabilised this could be implemented as such, eg. `L!("hello world")` to produce a `&'static [u16]` instance.

I've never implemented a proc macro, sounds like a nice project to start the year!

## Support for `Cell` field projection

Given a `&Cell<T>` I'd love the ability to get cell references to fields of T. This ability goes under the name of 'cell field projection' and is only referenced in a handful of places.

This ability is completely absent in Rust, it's not even possible to create a macro wrapping the necessary unsafe transmutes (while you can get the offset of a field with unsafe code, the transmute requires to know the type of the field as well).

One of [my projects](https://github.com/CasualX/pelite) works with intricate binary data structures, the [Portable Executable](https://en.wikipedia.org/wiki/Portable_Executable) file format.

I've currently modeled it with an API that maps images in memory creates shared references into the mapped data. This enables zero copy APIs and works very nicely if all you need to do is read information, however writing back to the image is not possible at all because of the shared references.

It is not possible to use unique references as it becomes hard to prove they are disjoint. Imagine the desire to modify the image based on some other information in the binary. It is still possible to support mutation if the reading and writing of the image are strictly separate phases however mutating the information on which the modifications are being made may be the entire point.

To support the kind of API I desire would require to access the mapped memory as a `&[Cell<u8>]` followed by creating `&Cell<repr C struct>` references on specific offsets. Now it becomes a pain to modify this struct.

I've no idea what a solution (or even workaround) would look like.

## Better support for unsafe patterns

I have an experiment where I wrote a Windows device driver in Rust. It doesn't do anything specific, just some experimenting with what I can do from the kernel side. This case is essentially unsupported in Rust, which is fine! Just slap some `no_std` and carefully call driver functions without any wrapping while manually managing the resources.

This becomes incredibly tedious really quickly! Specifically when dealing with raw pointers. On occasion I've found myself converting raw pointers into unique references without really caring if those pointers were really unique _just_ because it was so much more convenient.

A more convenient way to access fields of a struct behind a pointer in a 'safe' way (without the uniqueness constraint of unique references) with `->` would be really nice. I know this will probably never happen, but it is something I've struggled with.

Another sticky point when working with C APIs are out parameters. Out parameters are pointers to uninitialized memory which the C API is supposed to initialize. Rust really doesn't like these, requiring use of the much maligned `std::mem::uninitialized` or just taking the hit and initializing with a dummy value. Most of the time that isn't a big deal, but it does annoy me.

Rust proper (without the intricacies of C APIs) also touches this subject with output buffers (eg. `std::io::Read`) for which (imho) elaborate APIs are being designed where I feel a direct language approach would be better suited.

How that direct language approach would look like, I've no idea.

## Conclusion

Rust rekindled my love for programming after so many years of C++. Rust taught me what memory safety really means and made me a better C++ developer.

These issues are not breaking, not even close to making me go back to C++ for hobby projects, I will continue to write my hobby projects in Rust in 2019 and beyond.

Happy new year! 🎆

Discuss on [reddit](https://old.reddit.com/r/rust/comments/abtf9c/my_personal_rust_2019/)
