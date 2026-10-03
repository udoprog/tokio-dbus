use crate::{Builder, Result};

const EXAMPLE: &str = r#"
<node>
  <interface name="com.example.Example">
    <method name="Add">
      <arg name="a" type="i" direction="in"/>
      <arg name="b" type="i" direction="in"/>
      <arg name="sum" type="i" direction="out"/>
    </method>
    <method name="Reset"/>
    <method name="Split">
      <arg name="input" type="s" direction="in"/>
      <arg name="head" type="s" direction="out"/>
      <arg name="tail" type="as" direction="out"/>
    </method>
    <signal name="Changed">
      <arg name="value" type="i"/>
    </signal>
    <signal name="Reset"/>
    <property name="Total" type="i" access="read"/>
    <property name="Label" type="s" access="readwrite"/>
    <property name="Hints" type="a{sv}" access="read"/>
  </interface>
</node>
"#;

/// Write the interface file under a name of its own, since the tests run in
/// parallel and would otherwise read a file another one is still writing.
fn builder(name: &str) -> Builder {
    let dir = std::env::temp_dir().join("tokio-dbus-codegen-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.xml"));
    std::fs::write(&path, EXAMPLE).unwrap();
    Builder::new().file(&path)
}

const EXAMPLE_NAME: &str = "com.example.Example";

#[test]
fn generates_a_client() -> Result<()> {
    let out = builder("client").client(EXAMPLE_NAME).to_string()?;
    assert!(out.contains("pub mod example"));
    assert!(out.contains("pub struct Example"));
    assert!(out.contains("pub async fn add("));
    assert!(out.contains("a: i32,"));
    assert!(out.contains("-> Result<i32>"));
    assert!(out.contains("-> Result<(String, Vec<String>)>"));
    assert!(out.contains("pub async fn label(&self, conn: &mut Connection) -> Result<String>"));
    assert!(out.contains("pub async fn set_label("));
    assert!(!out.contains("pub async fn set_total("));
    assert!(out.contains("pub enum Signal"));
    assert!(out.contains("Changed { "));
    assert!(!out.contains("ExampleServer"));
    assert!(out.contains("__reply.checked_body(Signature::new_const(b\"i\"))?"));
    assert!(out.contains("__reply.checked_body(Signature::EMPTY)?"));
    assert!(out.contains("message.checked_body(Signature::new_const(b\"i\"))?"));
    Ok(())
}

#[test]
fn generates_a_server() -> Result<()> {
    let out = builder("server").server(EXAMPLE_NAME).to_string()?;
    assert!(out.contains("pub trait ExampleServer"));
    assert!(out.contains("async fn add(&mut self, a: i32, b: i32) -> Result<i32>;"));
    assert!(out.contains("async fn reset(&mut self) -> Result<()>;"));
    assert!(out.contains("async fn total(&mut self) -> Result<i32>;"));
    assert!(out.contains("async fn set_label(&mut self, value: String) -> Result<()>;"));
    assert!(out.contains("pub async fn dispatch<T>"));
    assert!(out.contains("async fn dispatch_properties<T>"));
    assert!(!out.contains("pub struct Example {"));
    assert!(out.contains("pub enum Property"));
    assert!(out.contains("Total,"));
    assert!(out.contains("Label,"));
    assert!(out.contains("Hints,"));
    assert!(out.contains("pub async fn properties_changed<T>"));
    assert!(out.contains("pub fn properties_invalidated("));
    assert!(out.contains("b\"sa{sv}as\""));
    assert!(out.contains("call.checked_body(Signature::new_const(b\"ii\"))?"));
    assert!(out.contains("fn __decode_reset(call: &Call) -> Result<()>"));
    assert!(out.contains("call.checked_body(Signature::EMPTY)?"));
    Ok(())
}

#[test]
fn no_property_enum_without_readable_properties() -> Result<()> {
    const METHODS_ONLY: &str = r#"
    <node>
      <interface name="com.example.Bare">
        <method name="Poke"/>
      </interface>
    </node>
    "#;

    let dir = std::env::temp_dir().join("tokio-dbus-codegen-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bare.xml");
    std::fs::write(&path, METHODS_ONLY).unwrap();

    let out = Builder::new()
        .file(&path)
        .server("com.example.Bare")
        .to_string()?;

    assert!(!out.contains("pub enum Property"));
    assert!(!out.contains("properties_changed"));
    Ok(())
}

#[test]
fn generates_both() -> Result<()> {
    let out = builder("both").both(EXAMPLE_NAME).to_string()?;
    assert!(out.contains("pub struct Example {"));
    assert!(out.contains("pub trait ExampleServer"));
    Ok(())
}
