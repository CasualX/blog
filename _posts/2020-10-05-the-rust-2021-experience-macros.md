---
layout: post
title: "The Rust 2021 Experience - Year of the Macro"
categories: rust
---

Part of a series for the [Call for 2021 Roadmap Blogs](https://blog.rust-lang.org/2020/09/03/Planning-2021-Roadmap.html).

Macros enable writing powerful domain-specific languages within the Rust language itself. When I finally mastered declarative macros, tt-munchers' power allows me to write intricate and complex macros.

When the procedural macro MVP was stabilized last year, it was love at first sight. I can do so much more than what was possible with declarative macros, including a much wider range of syntax transformations as well as attribute and derive syntax. However, the honeymoon period is over, and its rough edges are exposed.

So here is my wishlist for macros in 2021:

### Keep improving const fn

I've explored using procedural macros as a more powerful version of const fn, but it has so many rough edges that make them borderline unusable for this purpose. Let's demonstrate with an example: I wrote a [small language](https://docs.rs/pelite/0.9.0/pelite/pattern/fn.parse.html) for matching code patterns in binaries. It comes with a parser with the following signature:

```rust
pub fn parse(pat: &str) -> Result<Vec<Atom>, ParsePatError>;
```

This parser is commonly invoked with a string constant, and in those cases, I would like to parse them at compiletime and return a `&'static [Atom]` directly (or fail compilation if there's an error).

Wrapping this parser as a procedural macro leads to the following issues:

Because procedural macros live in separate crates, it becomes hard to share this parsing code between the procedural macro and the runtime parser. I used a trick to hotlink the same source code in both crates:

```rust
#[path = "../pattern.rs"]
mod pattern;
```

While this works locally, crates.io does not accept this as technically the path links to a file outside the procedural macro's crate root (separate from the project's crate root). This horribly mangles my crate release procedure...

Ideally, this should be a const fn, but you see the issue of how a const fn is supposed to turn that `Vec<Atom>` into a `&'static [Atom]`? An idea I have is to turn [`Vec::leak`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.leak) into a const fn, which would allow writing a declarative macro which wraps this parser for compiletime evaluation:

```rust
#[macro_export]
macro_rules! parse {
	($s:expr) => {
		{
			// Make a const to force compiletime const fn evaluation
			const PAT: &[$crate::pattern::Atom] =
				$crate::pattern::parse($s).unwrap().leak();
			PAT
		}
	};
}
```

I understand that const fn is not ready for this use case, but I hope it will be powerful enough someday.

### Proc-macro sister `$crate`

When designing a declarative macro that depends the current crate items, this useful meta macro variable called `$crate`, which expands to whatever path is necessary to resolve to the crate the macro is defined in.

The `$crate` variable is necessary to allow a declarative macro to work correctly within the crate that defined it and when used by downstream users. e.g.

```rust
pub struct Foo {}

#[macro_export]
macro_rules! Foo {
	() => { $crate::Foo{} };
}
```

When the macro is expanded within the crate that defined it without the `$crate::` prefix, it works correctly as `Foo` is in scope. Now let's expand it in a foreign crate: the `Foo` symbol would need to be explicitly imported, which may not be desired. When replacing the `$crate::` prefix with `::crate_name::` the macro functions correctly when invoked by a foreign crate but not within the crate that defined it. There's a similarity with the `crate::` pseudo module to refer to items from your crate.

Unfortunately, procedural macros have no such luxury. Because they are defined in a separate crate they have no explicit way to refer to items defined in a sister crate. Once you start re-exporting procedural macros and [renaming dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml), things go south quickly.

In essence, I'm missing the ability to statically 'link' the procedural macro to the items defined in its sister crate. Rust is tracking this issue on its [issue tracker](https://github.com/rust-lang/rust/issues/54363).

For me, this causes friction when creating procedural macros that don't stand alone and makes me try building the desired macro as a declarative macro before using the procedural macro hammer to solve my problem.

### Finalize the long-awaited declarative macro 2.0 syntax

A post about declarative macros can't be complete without mentioning the new and improved macros by example 2.0: Looking at its [tracking issue](https://github.com/rust-lang/rust/issues/39412), it seems to have stalled a long time ago.

I'd love to see this feature finalized and pulled over the finish line.

### Declarative attribute and derive macros

Procedural macros have another cool superpower that they can be used with the `#[attribute]` and `#[derive(Trait)]` syntaxes.

However, the restriction that procedural macros must live in a separately published crate causes a lot of friction and headaches. The ability to use declarative macros as attribute and derive macros is my top wish.

Someone made a procedural macro adapter for this purpose: [macro_rules_attribute crate](https://crates.io/crates/macro_rules_attribute). However, the experience is not seamless, so I would love to see this supported by Rust out of the box.

### Eager evaluation of macro arguments

The arguments provided to macros are called 'token trees'. They are straightforward and contain no further meaning other than balancing brackets. Anything that is a valid Rust token is allowed to come in any order.

What happens if you were to 'invoke' a macro in the arguments of another macro? Example:

```rust
let x = foo!(concat!("hello", "world"));
```

The result is nothing special; the arguments are passed as a token tree: `[Ident(concat), Punct(!), Group([Literal("hello"), Punct(,), Literal("world")])]`. It is a desired feature have the option to expand a macro before passing its output into another macro.

Eager macro expansion is a long-standing feature request, with an open [RFC PR#2320](https://github.com/rust-lang/rfcs/pull/2320).

### Create hygienic identifiers

Today it is impossible to create identifiers within a declarative macro from a concatenation of identifier parts. This prevents macros from generating boilerplate code, which involves producing a bunch of similarly named functions.

The idea I have is outlined by the following built-in macro defined by the Rust compiler which provides a safe and targetted opt-out of macro hygiene:

```rust
macro_rules! create_ident {
	($scope:ident, $($parts:ident)+) => { ... }
}
```

The idea is that you have to provide a _scope_ in which the identifier should be created, followed by a list of identifier parts that should be concatenated to form the final identifier. These rules keeps the resulting identifier hygienic.

### Named and default value arguments syntax extension

Rust attribute macros work at the AST level, meaning that its input must be parsed as valid Rust code for an attribute macro to be invoked. This limitation restricts how attribute macros can be used to experiment with Rust language extensions.

It would be cool to see fewer restrictions on the syntax parsing of Rust. Of course, the code would still produce an error later in the pipeline, but delaying the reporting of the error would allow attribute macros to rewrite the code into valid Rust.

This becomes especially interesting with named and default arguments, eg.

```rust
#[named]
pub fn foo(a: i32 = 42) {
	//...
}
```

This would remain invalid Rust code today, but delaying the reporting of the error, the attribute macro could rewrite this in interesting ways.

### Macros in method call position

Finally, I'd love to see macros allowed in method call position:

```rust
// This is the desired syntactic sugar:
let x = 42.foo!(arg).baz!(a: 13);

// Rewritten like this where _tmpN is captured as an opaque $expr:
let x = match 42 {
	_tmp1 => match foo!(_tmp1, arg) {
		_tmp2 => baz!(_tmp2, a: 13)
	}
};
```

Provide the evaluated `self` as the first argument (captured as an opaque `expr` fragment) to the macro and pass any additional tokens to the macro. This syntactic sugar is not to allow the macro to inspect the `self` argument but to focus on interesting designs possible with the macro arguments.

There has been some discussion on this topic, also called 'postfix macros': [[1]](https://old.reddit.com/r/rust/comments/6eegy3/macros_method_call_syntax/) [[2]](https://github.com/rust-lang/rfcs/pull/2442)

There are a lot of interesting trade-offs:

Should `self` be passed as tokens or an opaque expr? I'm in favor of the latter. The focus is on the macro arguments, not the receiver. This also ensures the receiver is evaluated exactly once.

Should it be possible to scope the macro to certain types? Now it's getting interesting. Because macros are expanded before types are inferred, I'm not sure how feasible it is to implement. This feature would be entirely backward compatible with 'type aware' macros being added in the future. Here it would only look up macros in the module scope.

This syntax has the potential to unlock so many possibilities that I think it's well worth reconsidering such a proposal. Even without 'type aware' macros, it could produce interesting new designs.

### Conclusion

This post is long enough as it is. Thanks for reading!
