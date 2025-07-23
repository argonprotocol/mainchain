import * as fs from 'node:fs';
// Clean up the generated wasm bindings by removing the __wbindgen_start call
// This is necessary to avoid issues with the WebAssembly module initialization
const filename = 'ts/wasm/bitcoin_bindings.js';
const contents = fs.readFileSync(filename, 'utf8');
// Remove the __wbindgen_start call at the end of the file
fs.writeFileSync(filename, contents.replace('wasm.__wbindgen_start();', ''));
