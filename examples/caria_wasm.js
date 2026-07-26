class CariaWasm {
    static async load(wasmUrl) {
        let wasmInstance = null;
        let memory = null;

        function getString(ptr, len) {
            if (!memory || ptr === 0) return "";
            const bytes = new Uint8Array(memory.buffer, ptr, len);
            return new TextDecoder("utf-8").decode(bytes);
        }

        const importObject = {
            env: {
                print_int: (val) => console.log("[Caria WASM] Int:", val),
                print_float: (val) => console.log("[Caria WASM] Float:", val),
                print_bool: (val) => console.log("[Caria WASM] Bool:", val === 1),
                print_str: (ptr, len) => console.log("[Caria WASM] Str:", getString(ptr, len)),
                dom_set_text: (idPtr, idLen, textPtr, textLen) => {
                    const id = getString(idPtr, idLen);
                    const text = getString(textPtr, textLen);
                    const el = document.getElementById(id);
                    if (el) el.innerText = text;
                },
                dom_get_int: (idPtr, idLen) => {
                    const id = getString(idPtr, idLen);
                    const el = document.getElementById(id);
                    return el ? BigInt(parseInt(el.value) || 0) : 0n;
                }
            }
        };

        const response = await fetch(wasmUrl);
        const bytes = await response.arrayBuffer();
        const results = await WebAssembly.instantiate(bytes, importObject);
        wasmInstance = results.instance;
        memory = wasmInstance.exports.memory;

        console.log("⚡ Calesteria WebAssembly Engine Loaded!", wasmInstance.exports);
        return wasmInstance.exports;
    }
}
