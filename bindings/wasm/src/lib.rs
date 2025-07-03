use wasm_bindgen::prelude::*;
use mimir_sdk::MemoryClient;

#[wasm_bindgen]
pub struct WasmMemoryClient {
    client: MemoryClient,
}

#[wasm_bindgen]
impl WasmMemoryClient {
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, app_id: String) -> Self {
        Self {
            client: MemoryClient::new(base_url, app_id),
        }
    }
    
    #[wasm_bindgen]
    pub async fn ingest(&self, content: String) -> Result<(), JsValue> {
        // TODO: Implement async call
        Ok(())
    }
    
    #[wasm_bindgen]
    pub async fn retrieve(&self, query: String, top_k: usize) -> Result<JsValue, JsValue> {
        // TODO: Implement async call and JS serialization
        Ok(JsValue::NULL)
    }
} 