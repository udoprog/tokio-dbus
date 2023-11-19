use super::Auth;

#[test]
fn test_external_from_uid() {
    assert_eq!(
        Auth::external_from_u32_ascii_hex(&mut [0; 32], 1000),
        Auth::External(b"31303030")
    );
    assert_eq!(
        Auth::external_from_u32_ascii_hex(&mut [0; 32], u32::MAX),
        Auth::External(b"34323934393637323935")
    );
    assert_eq!(
        Auth::external_from_u32_ascii_hex(&mut [0; 32], 0),
        Auth::External(b"00")
    );
}
