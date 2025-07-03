use napi::bindgen_prelude::*;
use napi_derive::napi;
use mimir_sdk::MemoryClient;

#[napi]
pub struct JsMemoryClient {
    client: MemoryClient,
}

#[napi]
impl JsMemoryClient {
    #[napi(constructor)]
    pub fn new(base_url: String, app_id: String) -> Self {
        Self {
            client: MemoryClient::new(base_url, app_id),
        }
    }
    
    #[napi]
    pub async fn ingest(&self, content: String) -> Result<()> {
        // TODO: Implement async call to client
        Ok(())
    }
    
    #[napi]
    pub async fn retrieve(&self, query: String, top_k: u32) -> Result<Vec<String>> {
        // TODO: Implement async call and serialization
        Ok(vec![])
    }
    
    #[napi]
    pub async fn health(&self) -> Result<bool> {
        // TODO: Implement health check
        Ok(true)
    }
} 