---
layout: post
title: "Exploring Function Overloading"
categories: rust
---

Overloading is the ability to create multiple functions of the same name with different implementations.

Rust has no traditional overloading, you cannot define two methods with the same name. The compiler will complain that you have a duplicate definition regardless of the different argument types.

Trying to be clever with traits also doesn't work:

```rust
trait Foo_A { fn foo(_: i32); }
trait Foo_B { fn foo(_: &str); }

struct Foo;
impl Foo_A for Foo { fn foo(_: i32) {} }
impl Foo_B for Foo { fn foo(_: &str) {} }
```

Then try to call the function with a `&str` argument type:

```rust
fn main() {
	Foo::foo("hello");
}
```

This won't compile because the invocation is ambiguous and Rust doesn't try to figure out which one to call based on argument types. If we run this code, the compiler reports that there are multiple applicable items in scope.

Instead this example requires an explicit disambiguation:

```rust
fn main() {
	<Foo as Foo_B>::foo("hello");
}
```

[Playground](https://play.rust-lang.org/?gist=13019a9b093a002ae0b6a15b81be99b2&version=stable)

However, that defeats the point of overloading.

At the end of this blog post demonstrates Rust can get pretty close to traditional overloading through the use of its trait system and generics.

## Static polymorphism

Rust uses static polymorphism with generics to allow a method to take different types of arguments.

The generic parameter is constrained by a trait meaning that the function will only accept types which implement that trait. The trait limits what you can do with the argument.

They can be very simple things like `AsRef` to make your API more accepting like so:

```rust
fn print_bytes<T: AsRef<[u8]>>(bytes: T) {
	println!("{:?}", bytes.as_ref());
}
```

At the call site it certainly looks like overloading:

```rust
fn main() {
	print_bytes("hello world");
	print_bytes(&[12, 42, 39, 15, 91]);
}
```

[Playground](https://play.rust-lang.org/?gist=538781b908642b4d578778b7ab64432f&version=stable)

Perhaps the best demonstration of this is [the `ToString` trait](https://doc.rust-lang.org/std/string/trait.ToString.html) which accepts a whole host of types:

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

This kind of overloading makes your API more accessible for your users. They won't be burdened by ensuring the arguments are converted to the correct type your API expects, you'll do it for them. The result is an API which is more pleasant to use.

This approach has a major advantage over traditional overloading that by implementing the required traits makes your API accept the user's custom types.

Traditional overloading offers a lot more flexibility in the implementation and the number of arguments of the overloaded functions. That last point can be aleviated by using tuples as a stand-in for multiple arguments but it's not so pretty. An example of this can be found in [the `ToSocketAddrs` trait](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html#implementors) in the standard library.

## Intermezzo: Generics code bloat

Beware of '_generics code bloat_' when using generics. If you have a generic function with significant amount of non trivial code, a new copy of that function specialized for every unique set of type arguments is created. Even if all you do is convert the input arguments at the start of the function.

Luckily there's a simple solution to this problem: implement a private function without generics accepting the real types you want to work with then have your public generic method perform the type conversions and dispatch to your private implementation:

```rust
mod stats {
	pub fn stddev<T: ?Sized + AsRef<[f64]>>(values: &T) -> f64 {
		stddev_impl(values.as_ref())
	}
	fn stddev_impl(values: &[f64]) -> f64 {
		let len = values.len() as f64;
		let sum: f64 = values.iter().cloned().sum();
		let mean = sum / len;
		let var = values.iter().fold(0f64, |acc, &x| acc + (x - mean) * (x - mean)) / len;
		var.sqrt()
	}
}
pub use stats::stddev;
```

Despite being called with two different types (`&[f64]` and `&Vec<f64>`) the meat of the function is only implemented once, saving on binary size:

```rust
fn main() {
	let a = stddev(&[600.0, 470.0, 170.0, 430.0, 300.0]);
	let b = stddev(&vec![600.0, 470.0, 170.0, 430.0, 300.0]);

	assert_eq!(a, b);
}
```

[Playground](https://play.rust-lang.org/?gist=91dd76eca898115a3bea4dc39d1695c9&version=stable)

## Stretching to the limit

Not all overloading falls into this category of convenient argument conversion. Sometimes you really want to handle different types in a unique non-uniform way.
For these occasions you can define your own trait to implement the function's custom logic:

```rust
pub struct Foo(bool);

pub trait CustomFoo {
	fn custom_foo(self, this: &Foo);
}
```

This makes the trait very awkward as the `self` and arguments are swapped:

```rust
pub struct Foo(bool);

impl CustomFoo for i32 {
	fn custom_foo(self, this: &Foo) {
		println!("Foo({}) i32: {}", this.0, self);
	}
}
impl CustomFoo for char {
	fn custom_foo(self, this: &Foo) {
		println!("Foo({}) char: {}", this.0, self);
	}
}
impl<'a, S: AsRef<str> + ?sized> CustomFoo for &'a S {
	fn custom_foo(self, this: &Foo) {
		println!("Foo({}) str: {}", this.0, self.as_ref());
	}
}
```

The trait cannot be hidden as an implementation detail that isn't exposed to API users. If you try to make the trait private then the compiler will complain about 'private trait in public interface'.

Let's provide a wrapper for the trait so it doesn't have to be called through the argument type:

```rust
pub struct Foo(bool);

impl Foo {
	pub fn foo<T: CustomFoo>(&self, arg: T) -> T {
		arg.custom_foo(self)
	}
}

fn main() {
	Foo(false).foo(13);
	Foo(true).foo('😆'));
	Foo(true).foo("baz");
}
```

[Playground](https://play.rust-lang.org/?gist=dcff3002e5bef6706085dd622829566f&version=stable)

An example of this technique can be found in the standard library in [the `Pattern` trait](https://doc.rust-lang.org/std/str/pattern/trait.Pattern.html) used by various string matching [functions like `str::find`](https://doc.rust-lang.org/std/primitive.str.html#method.find).

Unlike you, the standard library has special powers to hide these traits while still allowing them to be used in its public interface through the `#[unstable]` attribute.

## Have your cake and eat it too

There is a better way, that gets us almost all the way to traditional overloading.

Define the trait for the method you would like to overload, with generic parameters for all the parameters you'd like to be able to change through overloading:

```rust
trait OverloadedFoo<T, U> {
	fn overloaded_foo(&self, tee: T, yu: U);
}
```

Rust's trait constraints with where clauses are incredibly powerful.

When implementing the method, simply constrain `Self` to implement the trait and any generic parameters your trait needs. This is enough for Rust to figure everything out:

```rust
struct Foo;
impl Foo {
	fn foo<T, U>(&self, tee: T, yu: U) where Self: OverloadedFoo<T, U> {
		self.overloaded_foo(tee, yu)
	}
}
```

Then implement the trait for all the types you wish to provide an overload for:

```rust
impl OverloadedFoo<i32, f32> for Foo {
	fn overloaded_foo(&self, tee: i32, yu: f32) {
		println!("foo<i32, f32>(tee: {}, yu: {})", tee, yu);
	}
}
```

These can be blanket impls. Although be careful to not run into trait coherence errors. The compiler's error messages are extremely helpful here.

```rust
impl<'a, S: AsRef<str> + ?Sized> OverloadedFoo<&'a S, char> for Foo {
	fn overloaded_foo(&self, tee: &'a S, yu: char) {
		println!("foo<&str, char>(tee: {}, yu: {})", tee.as_ref(), yu);
	}
}
```

That's it!

Try to uncomment last line and observe the helpful error message when the function is called with types the overload doesn't support:

```rust
fn main() {
	Foo.foo(42, 3.14159);
	Foo.foo("hello", '😄');
	// Foo.foo('😏', 13); // Overload not implemented
}
```

[Playground](https://play.rust-lang.org/?gist=ba7c0e9e321f48c5d53961cbc6b81a2f&version=stable)

## Final notes

As always, which technique you choose to achieve overloading depends on your specific needs. My goal with this blog post is to lay out the different overloading techniques and their limitations so you can make an informed decision for your codebase.

I haven't experimented yet with specialization and how it will affect these techniques. My impression is that specialization seeks to solve an orthogonal problem and nothing will prevent specialization combined with overloading as described here.

Feel free to experiment!

If you're still reading this I hope you've enjoyed this exploration of function overloading in Rust and its limitations.
