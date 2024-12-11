use crate::blocks::Blocks;

#[test]
fn test_put_get() {
    let blocks = Blocks::default();
    blocks.put_new(0, b"hello world"[..].into());
    blocks.put_new(6, b"earth"[..].into());

    let value = blocks.get(0, 5);
    assert_eq!(value.unwrap().as_ref(), &b"hello"[..]);

    let value = blocks.get(6, 5);
    assert_eq!(value.unwrap().as_ref(), &b"world"[..]);
}

#[test]
fn test_send_sync() {
    let blocks = Blocks::default();

    std::thread::scope(|s| {
        s.spawn(|| {
            blocks.get(0, 5);
        });
        s.spawn(|| {
            blocks.put_new(0, b"hello world"[..].into());
        });
    });

    assert!(blocks.get(0, 5).is_some());
}
