---
layout: post
title: "Trait objects and helper methods"
categories: rust
---

There is a pattern in C++ and other languages where you create an interface with some methods to be overridden and some final helper methods (typically inlined) for convenience.

I'll use the following example ([cpp.sh](http://cpp.sh/4gtzn)):

```c++
#include <string>
#include <typeinfo>
#include <cstring>
#include <cassert>

class IQuery {
public:
	virtual const void* Query(const std::type_info& ty, const char* name) const = 0;

	template<typename T>
	inline const T* Get(const char* name) const {
		return (const T*)this->Query(typeid(T), name);
	}
};

class Parameters : public IQuery {
public:
	Parameters(int i, float f, std::string s) : i(i), f(f), s(std::move(s)) {}

	virtual const void* Query(const std::type_info& ty, const char* name) const {
		if (!strcmp(name, "i")) {
			return ty == typeid(int) ? &this->i : nullptr;
		}
		else if (!strcmp(name, "f")) {
			return ty == typeid(float) ? &this->f : nullptr;
		}
		else if (!strcmp(name, "s")) {
			return ty == typeid(std::string) ? &this->s : nullptr;
		}
		return nullptr;
	}

private:
	int i;
	float f;
	std::string s;
};

int main()
{
	Parameters p(42, 3.1415927, "Hello world!");
	assert(*p.Get<int>("i") == 42);
	assert(*p.Get<float>("f") == 3.1415927);
	assert(*p.Get<std::string>("s") == "Hello world!");

	IQuery* q = &p;
	assert(*q->Get<int>("i") == 42);
	assert(*q->Get<float>("f") == 3.1415927);
	assert(*q->Get<std::string>("s") == "Hello world!");
}
```

The example demonstrates the pattern of creating an implementable interface with convenience methods to handle the ugly casting.

