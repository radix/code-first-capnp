# Demo Run Example

This example demonstrates the complete end-to-end workflow of using `code-first-capnp` to generate Cap'n Proto schemas from Rust types and then use those schemas with the standard Cap'n Proto Rust library.

## What this example does

1. **Schema Generation**: Uses the `demo-types` crate to generate a Cap'n Proto schema at build time
2. **Code Generation**: Uses `capnpc` to compile the schema into Rust code
3. **Runtime Usage**: Creates and manipulates Cap'n Proto messages using the generated types

## Build Process

The build process is orchestrated by `build.rs`:

1. Runs `cargo run` in the `demo-types` crate to generate the `.capnp` schema
2. Saves the schema to `$OUT_DIR/demo.capnp`
3. Uses `capnpc` to generate Rust code from the schema
4. The generated code is included in `lib.rs` via `include!()` macro

## Dependencies

- **demo-types**: Path dependency on our schema-generating crate
- **capnp**: The standard Cap'n Proto Rust library for runtime usage
- **capnpc**: Build dependency for compiling `.capnp` files to Rust code

## Running the example

From this directory, run:

```bash
cargo run
```

This will:
1. Trigger the build script to generate the schema and Rust code
2. Compile and run the main binary
3. Demonstrate creating and reading Cap'n Proto messages

## What gets generated

The build process creates:
- `demo.capnp` - The Cap'n Proto schema file (in `$OUT_DIR`)
- `demo_capnp.rs` - Generated Rust code for the schema (in `$OUT_DIR`)

## Code Structure

- `build.rs` - Build script that generates schema and compiles it
- `src/lib.rs` - Includes the generated code and re-exports it
- `src/main.rs` - Demonstrates using the generated types
- `Cargo.toml` - Dependencies and build dependencies

This example shows the complete bridge between Rust-first type definitions and Cap'n Proto's binary serialization format.