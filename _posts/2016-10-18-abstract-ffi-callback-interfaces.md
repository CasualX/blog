---
layout: post
title: "Writing abstractions for FFI callback interfaces"
categories: rust
---
Everyone's favourite pastime!

Callbacks can be separated in two categories: those which pass a context pointer and those which don't. They require considerably different designs to abstract over!

## Callbacks with context

Let us consider an extern function that simply calls the callback.

```rust
mod sys {
	use ::std::os::raw::{c_void, c_int};

	// This looks ugly but it's pretty simple:
	// It iterates over some abstract items, calling the callback for each
	// Upon returning `0` will it return the item from that callback.
	// In C tradition a negative return value indicates an error.
	pub unsafe extern "C" fn items(
		callback: unsafe extern "C" fn(item: c_int, user_ptr: *mut c_void) -> c_int,
		user_ptr: *mut c_void
	) -> c_int {
		// Silly implementation for demonstrational purposes
		if callback(42, user_ptr) == 0 {
			return -1;
		}
		else {
			return 42;
		}
	}
}
```

Creating an abstraction for this is pretty simple.

Keep in mind that the API only allows us to pass a thin pointer and `&mut FnMut()` is a fat pointer! Furtunately `mem::transmute` will catch such mistakes. `&mut &mut FnMut()` is a thin pointer but for my sanity I like to write it out with an explicit struct.

```rust
use ::std::mem;
use ::std::os::raw::{c_void, c_int};

// Explicit context for the wrapper.
struct Items<'a>(&'a mut FnMut(i32) -> bool);

// This is the unsafe wrapper that satisfies the interface.
unsafe extern "C" fn items_thunk(item: c_int, user_ptr: *mut c_void) -> c_int {
	// Extract the callback from the `user_ptr`.
	let ctx: &mut Items = mem::transmute(user_ptr);
	// Call it and transform to expected output.
	if (ctx.0)(item) { 1 } else { 0 }
}
```

Note: The callback invocation should not panic as it could unwind into FFI code which is UB.

However `catch_unwind` won't work here as you are not allowed to move the `&mut Items` context into the catch handler. Ugh... See [Further considerations](#further-considerations).

Now the public API for this wrapper.

```rust
#[inline]
pub fn items<F>(mut f: F) -> Option<i32> where F: FnMut(i32) -> bool {
	let code = unsafe {
		let mut ctx = Items(&mut f);
		sys::items(items_thunk, mem::transmute(&mut ctx))
	};
	// Wrap the error checking
	if code < 0 { None }
	else { Some(code) }
}
```

Usage is as elegant as it gets.

```rust
fn main() {
	// Look for the item `42` which we know exists.
	let item = items(|item| {
		item == 42
	});
	assert_eq!(item, Some(42));

	// Look for an item that doesn't exist.
	let not_found = items(|_| {
		false
	});
	assert_eq!(not_found, None);
}
```

Make sure to pay attention to what the extern function is doing with the callback! If it is stored in an internal context, annotate its lifetime correctly eg. `<'a, F: 'a>`. In this case it is not necessary as the closure is never used outside its stack frame.

