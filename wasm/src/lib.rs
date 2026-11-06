//! ReliaKit WASM host: raw FFI over the engine JSON dispatcher,
//! with no wasm-bindgen. The demo page loads reliakit_wasm.wasm,
//! calls rk_init for the initial state, and rk_dispatch for every
//! action after that.
//!
//! Memory contract:
//! - rk_alloc(len) returns a pointer to len writable bytes. The
//!   caller fills it and passes it to rk_dispatch, then releases
//!   it with rk_free. Allocations are registered by pointer, and
//!   rk_free releases the registered allocation; the length
//!   argument is accepted for the documented two-argument form
//!   and may be omitted by callers that tracked the size
//!   themselves. rk_free on a pointer that did not come from
//!   rk_alloc (for example a response buffer) is a no-op.
//! - Requests: rk_dispatch(ptr, len) reads len bytes of UTF-8
//!   JSON at ptr. A caller may instead pass len as 0 and write a
//!   length-prefixed buffer: 4 little-endian length bytes, then
//!   the JSON. Both forms are accepted.
//! - rk_init and rk_dispatch return a pointer to a length-prefixed
//!   buffer owned by this module (a thread_local static): the
//!   first 4 bytes are the little-endian length of the UTF-8 JSON
//!   state that follows. The buffer stays valid until the next
//!   rk_init or rk_dispatch call; the caller copies it out and
//!   never frees it (rk_free on it is a no-op).

use reliakit_engine::handle::{handle_str, new_work};
use reliakit_engine::Work;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static WORK: RefCell<Work> = RefCell::new(new_work());
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ALLOCS: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
}

/// Allocate len bytes for a request buffer. The caller owns the
/// buffer and releases it with rk_free.
#[no_mangle]
pub extern "C" fn rk_alloc(len: usize) -> *mut u8 {
    let cap = len.max(1);
    let mut buf: Vec<u8> = Vec::with_capacity(cap);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ALLOCS.with(|a| {
        a.borrow_mut().insert(ptr as usize, cap);
    });
    ptr
}

/// Release a buffer from rk_alloc. Pointers that did not come
/// from rk_alloc (response buffers, stale pointers) are ignored.
///
/// # Safety
/// The pointer must be one previously returned by rk_alloc in
/// this module instance, or null. Passing any other pointer is a
/// no-op only because allocations are looked up in the registry
/// before anything is freed; a pointer value that was never
/// registered is never dereferenced.
#[no_mangle]
pub unsafe extern "C" fn rk_free(ptr: *mut u8, _len: usize) {
    if ptr.is_null() {
        return;
    }
    let cap = ALLOCS.with(|a| a.borrow_mut().remove(&(ptr as usize)));
    if let Some(cap) = cap {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, 0, cap);
        }
    }
}

fn prefixed(json: String) -> *mut u8 {
    OUT.with(|out| {
        let mut buf = out.borrow_mut();
        buf.clear();
        buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
        buf.extend_from_slice(json.as_bytes());
        buf.as_mut_ptr()
    })
}

/// Reset the work state and return the initial UI state as a
/// length-prefixed JSON buffer (see the module docs).
#[no_mangle]
pub extern "C" fn rk_init() -> *mut u8 {
    let json = WORK.with(|w| {
        let mut w = w.borrow_mut();
        *w = new_work();
        reliakit_engine::handle::handle(&mut w, &serde_json::json!({"action": "state"})).to_string()
    });
    prefixed(json)
}

/// Run one JSON request ({"action": ..., "fields": {...}}, see
/// engine/API.md) and return the full UI state as a
/// length-prefixed JSON buffer (see the module docs). With a
/// nonzero len the request is len bytes of JSON at ptr; with
/// len 0 the buffer at ptr is length-prefixed instead.
///
/// # Safety
/// With a nonzero len, ptr must be readable for len bytes. With
/// len 0, ptr must be readable for 4 length bytes plus the
/// payload length those bytes declare. A null ptr is accepted
/// and treated as an empty request.
#[no_mangle]
pub unsafe extern "C" fn rk_dispatch(ptr: *const u8, len: usize) -> *mut u8 {
    let req = if ptr.is_null() {
        "{}".to_string()
    } else if len == 0 {
        let head = unsafe { std::slice::from_raw_parts(ptr, 4) };
        let n = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let bytes = unsafe { std::slice::from_raw_parts(ptr.add(4), n) };
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        String::from_utf8_lossy(bytes).into_owned()
    };
    let json = WORK.with(|w| handle_str(&mut w.borrow_mut(), &req));
    prefixed(json)
}
