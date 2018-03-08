---
layout: post
title: "Function Overloading"
categories: rust
---

Rust has no traditional overloading, you cannot define two methods with the same name (1) or call a method with if multiple applicable items with that name are in scope (2).

#### Example (1):

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

```rust
trait Foo_A {
	fn foo(_: i32);
}
trait Foo_B {
	fn foo(_: &str);
}

struct Foo;
impl Foo_A for Foo {
	fn foo(_: i32) {}
}
impl Foo_B for Foo {
	fn foo(_: &str) {}
}

fn main() {
	Foo::foo("hello");
}
```

The compiler reports:

```
error[E0034]: multiple applicable items in scope
```

Instead Rust uses static polymorphism with generics to allow a method to take different kinds of arguments who are unified by having implemented a shared trait (3).

In fact, it is fairly common to write functions generic over the `AsRef`, `Into`, `From` or `Borrow` traits.

#### Example (3):

```rust
fn print_bytes<T: ?Sized + AsRef<[u8]>>(bytes: &T) {
	println!("{:?}", bytes.as_ref());
}

fn main() {
	// Looks like overloading to me :)
	print_bytes("hello world");
	print_bytes(&[12, 42, 39, 15, 91]);
}
```

Overloading is best demonstrated with `ToString::to_string` method, it accepts a whole host of types. At the call site this looks very much like function overloading (4).

#### Example (4):

```rust
fn print_str<T: ToString>(value: T) {
	let s = value.to_string();
	println!("{}", s);
}
fn main() {
	print_str(12);
	print_str(3.141593);
	print_str("hello");
	print_str(true);
	print_str('😎');
}
```

This kind of overloading allows you to accept parameter types which can all be converted to some underlying type which you really want to work on without burdening the user of your API to perform this conversion manually every time they call your API (5).

#### Example (5):

```rust
use std::{mem, ptr};
use std::marker::PhantomData;

// Marker defines that any byte pattern is valid for this type
// Allowing any byte pattern to be safely transmuted to this type
unsafe trait Pod: 'static {}
unsafe impl Pod for i32 {}
unsafe impl Pod for f32 {}
// etc...

// Custom typed, pointer data structure
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

// Read and transmute some bytes from the buffer with our fancy pointer wrapper
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

A major advantage over classical overloading is that this allows the API user to extend the accepted types to custom types defined by the user.

Beware of 'generics code bloat' when overusing this technique. A new instance of your generic function is created for every unique set of type arguments even if all you do is convert the argument types at the beginning of your large function.

Luckily there's a simple solution to this problem: implement a private function without generics accepting the raw types you want to work with then have your public generic method perform the type conversions and dispatch to your private implementation (6).

Example (6):

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
For these occasions you can define your own trait implemented for the types your API supports (7).

Example (7):

```rust
```

However this technique can only be stretched so far. In the most extreme case the previous example will dispatch its entire logic into the trait implementation which looks very odd (the `self` and arguments are swapped). Furthermore this won't extend where you have more than overloaded parameter.

The last example demonstrates how to use traits in a way to write overloads in an intuitive way (minus the boilerplate of traits, that is). By providing an inherent method the user of your API does not have to worry about how you've achieved overloading through traits (8).

Example (8):

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
// Demonstrate
fn main() {
	Foo.foo(42, 3.14159);
	Foo.foo("hello", '😄');
}
```

There is one caveat, one which makes the APIs using this technique less usable. Autoderef is Rust compiler magic that will coerce a reference to your type to a reference of any type to which your type derefences to (that's a mouthful). That is to say, given a `&Vec<_>` or `&String` the compiler will derefence them to `&[_]` or `&str` as needed.

However using traits for overloading and you will lose this highly convenient magic and the compiler will produce less than helpful error messages (9).

Example (9):

```rust
```

You may try to add blanket impls to fix these cases but very quickly you'll run into trait coherence issues (10).

Example (10):

```rust
```

If you're still reading this I hope you've enjoyed this exploration of function overloading in Rust and its limitations.
