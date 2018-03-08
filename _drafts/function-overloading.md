---
layout: post
title: "Function Overloading"
categories: rust
---

Rust has no traditional overloading, you cannot define two methods with the same name [(1)](#example-1) or call a method with if multiple applicable items with that name are in scope [(2)](#example-2).

The format is focussed around short paragraphs and examples demonstrating the subject. Each example has a link to the playground so you can play along. Needless to say the examples are toys used to demonstrate an idea, don't take them too literal.

Out of scope is parameter arity-based overloading where an overloaded function can take different number of arguments. They can be simulated with tuples, although the extra set of parentheses can be visually noisy.

#### Example (1):

[Playground](https://play.rust-lang.org/?gist=e4487869a425db934db5cbfe227a63c8&version=stable)

```rust
struct Foo;
impl Foo {
	fn foo(_: i32) {}
	fn foo(_: &str) {}
}
```

The compiler reports:

```
error[E0201]: duplicate definitions with name `foo`
```

#### Example (2):

[Playground](https://play.rust-lang.org/?gist=13019a9b093a002ae0b6a15b81be99b2&version=stable)

```rust
// Multiple traits to provide the different overloads
trait Foo_A { fn foo(_: i32); }
trait Foo_B { fn foo(_: &str); }

struct Foo;
impl Foo_A for Foo { fn foo(_: i32) {} }
impl Foo_B for Foo { fn foo(_: &str) {} }

fn main() {
	Foo::foo("hello");
}
```

The compiler reports:

```
error[E0034]: multiple applicable items in scope
```

Instead Rust uses static polymorphism with generics to allow a method to take different kinds of arguments who are unified by having implemented a shared trait [(3)](#example-3).

In fact, it is fairly common to write functions generic over the `AsRef`, `Into`, `From` or `Borrow` traits.

#### Example (3):

[Playground](https://play.rust-lang.org/?gist=538781b908642b4d578778b7ab64432f&version=stable)

```rust
// Polymorphic function can take many shapes
// At the call site this is looks very much like traditional overloading
fn print_bytes<T: ?Sized + AsRef<[u8]>>(bytes: &T) {
	println!("{:?}", bytes.as_ref());
}

fn main() {
	// Looks like overloading to me
	print_bytes("hello world");
	print_bytes(&[12, 42, 39, 15, 91]);
}
```

Overloading is best demonstrated with `ToString::to_string` method, it accepts a whole host of types. At the call site this looks very much like function overloading [(4)](#example-4).

#### Example (4):

[Playground](https://play.rust-lang.org/?gist=566e593421a8bc24d5f0bc4ace7eb9ba&version=stable)

```rust
// This function is very promiscuous
// Look at all the different kinds of parameter types it accepts!
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

This kind of overloading makes your API more accessible for your users. They won't be burdened by ensuring the arguments are converted to the correct type your API expects, you'll do it for them. The result is an API which is much nicer to use, especially in simple cases [(5)](#example-5).

#### Example (5):

[Playground](https://play.rust-lang.org/?gist=55382f346d8982b2242aaf7ec7860a5e&version=stable)

```rust
use std::{mem, ptr};
use std::marker::PhantomData;

// Marker defines that any byte pattern is valid for this type
// Allowing any byte pattern to be safely transmuted to this type
unsafe trait Pod: 'static {}
unsafe impl Pod for i32 {}
unsafe impl Pod for f32 {}
// etc...

// Let's say you've created an API around a custom pointer datastructure
// Which really just wraps a usize and tags it with a type
// Users of your API now must always convert to your custom pointer type
// Even for simple examples where a static offset would do
struct Pointer<T> {
	value: usize,
	_phantom: PhantomData<*mut T>,
}
impl<T> From<usize> for Pointer<T> {
	fn from(value: usize) -> Pointer<T> {
		Pointer { value, _phantom: PhantomData }
	}
}
impl<T> Into<usize> for Pointer<T> {
	fn into(self) -> usize {
		self.value
	}
}

// However through usage of the `Into` trait your users don't need any boilerplate
// This can be called with raw usize values as well as Pointer<T> types
// The function reads a POD `T` from an arbitrary offset in the byte buffer
fn read_from<T, P>(buffer: &[u8], ptr: P) -> T
	where T: Pod,
	      P: Into<Pointer<T>>,
{
	let ptr: usize = ptr.into().into();
	let ptr = buffer[ptr..ptr + mem::size_of::<T>()].as_ptr();
	unsafe {
		ptr::read_unaligned(ptr as *const T)
	}
}

fn main() {
	let bytes = [0, 0, 0b00101001, 0b00100011, 0, 0, 42, 0b11011011, 0b00001111, 0b01001001, 0b01000000, 255];
	let int: i32 = read_from(&bytes, 2);
	let float: f32 = read_from(&bytes, 7);
	println!("int: {}", int);
	println!("float: {}", float);
}
```

This approach has a major advantage over traditional overloading as it allows the API user to extend the accepted types to custom types defined by your users.

Beware of '_generics code bloat_' when overusing this technique. If you have a generic function with significant amount of non trivial code, a new copy of that function specialized for every unique set of type arguments is created. Even if all you do is convert the input arguments at the start of said function.

Luckily there's a simple solution to this problem: implement a private function without generics accepting the real types you want to work with then have your public generic method perform the type conversions and dispatch to your private implementation [(6)](#example-6).

#### Example (6):

[Playground](https://play.rust-lang.org/?gist=91dd76eca898115a3bea4dc39d1695c9&version=stable)

```rust
mod stats {
	// Implement the public facade, converting the arguments for the user's convenience
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
	// Despite being called with two different types (&[f64] and &Vec<f64>)
	// The meat of the function is only implemented once, saving on binary size
	let a = stddev(&[600.0, 470.0, 170.0, 430.0, 300.0]);
	let b = stddev(&vec![600.0, 470.0, 170.0, 430.0, 300.0]);

	assert_eq!(a, b);
}
```

Not all overloading falls into this category of convenient argument conversion. Sometimes you really want to handle different types in a unique way.
For these occasions you can define your own trait implemented for the types your API supports [(7)](#example-7).

#### Example (7):

[Playground](https://play.rust-lang.org/?gist=dcff3002e5bef6706085dd622829566f&version=stable)

```rust
// The trait which defines the custom interface
// Notice how the subject and argument are 'swapped'
trait CustomFoo {
	fn custom_foo(self, this: &Foo) -> Self;
}
// Explicit custom implementation for every supported type
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

However this technique can only be stretched so far. In the most extreme case the previous example will dispatch its entire logic into the trait implementation which looks very odd (the `self` and arguments are swapped). Furthermore this won't extend where you have more than one overloaded parameter.

An example of this technique can be found in the standard library in the [`Pattern` trait](https://doc.rust-lang.org/std/str/pattern/trait.Pattern.html) used by various string matching [functions like `str::find`](https://doc.rust-lang.org/std/primitive.str.html#method.find).

The next example demonstrates how to use traits in a way to write overloads in an intuitive way (minus the trait boilerplate, that is). The trait used still looks nice enough that you wouldn't mind to expose your users to it however you still have the option of providing an inherent method for your API [(8)](#example-8).

#### Example (8):

[Playground](https://play.rust-lang.org/?gist=ba7c0e9e321f48c5d53961cbc6b81a2f&version=stable)

```rust
// Trait defines the method to overload
trait OverloadedFoo<T, U> {
	fn overloaded_foo(&self, tee: T, yu: U);
}

// Struct to use with the overload
struct Foo;
impl Foo {
	// Convenience helper to hide the trait boilerplate from the user
	fn foo<T, U>(&self, tee: T, yu: U) where Self: OverloadedFoo<T, U> {
		self.overloaded_foo(tee, yu)
	}
}

// Implement the overloads
impl OverloadedFoo<i32, f32> for Foo {
	fn overloaded_foo(&self, tee: i32, yu: f32) {
		println!("foo<i32, f32>(tee: {}, yu: {})", tee, yu);
	}
}
impl<'a> OverloadedFoo<&'a str, char> for Foo {
	fn overloaded_foo(&self, tee: &'a str, yu: char) {
		println!("foo<&str, char>(tee: {}, yu: {})", tee, yu);
	}
}

fn main() {
	Foo.foo(42, 3.14159);
	Foo.foo("hello", '😄');
}
```

There is one caveat, one which makes the APIs using this technique less usable. Autoderef is Rust compiler magic that will coerce a reference to your type to a reference of any type to which your type derefences to (that's a mouthful). That is to say, given a `&Vec<_>` or `&String` the compiler will derefence them to `&[_]` or `&str` as needed.

However using traits for overloading and you will lose this highly convenient magic, the compiler helpfully prints all the available implementations which in this case are the available overloads [(9)](#example-9).

#### Example (9):

[Playground](https://play.rust-lang.org/?gist=091ef3584b737bfe7806154497ed9bce&version=stable)

```rust
// From Example (8)
trait OverloadedFoo<T, U> {
	fn overloaded_foo(&self, tee: T, yu: U);
}

struct Foo;
impl Foo {
	fn foo<T, U>(&self, tee: T, yu: U) where Self: OverloadedFoo<T, U> {
		self.overloaded_foo(tee, yu)
	}
}

impl OverloadedFoo<i32, f32> for Foo {
	fn overloaded_foo(&self, tee: i32, yu: f32) {
		println!("foo<i32, f32>(tee: {}, yu: {})", tee, yu);
	}
}
impl<'a> OverloadedFoo<&'a str, char> for Foo {
	fn overloaded_foo(&self, tee: &'a str, yu: char) {
		println!("foo<&str, char>(tee: {}, yu: {})", tee, yu);
	}
}

// Demonstrate lack of autoderef
fn main() {
	let string = String::from("hello");
	// Incorrect usage, no autoderef
	Foo.foo(&string, '😒');
	// Correct usage through the &* dance
	Foo.foo(&*string, '😔');
}
```

The compiler reports:

```
error[E0277]: the trait bound `Foo: OverloadedFoo<&std::string::String, _>` is not satisfied
  --> src/main.rs:25:6
   |
25 |     Foo.foo(&string, '😒');
   |         ^^^ the trait `OverloadedFoo<&std::string::String, _>` is not implemented for `Foo`
   |
   = help: the following implementations were found:
             <Foo as OverloadedFoo<i32, f32>>
             <Foo as OverloadedFoo<&'a str, char>>
```

You may try to add blanket impls to fix these cases but you may run into trait coherence issues [(10)](#example-10).

#### Example (10):

[Playground](https://play.rust-lang.org/?gist=3c85d99b44d4560a230bfa9b5bbd9027&version=stable)

```rust
trait CustomFoo<T> {
	fn custom_foo(&self, arg: T) -> i32;
}
struct Foo {
	case: bool,
}
impl Foo {
	fn foo<T>(&self, arg: T) -> i32 where Self: CustomFoo<T> {
		self.custom_foo(arg)
	}
}
impl CustomFoo<i32> for Foo {
	fn custom_foo(&self, arg: i32) -> i32 {
		if self.case { 42 } else { arg }
	}
}

// Error (see below)
// Of course std won't ever add AsRef<str> for i32.
// Rust does not allow to communicate this intent so trait coherence has to be conservative.
impl<T: AsRef<str>> CustomFoo<T> for Foo {
	fn custom_foo(&self, arg: T) -> i32 {
		arg.as_ref().len() as i32
	}
}

// This works as long as no other overload takes a reference.
// Because the compiler won't ever confuse `i32` for `&'a _`.
impl<'a, T: AsRef<str> + ?Sized> CustomFoo<&'a T> for Foo {
	fn custom_foo(&self, arg: &'a T) -> i32 {
		arg.as_ref().len() as i32
	}
}

fn main() {}
```

The compiler reports:

```
error[E0119]: conflicting implementations of trait `CustomFoo<i32>` for type `Foo`:
  --> src/main.rs:19:1
   |
12 | impl CustomFoo<i32> for Foo {
   | --------------------------- first implementation here
...
19 | impl<T: AsRef<str>> CustomFoo<T> for Foo {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Foo`
   |
   = note: upstream crates may add new impl of trait `std::convert::AsRef<str>` for type `i32` in future versions
```

That's it.

As always, which technique you choose to achieve overloading depends on your specific needs. My goal with this blog post is to lay out the different overloading techniques and their limitations so you can make an informed decision for your codebase.

I haven't experimented yet with specialization and how it will affect these techniques. My impression is that specialization seeks to solve an orthogonal problem and nothing will prevent specialization combined with overloading as described here.

Feel free to experiment!

If you're still reading this I hope you've enjoyed this exploration of function overloading in Rust and its limitations.