A first attempt to convert this to Rust: [playground](https://play.rust-lang.org/?gist=c0cd054c5df1595447cb99573e7328ff&version=stable&backtrace=0)

```rust
use ::std::any::{Any};

trait IQuery {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any>;

	fn get<'s, T: 'static>(&'s self, name: &str) -> Option<&'s T> {
		self.query(name).and_then(|val| val.downcast_ref())
	}
}

struct Parameters {
	i: i32,
	f: f64,
	s: String,
}
impl IQuery for Parameters {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any> {
		match name {
			"i" => Some(&self.i),
			"f" => Some(&self.f),
			"s" => Some(&self.s),
			_ => None,
		}
	}
}

fn main() {
	let p = Parameters {
		i: 42,
		f: 3.1415927,
		s: String::from("Hello world!"),
	};
	assert_eq!(p.get("i"), Some(&42));
	assert_eq!(p.get("f"), Some(&3.1415927));
	assert_eq!(p.get("s"), Some(&String::from("Hello world!")));

	let q = &p as &IQuery;
	assert_eq!(q.get("i"), Some(&42));
	assert_eq!(q.get("f"), Some(&3.141592));
	assert_eq!(q.get("s"), Some(&String::from("Hello world!")));
}
```

This works but has an unfortunate limitation: the trait isn't object-safe!

```
rustc 1.17.0 (56124baa9 2017-04-24)
error[E0038]: the trait `IQuery` cannot be made into an object
  --> <anon>:37:16
   |
37 | 	let q = &p as &IQuery;
   | 	              ^^^^^^^ the trait `IQuery` cannot be made into an object
   |
   = note: method `get` has generic type parameters
```

Rust offers a canonical solution [[1]]: mark the offending methods with `where Self: Sized` bounds: [playground](https://play.rust-lang.org/?gist=3e27aa2d1f1f4d37a93f5768ed907d77&version=stable&backtrace=0)

[1]: https://huonw.github.io/blog/2015/05/where-self-meets-sized-revisiting-object-safety/

```rust
trait IQuery {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any>;

	fn get<'s, T: 'static>(&'s self, name: &str) -> Option<&'s T>
		where Self: Sized
	{
		self.query(name).and_then(|val| val.downcast_ref())
	}
}
```

Unfortunately this won't get us the whole way there, see we've filtered out the convenience methods such that they're completely unavailable on trait objects!

```
rustc 1.17.0 (56124baa9 2017-04-24)
error[E0277]: the trait bound `IQuery: std::marker::Sized` is not satisfied
  --> <anon>:40:15
   |
40 | 	assert_eq!(q.get("i"), Some(&42));
   | 	             ^^^ the trait `std::marker::Sized` is not implemented for `IQuery`
   |
   = note: `IQuery` does not have a constant size known at compile-time
```

This somewhat confusing error message is trying to say you cannot call `get` on a `&IQuery` which requires that `Self` is `Sized`, since `Self` is a 'bare' trait (not behind a reference) it is not `Sized`.

To me it was not entirely clear how to proceed from here. You can implement methods on the trait object type itself through the syntax `impl<'a> IQuery + 'a { }` but now you've just inverted the problem: those methods are only available on trait objects!

Attempting to define them both in the trait and on the trait object makes the compiler yell loudly at you: [playground](https://play.rust-lang.org/?gist=465792c877172dd2f65b287ae6c7530f&version=stable&backtrace=0)

```
rustc 1.17.0 (56124baa9 2017-04-24)
error[E0034]: multiple applicable items in scope
  --> <anon>:45:15
   |
45 | 	assert_eq!(q.get("i"), Some(&42));
   | 	             ^^^ multiple `get` found
   |
note: candidate #1 is defined in an impl for the type `IQuery`
  --> <anon>:13:2
   |
13 |   	fn get<'s, T: 'static>(&'s self, name: &str) -> Option<&'s T> {
   |  __^ starting here...
14 | | 		self.query(name).and_then(|val| val.downcast_ref())
15 | | 	}
   | |__^ ...ending here
note: candidate #2 is defined in the trait `IQuery`
  --> <anon>:6:2
   |
6  |   	fn get<'s, T: 'static>(&'s self, name: &str) -> Option<&'s T>
   |  __^ starting here...
7  | | 		where Self: Sized
8  | | 	{
9  | | 		self.query(name).and_then(|val| val.downcast_ref())
10 | | 	}
   | |__^ ...ending here
   = help: to disambiguate the method call, write `IQuery::get(&q, "i")` instead
```

I'm unsure how to describe the solution so I'll just let the code speak for itself: [playground](https://play.rust-lang.org/?gist=70f8a1cb7e5488a7d3757f5eb5b505c4&version=stable&backtrace=0)

```rust
impl<'a, T: 'a + ?Sized + IQuery> IQuery for &'a T {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any> {
		IQuery::query(*self, name)
	}
}
```

We're almost there. The above block implements the `IQuery` interface for all references to `T` implementing `IQuery`. The `?Sized` bound means this also applies to trait objects!

```
rustc 1.17.0 (56124baa9 2017-04-24)
error[E0277]: the trait bound `IQuery: std::marker::Sized` is not satisfied
  --> <anon>:45:15
   |
45 | 	assert_eq!(q.get("i"), Some(&42));
   | 	             ^^^ the trait `std::marker::Sized` is not implemented for `IQuery`
   |
   = note: `IQuery` does not have a constant size known at compile-time
```

Drat! What's going on here is that Rust is selecting the wrong `IQuery` implementation. You can fix this by calling `(&q).get`...

Another solution is to let your `Self: Sized` bounded methods take `self` (in which case the `Self` type will be resolved to `&IQuery` when going through the blanket impl). However since there's a lifetime involved this isn't possible here...

And that's where I'm stuck for now. If anyone has an idea to get past this last hurdle, do let me know!

The final working code, with workaround: [playground](https://play.rust-lang.org/?gist=4fa46a83576f35498782634a3dbefded&version=stable&backtrace=0)

```rust
use ::std::any::{Any};

trait IQuery {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any>;

	fn get<'s, T: 'static>(&'s self, name: &str) -> Option<&'s T>
		where Self: Sized
	{
		self.query(name).and_then(|val| val.downcast_ref())
	}
}
impl<'a, T: 'a + ?Sized + IQuery> IQuery for &'a T {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any> {
		IQuery::query(*self, name)
	}
}

struct Parameters {
	i: i32,
	f: f64,
	s: String,
}
impl IQuery for Parameters {
	fn query<'s>(&'s self, name: &str) -> Option<&'s Any> {
		match name {
			"i" => Some(&self.i),
			"f" => Some(&self.f),
			"s" => Some(&self.s),
			_ => None,
		}
	}
}

fn main() {
	let p = Parameters {
		i: 42,
		f: 3.1415927,
		s: String::from("Hello world!"),
	};
	assert_eq!(p.get("i"), Some(&42));
	assert_eq!(p.get("f"), Some(&3.1415927));
	assert_eq!(p.get("s"), Some(&String::from("Hello world!")));

	let q = &p as &IQuery;
	assert_eq!((&q).get("i"), Some(&42));
	assert_eq!((&q).get("f"), Some(&3.1415927));
	assert_eq!((&q).get("s"), Some(&String::from("Hello world!")));
}
```
