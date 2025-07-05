use mimir_sdk::MemoryClient;
use pyo3::prelude::*;

/// Python wrapper for MemoryClient
#[pyclass]
struct PyMemoryClient {
    client: MemoryClient,
}

#[pymethods]
impl PyMemoryClient {
    #[new]
    fn new(base_url: String, app_id: String) -> Self {
        Self {
            client: MemoryClient::new(base_url, app_id),
        }
    }

    fn ingest(&self, content: String) -> PyResult<()> {
        // TODO: Implement async wrapper
        Ok(())
    }

    fn retrieve(&self, query: String, top_k: usize) -> PyResult<Vec<String>> {
        // TODO: Implement async wrapper and serialization
        Ok(vec![])
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn mimir(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyMemoryClient>()?;
    Ok(())
}
