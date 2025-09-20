# Demo Types Example

This is a full example crate demonstrating how to use `code-first-capnp` to generate Cap'n Proto schemas from Rust structs and enums.

## What this example shows

This example demonstrates:

- **Basic struct definitions** with various primitive types (`Person`, `Company`)
- **Nested structs** (`Company` contains a list of `Person`)
- **Simple enums** (`Status`) 
- **Enums with data** (`EnumWithData`) showing different variant types
- **Empty structs** (`EmptyStruct`)
- **Backwards compatibility** (`UserProfileV2`) with deprecated/removed fields using `#[facet(capnp:extra)]`
- **Custom field names** using `#[facet(capnp:name)]`
- **Custom field IDs** using `#[facet(capnp:id)]`

## Running the example

From this directory, run:

```bash
cargo run
```

This will output the generated Cap'n Proto schema to stdout.

## Structure

- `src/lib.rs` - Contains all the type definitions with `#[derive(Facet)]` annotations
- `src/main.rs` - Binary that generates and prints the schema
- `Cargo.toml` - Depends on the main `code-first-capnp` crate via path dependency

## Generated Schema

The example generates a complete `.capnp` schema file that can be used with any Cap'n Proto implementation. The schema includes:

- Proper field numbering and types
- Backwards compatibility annotations
- Custom field names where specified
- Union types for Rust enums