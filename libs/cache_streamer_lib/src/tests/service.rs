use std::sync::Arc;

use super::*;
use crate::Service;

#[tokio::test]
async fn test_cache() {
    let backend = Arc::new(SimpleRequestBackend::new(true));
    let service = Service::new(backend.clone(), 1_000_000);

    let _ = service
        .call(&0, "/".into(), &RequestRange::None)
        .await
        .unwrap();
    let _ = service
        .call(&0, "/".into(), &RequestRange::None)
        .await
        .unwrap();
    assert_eq!(backend.request_count(), 1);
}

#[tokio::test]
async fn test_no_cache_on_passthrough() {
    let backend = Arc::new(SimpleRequestBackend::new(false));
    let service = Service::new(backend.clone(), 1_000_000);

    let _ = service
        .call(&0, "/".into(), &RequestRange::None)
        .await
        .unwrap();
    let _ = service
        .call(&0, "/".into(), &RequestRange::None)
        .await
        .unwrap();
    assert_eq!(backend.request_count(), 2);
}

#[tokio::test]
async fn test_expire() {
    let backend = Arc::new(SimpleRequestBackend::new(true));
    let service = Service::new(backend.clone(), 1_000_000);

    let _ = service
        .call(&0, "/".into(), &RequestRange::None)
        .await
        .unwrap();
    let _ = service
        .call(&(EXPIRE_TIME + 1), "/".into(), &RequestRange::None)
        .await
        .unwrap();
    assert_eq!(backend.request_count(), 2);
}
