# Sierra Programming Language

So in the beginning this should be a simple language that compiles to C.
Something like this:

```rs
// main.sr
@include("stdio.h")

fn main() {
    printf("Hello world\n")
}
```

It should support using the C standard library, so like printf, scanf etc. It should support header files in the beginning to make things simpler.

```rs
// stdio.h
fn printf(fmt: []char, ...) -> i32
```
