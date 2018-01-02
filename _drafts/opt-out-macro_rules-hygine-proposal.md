---
layout: post
title: "Opt-out macro_rules hygine proposal"
categories: rust
---

## Hygine

As we know, `macro_rules!` has hygine.

Any identifiers created within the macro will not collide with any arguments.

```rust
macro_rules! add_one {
	($expr:expr) => {
		let val = $expr;
		val + 1
	}
}

fn main() {
	let val = 21;
	let result = add_one!(val);
	assert_eq!(result, 22);
}
```

The local variable `val` in `main` will not wreck havoc with the temporary variable in `add_one!`.

## Limiting potential

It is useful to want to create new identifiers in the scope where the macro is invoked, hygine will prevent this from working.

In the following example we want to create a helper function implementing some common functionality and dispatching to the given function.

Sure this example can be trivially refactored, but that is beside the point.

```rust
macro_rules! create_fn {
	($ident:ident) => {
		// Somehow create a new identifier by taking the existing one and adding `_helper`.
		fn $ident _helper() {
			$ident()
		}
	}
}

fn foo() {}

// Create a function `foo_helper` which calls `foo`.
create_fn!(foo);

fn main() {}
```

The above won't compile of course.

At first you might find `concat_idents!`. But after reading the docs you'll be disappointed to read it cannot be used to create new identifiers.

The workaround is to pass all the identifiers to be created as an argument to the macro. That works, but isn't very nice.

## Opt-out hygine

Here's my proposal to opt-out of hygine. A compiler intrinsic macro which has the following signature:

```rust
macro_rules! create_ident {
	($scope:ident, $($components:ident)+) => { ... }
}
```

The first argument specifies the syntactic context in which the identifier should be created followed by a commo and followed by one or more identifiers which should be concatenated.

This provides a very clean, eplicit opt-out mechanism to macro hygine.

The original example would then look like this:

```rust
macro_rules! create_fn {
	($ident:ident) => {
		fn create_ident!($ident, $ident _helper)() {
			$ident()
		}
	}
}

fn foo() {}

// Create a function `foo_helper` which calls `foo`.
create_fn!(foo);

fn main() {}
```

## Feasability


