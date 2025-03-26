use wasm_bindgen::prelude::*;

mod partial_load_cursor;

use partial_load_cursor::PartialLoadCursor;

#[wasm_bindgen(catch)]
pub async fn test_bigfile(file: web_sys::File) {
    let mut cursor = PartialLoadCursor::new(&file);
    for offset in (0..1000) {
        cursor.seek(offset as u64);
        cursor.load_chunk().await;
        web_sys::console::log_1(&format!("Offset: {}", offset).into());
    }
}
