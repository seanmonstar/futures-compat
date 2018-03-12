# futures-compat

This is a compatibility shim between [futures][] v0.1 and v0.2. It provides implementations that allow a type that implements `Future` from v0.1 to act as a `Future` from v0.2, and vice-versa.

**Note**: Task-local data doesn't really work, and getting it to work would require horrid unsafe hackery. It could probably be done if there was enough demand.

## Example

### Using a v0.1 Future as v0.2

```rust
extern crate futures_compat;
use futures_compat::futures_01::FutureInto02;

let futv01 = some_lib_that_hasnt_upgraded();

let futv02 = futv01.into_02_compat()
    .map(|val| {
        println!("map from v0.2! {:?}", val);
        Ok(())
    });

// spawn in a futures 0.2 executor
```

### Using a v0.2 Future as v0.1

```rust
extern crate futures_compat;
use futures_compat::futures_01::FutureInto01;

let futv02 = some_lib_using_the_new_hotness();
let exec = get_my_current_executor();

let futv01 = futv02.into_01_compat(exec)
    .map(|val| {
        println!("map from v0.1! {:?}", val);
        Ok(())
    });

// spawn in a futures 0.1 executor
futv01.wait().unwrap();
```
