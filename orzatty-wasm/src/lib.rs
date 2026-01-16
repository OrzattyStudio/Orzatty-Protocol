use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    WebTransport, WebTransportOptions, WritableStreamDefaultWriter, ReadableStreamDefaultReader,
    console
};
use js_sys::{Uint8Array, Reflect};
use orzatty_core::frame::{FrameHeader, FrameType, FrameFlags};
// Removed Framer: use orzatty_core::Framer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

// Need a panic hook for better debugging in browser console
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// The Orzatty Client for Web (WASM)
/// 
/// Bridges rust-core framing with browser WebTransport.
#[wasm_bindgen]
pub struct OrzattyWasmClient {
    transport: WebTransport,
    writer: WritableStreamDefaultWriter,
    callbacks: Arc<Mutex<HashMap<u32, js_sys::Function>>>,
}

#[wasm_bindgen]
impl OrzattyWasmClient {
    /// Connects to a server url (e.g., "https://localhost:5000")
    // Changed from constructor to static method to avoid async constructor warning
    pub async fn connect(url: String, token: String) -> Result<OrzattyWasmClient, JsValue> {
        // ... (Log omitted for brevity)
        let options = WebTransportOptions::new();
        let transport = WebTransport::new_with_options(&url, &options)?;
        
        JsFuture::from(transport.ready()).await?;
        console::log_1(&"WebTransport Ready!".into());

        let stream_promise = transport.create_bidirectional_stream();
        let stream = JsFuture::from(stream_promise).await?;
        
        // Fix: Use generic JS wrapper or specific cast
        let bi_stream: web_sys::WebTransportBidirectionalStream = stream.into();
        let writer = bi_stream.writable().get_writer()?;
        // Fix: Cast WebTransportReceiveStream to ReadableStream
        let readable: web_sys::ReadableStream = bi_stream.readable().into();
        let reader_lock = readable.get_reader().unchecked_into::<ReadableStreamDefaultReader>();

        // ... (Skipping Auth logic for MVP speed)

        // Store transport writer for Datagrams (or Streams)
        let datagrams_writable = transport.datagrams().writable();
        let writer = datagrams_writable.get_writer()?;

        let client = OrzattyWasmClient {
            transport: transport.clone(),
            writer,
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        };

        let callbacks_clone = client.callbacks.clone();
        let incoming_uni = transport.incoming_unidirectional_streams();
        
        wasm_bindgen_futures::spawn_local(async move {
            let reader = incoming_uni.get_reader().unchecked_into::<ReadableStreamDefaultReader>();
            loop {
                 match JsFuture::from(reader.read()).await {
                     Ok(chunk) => {
                         let done = Reflect::get(&chunk, &"done".into()).unwrap().as_bool().unwrap();
                         if done { break; }
                         let value = Reflect::get(&chunk, &"value".into()).unwrap();
                         let stream: web_sys::WebTransportReceiveStream = value.into();
                         Self::handle_stream(stream, callbacks_clone.clone());
                     }
                     Err(_) => break,
                 }
            }
        });

        Ok(client)
    }
    
    // Fix: Prefix unused arg
    fn handle_stream(stream: web_sys::WebTransportReceiveStream, _callbacks: Arc<Mutex<HashMap<u32, js_sys::Function>>>) {
        wasm_bindgen_futures::spawn_local(async move {
             let readable: web_sys::ReadableStream = stream.into();
             let reader = readable.get_reader().unchecked_into::<ReadableStreamDefaultReader>();
             
             loop {
                 let res = JsFuture::from(reader.read()).await;
                 if let Ok(chunk) = res {
                     let done = Reflect::get(&chunk, &"done".into()).unwrap().as_bool().unwrap();
                     if done { break; }
                     
                     let val = Reflect::get(&chunk, &"value".into()).unwrap();
                     let data = Uint8Array::new(&val);
                     let vec = data.to_vec();
                     
                     if vec.len() > 10 {
                          // TODO: Callback logic
                     }
                 } else { break; }
             }
        });
    }

    pub fn on(&self, channel_id: u32, callback: js_sys::Function) {
        self.callbacks.lock().unwrap().insert(channel_id, callback);
    }

    pub async fn send(&self, channel_id: u32, data: &[u8]) -> Result<(), JsValue> {
        let stream_promise = self.transport.create_unidirectional_stream();
        let stream = JsFuture::from(stream_promise).await?;
        let send_stream: web_sys::WebTransportSendStream = stream.into();
        let writer = send_stream.get_writer()?;
        
        let arr = Uint8Array::from(data);
        // Fix: Use write_with_chunk
        JsFuture::from(writer.write_with_chunk(&arr)).await?;
        JsFuture::from(writer.close()).await?;
        
        Ok(())
    }
}
