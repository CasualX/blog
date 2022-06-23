---
layout: post
title: "Reinventing Rust formatting syntax"
categories: rust
---

In this post I'm announcing my crate `fmtools`, some links if you prefer those: [github](https://github.com/CasualX/fmtools), [crates.io](https://crates.io/crates/fmtools), [docs.rs](https://docs.rs/fmtools).

Formatting large blocks of text interspersed with formatted values has always been really awkward in Rust (and other programming languages, really):

```rust
let values = [1, 2, 0, 4, 5];
let separator = "------------";

let s = format!("\
	First value: {}\n\
	Second value: {}\n\
	Third value: {}\n\
	{}\n\
	Fourth value: {}\n\
	Fifth value: {}\n",
	values[0], values[1], values[2],
	separator,
	values[3], values[4]);
```

Maintaining such code is hard enough, it becomes even more tricky if you only want to emit part of the formatting on some condition. Eg. only display the third line if the third value is non-zero.

There have been various attempts at making this use case work better in Rust but I haven't been following any of the discussion and set out to do my own thing. I heard you can now use implicit identifiers in format arguments? Rookie stuff.

While formatting expressions in format strings has been implemented in other languages they tend to come with various restrictions:

* Python requires an alternative syntax for the string literal inside an f-string: [link](https://stackoverflow.com/questions/71627968/why-can-i-not-embed-a-string-literal-inside-an-f-string).
* Requires escaping the braces `{%raw%}{{}}{%endraw%}` if you want to write them.
* Rust's formatting syntax is _still_ extremely limited.
* No language I know allows custom control flow to conditionally emit a piece of the formatting string.
* Probably more that I cannot think of right now...

Here's an example of how control flow may come up expressed with consecutive `print!`:

```rust
let power = 0.5;

print!("At ");
if power >= 1.0 {
	print!("full");
}
else {
	print!("{:.0}%", power * 100.0);
}
print!(" power");
```

How would _you_ express this simple task in an elegant format string?

I present to you my solution to these problems for both examples:

```rust
let values = [1, 2, 0, 4, 5];
let separator = "------------";

let s = fmtools::format! {
	"First value: "{values[0]}"\n"
	"Second value: "{values[1]}"\n"
	if values[2] != 0 {
		"Third value: "{values[2]}"\n"
	}
	{separator}"\n"
	"Fourth value: "{values[3]}"\n"
	"Fifth value: "{values[4]}"\n"
};
```

```rust
let power = 0.5;

fmtools::println!("At "
	if power >= 1.0 { "full" }
	else { {power * 100.0:.0}"%" }
	" power");
```

Implemented in a single, no_std compatible, zero dependencies crate: [github](https://github.com/CasualX/fmtools), [crates.io](https://crates.io/crates/fmtools), [docs.rs](https://docs.rs/fmtools).

Features:

* Allows arbitrary expressions inside the formatting braces.

* Generates optimized Rust code at compiletime.

  All the parsing is done at compiletime by the macro and the control flow is lowered to native Rust code, exactly as written.

* Supports rust-analyzer autocomplete, refactoring and more!

  This is more because rust-analyzer is awesome! It's successful at looking back from the generated code and where an expression is expected it's able to provide completion and renaming features. Read more about it [here](https://rust-analyzer.github.io/blog/2021/11/21/ides-and-macros.html)!

  Some special care was taking when writing stuff like `if` expressions before you've written the `{}` that the macro expands in a way that rust-analyzer can follow along and provide IDE features.

  ```rust
  let cond = true;
  fmtools::format!(if cond.) // autocomplete works, but does not compile!
  ```

* Supports Rust's standard formatting specifiers.

  When formatting specifiers are encountered the macro falls back to `format_args!` for the actual implementation giving access to all its features.

* Single package, no proc-macro, no_std compatible, no extra dependencies.

  Proc-macros complicate things, require extra build step and trust that the proc-macro won't do something it shouldn't.

  The package drags in no extra dependencies, it's fully self contained!

* Create `let` bindings to store temporary values for formatting.

  Useful for more complex templates and allows the value to be printed multiple times while only being evaluated once.

  Rust's standard formatting equivalent is [positional parameters](https://doc.rust-lang.org/std/fmt/index.html#positional-parameters), here's what I have to say about that:

  ```rust
  fmtools::format!(
  	let (one, two) = (1, 2);
  	{two}" "{one}" "{one}" "{two}); // => "2 1 1 2"
  ```

  Okay this is pretty contrived and looks worse here, but in larger format strings this works out well!

* Control flow allows conditional and repeated formatting.

  As demonstrated earlier, control flow in the form of `if`, `if let`, `else`, `match`, `for` make it really nice to format larger and more complex strings while keeping your code maintainable.

* Capture variables by value or by reference.

  Values of `format_args!` [cannot be returned](https://users.rust-lang.org/t/why-cant-i-return-format-args-from-a-closure/39281) from closures or functions. Rust's standard formatting simply does not support this. `fmtools` is built on closures which allows variables to be captured by value instead of by reference.

  The `fmtools::fmt!` macro enables this behavior by starting the formatting with `move` just like closures!

  ```rust
  fn main() {
  	let values = [1, 2, 3, 4, 5];

  	// Without `move` this would not pass the borrow checker
  	let formatted = fmtools::join(", ",
  		values.iter().map(|v| fmtools::fmt!(move "x"{v})));

  	println!("{}", formatted); // => "x1, x2, x3, x4, x5"
  }
  ```

* Escape hatch to inject custom formatting code.

  If all else fails use closure syntax to access the underlying `&mut std::fmt::Formatter` and inject custom formatting code:

  ```rust
  fmtools::format! {
  	"Now entering ["
  	|f| f.write_str("custom formatting")?;
  	"]"
  }
  ```
