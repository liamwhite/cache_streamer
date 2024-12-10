use crate::blocks::Blocks;
use bytes::Bytes;

#[test]
fn test_put_get() {
    let blocks = Blocks::default();
    blocks.put_new(0, Bytes::from(&b"hello world"[..]));
    blocks.put_new(6, Bytes::from(&b"earth"[..]));

    let value = blocks.get(0, 5);
    assert_eq!(value.as_ref().map(|b| b.as_ref()), Some(&b"hello"[..]));

    let value = blocks.get(6, 5);
    assert_eq!(value.as_ref().map(|b| b.as_ref()), Some(&b"world"[..]));
}

#[test]
fn test_send_sync() {
    let blocks = Blocks::default();

    std::thread::scope(|s| {
        s.spawn(|| {
            blocks.get(0, 5);
        });
        s.spawn(|| {
            blocks.put_new(0, Bytes::from(&b"hello world"[..]));  
        });
    });

    assert!(blocks.get(0, 5).is_some());
}
