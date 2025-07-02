/// <reference types="vite/client" />

declare module "*.wasm?init" {
  const initWasm: (input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module) => Promise<any>;
  export default initWasm;
}

declare module "*.wasm?url" {
  const url: string;
  export default url;
}
