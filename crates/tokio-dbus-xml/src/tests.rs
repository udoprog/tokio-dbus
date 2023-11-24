use crate::{parse_interface, Result};

const SIMPLE: &str = r#"
<!DOCTYPE node PUBLIC
    "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
    "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd" >
<node xmlns:doc="http://www.freedesktop.org/dbus/1.0/doc.dtd">
  <interface name="com.example.MyService1.InterestingInterface">
    <method name="AddContact">
      <arg name="name" direction="in" type="s">
        <doc:doc><doc:summary>Name of new contact</doc:summary></doc:doc>
      </arg>
      <arg name="email" direction="in" type="s">
        <doc:doc><doc:summary>E-mail address of new contact</doc:summary></doc:doc>
      </arg>
      <arg name="id" direction="out" type="u">
        <doc:doc><doc:summary>ID of newly added contact</doc:summary></doc:doc>
      </arg>
      <doc:doc>
        <doc:description>
          <doc:para>
            Adds a new contact to the address book with their name and
            e-mail address.
          </doc:para>
        </doc:description>
      </doc:doc>
    </method>
  </interface>
</node>
"#;

#[test]
fn test_simple() -> Result<()> {
    let node = parse_interface(SIMPLE)?;
    assert_eq!(
        node.interfaces[0].name,
        "com.example.MyService1.InterestingInterface"
    );
    assert_eq!(node.interfaces[0].methods[0].name, "AddContact");
    Ok(())
}
