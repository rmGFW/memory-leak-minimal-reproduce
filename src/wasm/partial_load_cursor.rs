use web_sys::js_sys;

use byteorder::{ByteOrder, LE};
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub enum CursorError {
    OutOfBounds,
    ChunkLoadFailed,
}

pub struct PartialLoadCursor<'a> {
    file: &'a web_sys::File,
    pos: u64,
    chunk: Option<Vec<u8>>,
    chunk_start: u64,
    chunk_size: u64,
}

impl<'a> Drop for PartialLoadCursor<'a> {
    fn drop(&mut self) {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "PartialLoadCursor is being dropped. pos: {}, chunk_start: {}, chunk_size: {}",
            self.pos, self.chunk_start, self.chunk_size
        )));
    }
}

impl<'a> PartialLoadCursor<'a> {
    pub fn new(file: &'a web_sys::File) -> Self {
        let chunk_size = 1024 * 1024 * 16;
        Self {
            file,
            pos: 0,
            chunk: None,
            chunk_start: 0,
            chunk_size,
        }
    }

    pub async fn load_chunk(&mut self) -> Result<(), CursorError> {
        let start = self.pos;
        self.chunk_start = start;
        let end = (start + self.chunk_size).min(self.file.size() as u64);
        let blob = self
            .file
            .slice_with_f64_and_f64(start as f64, end as f64)
            .unwrap();
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let reader = web_sys::FileReader::new().unwrap();
            let reader_clone = reader.clone();
            let onloadend = Closure::wrap(Box::new(move |_event: web_sys::ProgressEvent| {
                let result = reader_clone.result().unwrap();
                resolve.call1(&JsValue::NULL, &result).unwrap();
            }) as Box<dyn FnMut(_)>);

            reader.set_onloadend(Some(onloadend.as_ref().unchecked_ref()));
            reader.read_as_array_buffer(&blob).unwrap();
            onloadend.forget();
        });

        let result = wasm_bindgen_futures::JsFuture::from(promise).await;
        if let Ok(result) = result {
            let array = js_sys::Uint8Array::new(&result);
            let mut chunk = vec![0; array.length() as usize];
            array.copy_to(&mut chunk);
            self.chunk = Some(chunk);
            web_sys::console::log_1(&"loaded".into());
            Ok(())
        } else {
            self.chunk = None;
            Err(CursorError::ChunkLoadFailed)
        }
    }

    pub fn seek(&mut self, pos: u64) -> Result<(), CursorError> {
        if pos > self.len() {
            return Err(CursorError::OutOfBounds);
        }
        self.pos = pos;
        Ok(())
    }

    pub fn len(&self) -> u64 {
        self.file.size() as u64
    }

    fn pos(&self) -> u64 {
        self.pos
    }

    fn left(&self) -> u64 {
        self.len() - self.pos()
    }

    async fn next_bytes(&mut self, n: u64) -> Result<&'a [u8], CursorError> {
        if self.pos + n > self.len() {
            return Err(CursorError::OutOfBounds);
        }

        if n > self.chunk_size {
            self.chunk_size = n * 2;
            self.load_chunk().await?;
        }

        if self.chunk.is_none() || self.pos + n > self.chunk_start + self.chunk_size {
            self.load_chunk().await?;
        }

        let start = (self.pos - self.chunk_start) as usize;
        let end = (self.pos + n - self.chunk_start) as usize;
        let chunk = self.chunk.as_ref().unwrap();
        let slice = &chunk[start..end];
        self.pos += n;
        Ok(unsafe { std::mem::transmute::<&[u8], &'a [u8]>(slice) })
    }

    async fn next_chunk(&mut self) -> Result<&'a [u8], CursorError> {
        let n = self.next_u32().await? as u64;
        let bytes = self.next_bytes(n).await?;
        Ok(bytes)
    }

    pub async fn skip_chunk(&mut self) -> Result<(), CursorError> {
        let n = self.next_u32().await? as u64;
        self.pos += n;
        self.load_chunk().await?;
        Ok(())
    }

    async fn next_u32(&mut self) -> Result<u32, CursorError> {
        Ok(LE::read_u32(self.next_bytes(4).await?))
    }

    async fn next_time(&mut self) -> Result<u64, CursorError> {
        let s = self.next_u32().await? as u64;
        let ns = self.next_u32().await? as u64;
        Ok(1_000_000_000 * s + ns)
    }
}