Play around with it on the [playground](https://play.rust-lang.org/?gist=d41ae14675f1f44ea0667e424a793522&version=stable&backtrace=0).

## Callbacks without context

The previous trick works only because the extern API passes a thin context pointer through which we can `unsafe`ly cast to the expected type.

Let us consider this extern function. Note the lack of any context parameter when installing a handler.

```rust
mod sys {
	use ::std::mem;
	use ::std::os::raw::{c_void, c_int};

	// The handler fn, maybe null.
	// Adds some parameters to avoid a trivial solution.
	pub type HandlerFn = Option<unsafe extern "C" fn(ty: c_int, data: *mut c_void)>;
	// Uh-oh, global state. For this example this is out of our control.
	static mut global: HandlerFn = None;
	// Unsafely installs a global handler and returns the old handler.
	// You are expected to restore the old handler when you are done.
	pub unsafe extern "C" fn install(handler: HandlerFn) -> HandlerFn {
		mem::replace(&mut global, handler)
	}
}
```

Here the `HandlerFn` handles some event, this event has a `ty`pe with values `0` meaning `data` points to a `c_int` and `1` meaning it points to a `c_float`.

Our goal is to build an API that nicely abstracts this behaviour in a safe manner.

I would like to stress that it smuggling in a context pointer is practically impossible. No matter how you slice or dice it, you will not get a `self` value in there (required for the `Fn*` traits).

Note that you _could_ get a context pointer in there if you were to generate thunks at runtime which hard-codes a reference to the `self`. Yeah not gonna happen.

First let us abstract the callback itself, all that business with `ty` and `data` Rust can clearly do better with its powerful enums.

```rust
// Handler callback arguments in Rustic fashion
#[derive(Debug)]
pub enum HandArg<'a> {
	Int(&'a mut i32),
	Float(&'a mut f32),
}

type HandlerFn = Option<fn(HandArg)>;
```

With this in mind let us try a naive approach. Since there is no context parameter, we'll just accept an `fn()` argument directly.

```rust
use ::std::os::raw::{c_int, c_float, c_void};

// Wrapper transforming the arguments before handling control to the user.
unsafe extern "C" fn thunk(ty: c_int, data: *mut c_void) {
	let arg = match ty {
		0 => HandArg::Int(&mut *(data as *mut c_int)),
		1 => HandArg::Float(&mut *(data as *mut c_float)),
		_ => panic!("unexpected type: {} from handler", ty),
	};
	...?
}

// Automatically restore the old handler! Yay guards!
struct Guard(sys::HandlerFn);
impl Drop for Guard {
	fn drop(&mut self) {
		unsafe {
			let _ = sys::install(self.0);
		}
	}
}

fn install(handler: HandlerFn) -> Guard {
	Guard(unsafe { sys::install(Some(thunk)) })
}
```

But this immediately presents some problems. How does `thunk` know what function to call? It would need a function pointer of some sort to call... But that would be exactly the same as smuggling in a context pointer!

For this to work, `thunk` needs to be duplicated for every `fn` handler so the thunk can statically dispatch the callback. In a way you need some way to 'generate' thunks for every handler. That sounds close to requiring runtime code generation ☹.

Actually, not really, generics do _exactly_ this, they generate unique instances for every unique type you give it. Unfortunately the direct translation would require Rust support value generics and could look like this (imaginary syntax):

```rust
use ::std::os::raw::{c_int, c_float, c_void};

// Imaginary generics syntax, constrain value parameter by its type.
unsafe extern "C" fn thunk<f: fn(HandArg)>(ty: c_int, data: *mut c_void) {
	let arg = match ty {
		0 => HandArg::Int(&mut *(data as *mut c_int)),
		1 => HandArg::Float(&mut *(data as *mut c_float)),
		_ => panic!("unexpected type: {} from handler", ty),
	};
	// Imaginary syntax where `f` is a value, not a type. Some day, maybe?
	f(arg);
}

fn install<f: fn(HandArg)>() -> Guard {
	// Generates an instance of `thunk` specialized for this callback.
	Guard(unsafe { sys::install(Some(thunk::<f>)) })
}
```

One thing to take away from this is that _generic parameters_ do not affect the signature of the function if you don't want to. This makes them very powerful in this context.

So let us use the next best thing with a little more boilerplate: traits!

```rust
use ::std::os::raw::{c_int, c_float, c_void};

pub trait Handler {
	// Implement this!
	fn handle(HandArg);

	// Let's hide this from docs, it's a private detail...
	#[doc(hidden)]
	unsafe extern "C" fn thunk(ty: c_int, data: *mut c_void) {
		let arg = match ty {
			0 => HandArg::Int(&mut *(data as *mut c_int)),
			1 => HandArg::Float(&mut *(data as *mut c_float)),
			_ => panic!("unexpected type: {} from handler", ty),
		};
		// Call the user-defined handler.
		Self::handle(arg);
	}
}

// Note that the installer only has a generic parameter!
fn install<H: Handler>() -> Guard {
	// A new thunk is generated specialized for its callback.
	Guard(unsafe { sys::install(Some(H::thunk)) })
}
```

That will work well enough for our use case; simply create a dummy type and implement `Handler` with your callback!

```rust
fn main() {
	enum MyHandler {}
	impl Handler for MyHandler {
		fn handle(arg: HandArg) {
			println!("{:?}", arg);
		}
	}
	let _guard = install::<MyHandler>();
	// Guard will cleanup when it goes out of scope.
	// Note that *must* give it a name (probably prefixed with an underscore)
	// If you don't, or name it `_` rust will drop the value right there.
}
```

Could do with less boilerplate when Rust gets value generics but this isn't so bad.

Play around with the working sample on the [playground](https://play.rust-lang.org/?gist=dd4564629ea31ff635fa0056ae5e8fa7&version=stable&backtrace=0).

## Further considerations

FFI and unwinding: Ugh... What if the extern fn just wasn't designed to return errors? Just abort on panic would be my best guess... How do you even abort? Searching for [`abort`](https://doc.rust-lang.org/std/?search=abort) does yield a stable function in the standard library. I'd write a type which panics in its `Drop` impl, that should do the trick. Don't forget to `mem::forget` it before it goes out of scope.

In the [Callbacks without context](#callbacks-without-context) section the `Guard` holds a token used to restore some state. If the thunk pointer _is_ the token you can do better by making `Guard` generic over `H: Handler` avoiding wasting memory holding on to the token.

In the [Callbacks with context](#callbacks-with-context) section the `items_thunk` has double indirection but you can apply the same technique by making the thunk generic over an `Fn*` trait without affecting its signature! This generates a unique thunk for your closure, same trade-offs apply wrt generic bloat.

```rust
// This is the unsafe wrapper that satisfies the interface.
unsafe extern "C" fn items_thunk<F>(item: c_int, user_ptr: *mut c_void) -> c_int
	where F: FnMut(i32) -> bool
{
	// Extract the callback from the `user_ptr`.
	let f: &mut F = mem::transmute(user_ptr);
	// Call it and transform to expected output.
	if f(item) { 1 } else { 0 }
}
```
