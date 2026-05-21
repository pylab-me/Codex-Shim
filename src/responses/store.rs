use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct StoredResponse {
    pub response_object: Value,
    pub chat_messages: Vec<Value>,
}

#[derive(Clone)]
pub struct ResponseStore {
    inner: Arc<RwLock<HashMap<String, StoredResponse>>>,
    max_items: usize,
}

impl ResponseStore {
    pub fn new(max_items: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            max_items,
        }
    }

    pub async fn get(&self, id: &str) -> Option<StoredResponse> {
        self.inner.read().await.get(id).cloned()
    }

    pub async fn put(&self, id: String, value: StoredResponse) {
        let mut guard = self.inner.write().await;
        if guard.len() >= self.max_items {
            // Very small, deterministic eviction: remove one arbitrary oldest-unknown item.
            // This shim is process-local; persistence is intentionally out of scope.
            if let Some(first_key) = guard.keys().next().cloned() {
                guard.remove(&first_key);
            }
        }
        guard.insert(id, value);
    }
}
