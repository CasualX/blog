---
layout: post
title: "Function Overloading"
categories: rust
---

The format is focussed around short paragraphs and examples demonstrating the subject. Each example has a link to the playground so you can play along. Needless to say the examples are toys used to demonstrate an idea, don't take them too literal.

## First attempt

Rust has no traditional overloading, you cannot define two methods with the same name. The compiler will complain that you have a duplicate definition regardless of the different argument types.

Trying to be clever with traits also doesn't work:

```rust
trait Foo_A { fn foo(_: i32); }
trait Foo_B { fn foo(_: &str); }

struct Foo;
impl Foo_A for Foo { fn foo(_: i32) {} }
impl Foo_B for Foo { fn foo(_: &str) {} }

fn main() {
	//Foo::foo("hello"); // No automatic overload selection
	<Foo as Foo_B>::foo("hello"); // Requires explicit cast
}
```

[Playground](https://play.rust-lang.org/?gist=13019a9b093a002ae0b6a15b81be99b2&version=stable)

The compiler reports that there are multiple applicable items in scope. Your invocation is ambiguous and Rust doesn't try to figure out which one to call based on argument types.

## Static polymorphism

Instead Rust uses static polymorphism with generics to allow a method to take different types of arguments.

The generic parameter is constrained by a trait meaning that the function will only accept types which implement the trait. The trait limits the things you can do with the argument.

They can be very simple things like `AsRef` to make using your API nice for your users:

```rust
fn print_bytes<T: AsRef<[u8]>>(bytes: T) {
	println!("{:?}", bytes.as_ref());
}

fn main() {
	print_bytes("hello world");
	print_bytes(&[12, 42, 39, 15, 91]);
}
```

[Playground](https://play.rust-lang.org/?gist=538781b908642b4d578778b7ab64432f&version=stable)

At the call site, it certainly looks like overloading. Of course traditional overloading has a lot more flexibility in the implementation of the overloaded functions.

Perhaps the best demonstration of this is [the `ToString` trait](https://doc.rust-lang.org/std/string/trait.ToString.html), it accepts a whole host of types:

```rust
fn print_str<T: ToString>(value: T) {
	let s = value.to_string();
	println!("{}", s);
}
fn main() {
	print_str(42);
	print_str(3.141593);
	print_str("hello");
	print_str(true);
	print_str('😎');
}
```

[Playground](https://play.rust-lang.org/?gist=566e593421a8bc24d5f0bc4ace7eb9ba&version=stable)


This kind of overloading makes your API more accessible for your users. They won't be burdened by ensuring the arguments are converted to the correct type your API expects, you'll do it for them. The result is an API which is much nicer to use.

This approach has a major advantage over traditional overloading as it allows the API user to extend the accepted types to custom types defined by your users.

As an aside, you are unable to use traits like this to overload a function with a different number of arguments (because the trait provides the signature).

[`ToSocketAddrs`](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html#implementors)

Out of scope is parameter arity-based overloading where an overloaded function can take different number of arguments. They can be simulated with tuples, although the extra set of parentheses can be visually noisy.

## Intermezzo: Generics code bloat

Beware of '_generics code bloat_' when using generics. If you have a generic function with significant amount of non trivial code, a new copy of that function specialized for every unique set of type arguments is created. Even if all you do is convert the input arguments at the start of the function.

Luckily there's a simple solution to this problem: implement a private function without generics accepting the real types you want to work with then have your public generic method perform the type conversions and dispatch to your private implementation:

```rust
mod stats {
	pub fn stddev<T: ?Sized + AsRef<[f64]>>(values: &T) -> f64 {
		stddev_impl(values.as_ref())
	}
	// Private non-generic implementation
	fn stddev_impl(values: &[f64]) -> f64 {
		let len = values.len() as f64;
		let sum: f64 = values.iter().cloned().sum();
		let mean = sum / len;
		let var = values.iter().fold(0f64, |acc, &x| acc + (x - mean) * (x - mean)) / len;
		var.sqrt()
	}
}
pub use stats::stddev;

fn main() {
	let a = stddev(&[600.0, 470.0, 170.0, 430.0, 300.0]);
	let b = stddev(&vec![600.0, 470.0, 170.0, 430.0, 300.0]);

	assert_eq!(a, b);
}
```

[Playground](https://play.rust-lang.org/?gist=91dd76eca898115a3bea4dc39d1695c9&version=stable)

Despite being called with two different types (`&[f64]` and `&Vec<f64>`) the meat of the function is only implemented once, saving on binary size.

## Stretching to the limit

Not all overloading falls into this category of convenient argument conversion. Sometimes you really want to handle different types in a unique non-uniform way.
For these occasions you can define your own trait implemented for the types your API supports.

Implement all the custom details for your function in that trait:

```rust
trait CustomFoo {
	fn custom_foo(self, this: &Foo) -> Self;
}

impl CustomFoo for i32 {
	fn custom_foo(self, this: &Foo) -> i32 {
		if this.case { 42 } else { self }
	}
}
impl CustomFoo for f64 {
	fn custom_foo(self, this: &Foo) -> f64 {
		if this.case { 3.14159265 } else { self }
	}
}
impl<'a> CustomFoo for &'a str {
	fn custom_foo(self, this: &Foo) -> &'a str {
		if this.case { "foo" } else { self }
	}
}

// Silly example to have *something*
struct Foo {
	case: bool,
}
impl Foo {
	// Make the function generic over the argument
	// Straight up dispatch into the trait for the custom implementation
	fn foo<T: CustomFoo>(&self, arg: T) -> T {
		arg.custom_foo(self)
	}
}

fn main() {
	let foo = Foo { case: false };

	println!("{}", foo.foo(13));
	println!("{}", foo.foo(2.718281828));
	println!("{}", foo.foo("baz"));
}
```

[Playground](https://play.rust-lang.org/?gist=dcff3002e5bef6706085dd622829566f&version=stable)

This makes the trait very awkward as the `self` and argument are swapped.

An example of this technique can be found in the standard library in the [`Pattern` trait](https://doc.rust-lang.org/std/str/pattern/trait.Pattern.html) used by various string matching [functions like `str::find`](https://doc.rust-lang.org/std/primitive.str.html#method.find).

## Have your cake and eat it too

There is a better way, Rust's trait constraints with where clauses are incredibly powerful:

```rust
trait OverloadedFoo<T, U> {
	fn overloaded_foo(&self, tee: T, yu: U);
}
```

Define the trait for the method you would like to overload, with generic parameters for all the parameters you'd like to be able to change through overloading.

```rust
struct Foo;
impl Foo {
	fn foo<T, U>(&self, tee: T, yu: U) where Self: OverloadedFoo<T, U> {
		self.overloaded_foo(tee, yu)
	}
}
```

When implementing the method, simply constrain `Self` to implement the trait and any generic parameters your trait needs. This is enough for Rust to figure everything out.

```rust
impl OverloadedFoo<i32, f32> for Foo {
	fn overloaded_foo(&self, tee: i32, yu: f32) {
		println!("foo<i32, f32>(tee: {}, yu: {})", tee, yu);
	}
}
impl<'a, S: AsRef<str> + ?Sized> OverloadedFoo<&'a S, char> for Foo {
	fn overloaded_foo(&self, tee: &'a S, yu: char) {
		println!("foo<&str, char>(tee: {}, yu: {})", tee.as_ref(), yu);
	}
}
```

Then implement the trait for all the types you wish to provide an overload for.

These can be blanket impls. Although be careful to not run into trait coherence errors. The compiler's error messages are extremely helpful here.

```rust
fn main() {
	Foo.foo(42, 3.14159);
	Foo.foo("hello", '😄');
}
```

[Playground](https://play.rust-lang.org/?gist=ba7c0e9e321f48c5d53961cbc6b81a2f&version=stable)

That's it, play with it on the playground. See what happens if you provide argument types for which there is no overload.

## Final notes

As always, which technique you choose to achieve overloading depends on your specific needs. My goal with this blog post is to lay out the different overloading techniques and their limitations so you can make an informed decision for your codebase.

I haven't experimented yet with specialization and how it will affect these techniques. My impression is that specialization seeks to solve an orthogonal problem and nothing will prevent specialization combined with overloading as described here.

Feel free to experiment!

If you're still reading this I hope you've enjoyed this exploration of function overloading in Rust and its limitations.
