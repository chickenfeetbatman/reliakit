// Transport layer. One codebase, two hosts:
// - Tauri desktop: window.__TAURI__ is present, calls invoke("dispatch").
// - Browser demo: the same engine compiled to WASM, loaded from
//   ./reliakit_wasm.wasm, driven through the raw FFI exports
//   rk_alloc, rk_free, rk_init, rk_dispatch with length prefixed
//   JSON buffers (4 byte little endian length, then UTF-8 JSON).
// If neither host is available the transport reports "offline" and the
// shell still renders, so the page can be opened from file:// for review.

export function detectHost() {
  if (typeof window !== "undefined" && (window.__TAURI__ || window.__TAURI_INTERNALS__)) return "tauri";
  return "wasm";
}

function tauriInvoke() {
  const t = window.__TAURI__;
  if (t && t.core && typeof t.core.invoke === "function") return t.core.invoke.bind(t.core);
  if (t && typeof t.invoke === "function") return t.invoke.bind(t);
  const internals = window.__TAURI_INTERNALS__;
  if (internals && typeof internals.invoke === "function") return internals.invoke.bind(internals);
  return null;
}

export class Transport {
  constructor() {
    this.kind = detectHost();
    this.ready = false;
    this.error = "";
    this._mod = null;
    this._memory = null;
  }

  async init() {
    if (this.kind === "tauri") {
      this.ready = !!tauriInvoke();
      if (!this.ready) this.error = "Tauri bridge was not found in this window.";
      return this.ready;
    }
    try {
      const url = new URL("./reliakit_wasm.wasm", document.baseURI).href;
      const res = await fetch(url);
      if (!res.ok) throw new Error("WASM engine file returned HTTP " + res.status);
      const bytes = await res.arrayBuffer();
      const { instance } = await WebAssembly.instantiate(bytes, {});
      const ex = instance.exports;
      for (const name of ["rk_alloc", "rk_free", "rk_dispatch"]) {
        if (typeof ex[name] !== "function") throw new Error("WASM export " + name + " is missing.");
      }
      this._mod = ex;
      this._memory = ex.memory || null;
      if (typeof ex.rk_init === "function") ex.rk_init();
      this.ready = true;
      return true;
    } catch (err) {
      this.ready = false;
      this.error = "The browser demo engine could not be loaded: " + (err && err.message ? err.message : String(err));
      return false;
    }
  }

  _writeJson(text) {
    const ex = this._mod;
    const data = new TextEncoder().encode(text);
    const ptr = ex.rk_alloc(data.length + 4);
    if (!ptr) throw new Error("rk_alloc returned no memory.");
    const mem = new Uint8Array(this._memory.buffer);
    const view = new DataView(this._memory.buffer);
    view.setUint32(ptr, data.length, true);
    mem.set(data, ptr + 4);
    return ptr;
  }

  _readJson(ptr) {
    const view = new DataView(this._memory.buffer);
    const len = view.getUint32(ptr, true);
    const bytes = new Uint8Array(this._memory.buffer, ptr + 4, len);
    return new TextDecoder().decode(bytes.slice());
  }

  async dispatch(request) {
    if (this.kind === "tauri") {
      const invoke = tauriInvoke();
      if (!invoke) throw new Error(this.error || "Tauri bridge unavailable.");
      const out = await invoke("dispatch", { request });
      return typeof out === "string" ? JSON.parse(out) : out;
    }
    if (!this.ready) throw new Error(this.error || "Demo engine unavailable.");
    const ex = this._mod;
    const reqPtr = this._writeJson(JSON.stringify(request));
    let resPtr = 0;
    try {
      resPtr = ex.rk_dispatch(reqPtr);
      if (!resPtr) throw new Error("The engine returned no response.");
      // The response buffer lives in WASM memory, so refresh the view
      // in case dispatch grew memory, then copy the text out.
      this._memory = ex.memory || this._memory;
      const text = this._readJson(resPtr);
      return JSON.parse(text);
    } finally {
      try { ex.rk_free(reqPtr); } catch (e) { /* buffer already released */ }
      if (resPtr) { try { ex.rk_free(resPtr); } catch (e) { /* buffer already released */ } }
    }
  }
}

export function downloadTextFile(filename, text) {
  const blob = new Blob([text], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = filename;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
}

export function pickTextFile(accept) {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file"; input.accept = accept || ".json,.csv,.txt";
    input.className = "hidden-file-input";
    input.addEventListener("change", async () => {
      const file = input.files && input.files[0];
      if (!file) { resolve(null); return; }
      try { resolve({ name: file.name, text: await file.text() }); }
      catch (err) { resolve(null); }
    });
    document.body.appendChild(input);
    input.click();
    setTimeout(() => input.remove(), 60000);
  });
}
