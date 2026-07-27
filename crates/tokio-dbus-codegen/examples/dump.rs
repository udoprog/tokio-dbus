//! Print the bindings generated for the interfaces in an interface file.
//!
//! ```sh
//! cargo run --example dump -- interfaces/example.xml com.example.Example
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);

    let file = args.next().ok_or("Usage: dump <file.xml> <interface>...")?;
    let mut builder = tokio_dbus_codegen::Builder::new().file(file);

    for name in args {
        builder = builder.both(name);
    }

    print!("{}", builder.to_string()?);
    Ok(())
}
