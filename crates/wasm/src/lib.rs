//! The shim. It moves bytes across the boundary and decides nothing: an `if`
//! here is a rule that belongs in `sim`, where the test suite can reach it.

use wasm_bindgen::prelude::*;

/// The copy file, as the page reads it. Shipped inside the module rather than
/// fetched, so the strings and the build that uses them cannot drift apart.
#[wasm_bindgen]
pub fn copy_json() -> String {
    content::copy::COPY_JSON.to_string()
}
