---
layout: post
title: "Semantic versioning is unsound"
author: "Casper"
categories: programming
---

I've been thinking a lot about versioning software libraries lately and [semantic versioning](https://semver.org/) promises a clean way to organize version of your libraries.

Semantic versioning specifies in [point 8](https://semver.org/#spec-item-8) (emphasis mine):

> 8\. **Major version X (X.y.z \| X > 0) MUST be incremented if any backward incompatible changes are introduced to the public API.** It MAY also include minor and patch level changes. Patch and minor versions MUST be reset to 0 when major version is incremented.

I believe this is unsound. I could never put my finger on why I feel this way but in this post I'm going to try to clarify my thoughts.

With unsound I mean that by following the rules of semantic versioning locally, it does not guarantee that you will not break code far, far away from your library.

## The Good

Let's start with a simple scenario:

We have a shared library 'Common' with two major versions, v1.0.0 and v2.0.0. As per semantic versioning this is perfectly fine! When you have a breaking change, simply bump the major version and be on your merry way! Alas...

Let's introduce two new libraries which depend on this common library: LibA and LibZ. LibA is a bit older and still uses Common v1.0.0, LibZ on the other hand was published recently and of course wants to use the latest features available in Common v2.0.0.

At your dayjob you're about to introduce your latest whizbang application! You've done your research and really need both LibA and LibZ to implement your cool widget! Except... it doesn't work. At least in some programming languages you're in for a hell of a ride.

## The Bad

I'm only going to talk here about programming languages I'm familiar with, I don't have anything specific against them it's just where I was able to create problems.

### Python

Oh boy, I don't have to try too hard 🙂

In python all the dependencies of your project get installed in a (single) virtual environment (at least I hope you're using virtual environments...). There is simply no way for python itself to understand which dependencies are declared by each library and restrict access to only those declared dependencies.

It is not possible to install multiple versions of the same library in the same virtual environment (I'm sure someone will chime in with a wonderful workaround you should definitely use in production).

In this world, there can be no breaking changes. Ever. The promise of semver is unfulfilled and introducing a breaking change is simply unsound.

### C#

One day at work I was happily writing a Windows application in C#, using my corporate Visual Studio. As I was installing nuget packages and managing C# project References a thought occurred to me. What happens if I install this library written by my colleagues (let's call it WidgetBeanFactory) which uses [Newtonsoft.Json](https://www.nuget.org/packages/Newtonsoft.Json/) internally.

Now things get exciting. In the top level application, I write:

```C#
using Newtonsoft.Json;
```

It works. I am able to use dependencies of my dependencies even though this is an implementation detail and is nowhere exposed in the public interface of WidgetBeanFactory.

This is problematic. C# does not seem to track which dependencies I have declared in my project and simply has a global list of assemblies anyone is able to import.

This runs into the same problem as python. Any breaking change simply breaks the world. The promise of major versions allowing breaking changes is unsound.

I am told there's something called [Strong Naming](https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/strong-naming) in C# that may somewhat help but I haven't looked at it in more detail.

### Rust

Surely these are just symptoms of old programming languages attempting to bolt on modern package management and being unable to keep backwards compatibility. Why can't they just bump their major version number as semantic versioning suggests? Well... more on that later.

First let's talk about the same scenario Rust.

Rust will restrict access to libraries to the ones you've declared in your Cargo.toml. You cannot use dependencies of your dependencies without their explicit consent.

This allows Rust to solve the original problem of both LibA and LibZ using different versions of Common library by simply importing them twice and letting LibA and LibZ only use the library they have declared.

Problem solved!

Well there wouldn't be a blog post if it was that easy: what if any types of Common are exposed in the public interface of LibA and LibZ?

In LibA:

```rust
use common::FancyComponent;

pub fn create_fancy_component() -> FancyComponent { ... }
```

In LibZ:

```rust
use common::FancyComponent;

pub fn consume_fancy_component(fc: FancyComponent) { ... }
```

In your application:

```rust
use liba::create_fancy_component;
use libz::consume_fancy_component;

fn main() {
	let fc = create_fancy_component();
	consume_fancy_component(fc);
}
```

This results in a compile error:

<pre><font color="#26A269"><b>    Checking</b></font> myapp v0.1.0 (~/myapp)
<font color="#F66151"><b>error[E0308]</b></font><b>: mismatched types</b>
 <font color="#2A7BDE"><b>--&gt; </b></font>src/main.rs:6:26
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>6</b></font> <font color="#2A7BDE"><b>|</b></font>     consume_fancy_component(fc);
  <font color="#2A7BDE"><b>| </b></font>    <font color="#2A7BDE"><b>-----------------------</b></font> <font color="#F66151"><b>^^</b></font> <font color="#F66151"><b>expected `FancyComponent`, found a different `FancyComponent`</b></font>
  <font color="#2A7BDE"><b>| </b></font>    <font color="#2A7BDE"><b>|</b></font>
  <font color="#2A7BDE"><b>| </b></font>    <font color="#2A7BDE"><b>arguments to this function are incorrect</b></font>
  <font color="#2A7BDE"><b>|</b></font>
  <font color="#2A7BDE"><b>= </b></font><b>note</b>: `FancyComponent` and `FancyComponent` have similar names, but are actually distinct types
<font color="#33DA7A"><b>note</b></font>: `FancyComponent` is defined in crate `common`
 <font color="#2A7BDE"><b>--&gt; </b></font>~/common/src/lib.rs:2:1
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>2</b></font> <font color="#2A7BDE"><b>|</b></font> pub struct FancyComponent {}
  <font color="#2A7BDE"><b>| </b></font><font color="#33DA7A"><b>^^^^^^^^^^^^^^^^^^^^^^^^^</b></font>
<font color="#33DA7A"><b>note</b></font>: `FancyComponent` is defined in crate `common`
 <font color="#2A7BDE"><b>--&gt; </b></font>~/common/src/lib.rs:2:1
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>2</b></font> <font color="#2A7BDE"><b>|</b></font> pub struct FancyComponent {}
  <font color="#2A7BDE"><b>| </b></font><font color="#33DA7A"><b>^^^^^^^^^^^^^^^^^^^^^^^^^</b></font>
  <font color="#2A7BDE"><b>= </b></font><b>note</b>: perhaps two different versions of crate `common` are being used?
<font color="#33DA7A"><b>note</b></font>: function defined here
 <font color="#2A7BDE"><b>--&gt; </b></font>~/libz/src/lib.rs:3:8
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>3</b></font> <font color="#2A7BDE"><b>|</b></font> pub fn consume_fancy_component(fc: FancyComponent) {}
  <font color="#2A7BDE"><b>| </b></font>       <font color="#33DA7A"><b>^^^^^^^^^^^^^^^^^^^^^^^</b></font>

<b>For more information about this error, try `rustc --explain E0308`.</b>
<font color="#C01C28"><b>error</b></font><b>:</b> could not compile `myapp` (bin &quot;myapp&quot;) due to 1 previous error
</pre>

It's good that Rust recognizes the source of the conflict (with an amazing error message), but this doesn't help us solve the problem. In fact I believe this problem is unsolvable. If you have no control over LibA and LibZ and you're not in a position to upgrade LibA to use the same version of Common as LibZ there is no recourse.

These libraries are fundamentally incompatible. The worst part is that according to semantic versioning, the Common library did nothing wrong!

Even worse if the breaking change didn't affect FancyComponent, it was somewhere else! And yet despite the Common library's authors best intentions to adhere to semantic versions and bumping the major version number when introducing a breaking change it introduced a fatal error in the consumer of libraries that depend on Common.

It is this aspect of semantic versioning that is unsound. Following the instructions to bump the major version when introducing a breaking change caused code far, far away to break unexpectedly. I say unexpectedly because the consumer of LibA and LibZ had no choice in this matter and has no (reasonable) recourse.

## The Ugly

Let's talk about the standard library (every programming language has something to that effect). It's a library right? It even has the word library right in the name! Can we apply semantic versioning to the standard library of your favourite programming language?

Given what I've just written the answer is a clear: **NO**.

The standard library can fix bugs, sure. The standard library can add features, sure. But the standard library can _never_ introduce breaking changes.

I'm interested in understanding why exactly is the standard library special, why can it _really_ not afford to break things?

I believe it's in the interface. The standard library (and the language itself) defines the glue which binds your libraries together.

Libraries define public interfaces which use types like integers, floats, String, Option, Result, File and many other commonly used types defined by the standard library / programming language.

It is these kind of types that are used to connect libraries together that when subject to breaking changes cause these problems. They are a special kind of object that must not be subject to semantic versioning's idea of 'just bump the major version and you're good'.

I fact I'm struggling to come up with what would be fine in a breaking change.

Any publicly defined type, class or interface suffers from this problem. The only thing that may be fine is a function, if it doesn't use any custom types and only types from the standard library (where we've already established it simply cannot afford to break anything).

## Conclusion

Semantic versioning's promise of 'just bump the major version when introducing a breaking change' is too simple. There are too many things that can be broken when bumping the major version of a library.

You should not be bumping your major version too often. Maybe consider accepting breaking a few consumers by sneaking in a _'tiny'_ breaking change in a minor or patch update instead of religiously updating the major version.

However in practice it seems this problem just doesn't come up all that often, but when it does it is catastrophic.

Or you could just ignore the problem and be happy! Ignorance is bliss 🙂
