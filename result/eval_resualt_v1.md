Feeding 5 dataset rows through Claude (mapped prompt)

======================================================================
id: 1
Full user prompt:
Please answer the user's question:

What is the capital of Afghanistan?

Reference answer:
Kabul is the capital and largest city of Afghanistan.

Claude Response:
The capital of Afghanistan is Kabul. It is the largest city in the country and has been the capital since 1776.

======================================================================
id: 2
Full user prompt:
Please answer the user's question:

In one sentence, what does HTTP stand for?

Reference answer:
HTTP stands for Hypertext Transfer Protocol, the application-layer protocol used for fetching web pages and APIs on the Internet.

Claude Response:
HTTP stands for HyperText Transfer Protocol.

======================================================================
id: 3
Full user prompt:
Please answer the user's question:

What is 15% of 80?

Reference answer:
15% of 80 is 12, because 0.15 × 80 = 12.

Claude Response:

# 15% of 80

To find 15% of 80:

**80 × 0.15 = 12**

The answer is **12**.

======================================================================
id: 4
Full user prompt:
Please answer the user's question:

What is ownership in Rust?

Reference answer:
Ownership is Rust’s compile-time rule that each value has a single owner, values are freed when their owner goes out of scope, and ownership can move or be temporarily borrowed (& and &mut) without violating aliasing XOR mutability rules.

Claude Response:

# Ownership in Rust

Ownership is Rust's core memory management system that ensures memory safety without requiring a garbage collector. Here are the key concepts:

## The Three Rules of Ownership

1. **Each value has one owner** - A value can only have one variable that owns it at a time
2. **Values can be borrowed** - You can lend references to values without transferring ownership
3. **When the owner goes out of scope, the value is dropped** - Memory is automatically freed

## Basic Example

```rust
fn main() {
    let s1 = String::from("hello");  // s1 owns the string
    let s2 = s1;                      // ownership moves to s2

    println!("{}", s1);  // ❌ ERROR: s1 no longer owns the string
    println!("{}", s2);  // ✅ OK
}
```

## Move vs. Copy

**Move** - For complex types (String, Vec, etc.)

```rust
let s1 = String::from("hello");
let s2 = s1;  // s1's value moves to s2
```

**Copy** - For simple types (integers, booleans, etc.)

```rust
let x = 5;
let y = x;  // x is copied (not moved)
println!("{}", x);  // ✅ OK, x still valid
```

## Borrowing (References)

Use `&` to borrow without transferring ownership:

```rust
fn main() {
    let s1 = String::from("hello");
    let len = calculate_length(&s1);  // borrow s1

    println!("'{}' has length {}", s1, len);  // ✅ s1 still valid
}

fn calculate_length(s: &String) -> usize {
    s.len()
}  // s goes out of scope, but doesn't own the data
```

## Mutable References

```rust
fn main() {
    let mut s = String::from("hello");
    change(&mut s);  // mutable borrow
    println!("{}", s);  // "hello world"
}

fn change(s: &mut String) {
    s.push_str(" world");
}
```

**Rule**: You can have _either_ one mutable reference OR multiple immutable references at a time.

## Key Benefits

✅ Memory safety at compile-time
✅ No garbage collector overhead
✅ No null pointer exceptions (uses `Option` instead)
✅ Prevents data races in concurrent code

Ownership is what makes Rust unique and enables its safety guarantees!

======================================================================
id: 5
Full user prompt:
Please answer the user's question:

Name two greenhouse gases.

Reference answer:
Carbon dioxide (CO₂) and methane (CH₄) are two greenhouse gases that trap heat in Earth’s atmosphere.

Claude Response:

# Two Greenhouse Gases

1. **Carbon dioxide (CO₂)** - Released primarily through burning fossil fuels and deforestation

2. **Methane (CH₄)** - Released from livestock farming, rice paddies, and decomposition of organic waste

======================================================================
