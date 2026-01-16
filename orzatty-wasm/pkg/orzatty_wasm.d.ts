/* tslint:disable */
/* eslint-disable */

/**
 * The Orzatty Client for Web (WASM)
 *
 * Bridges rust-core framing with browser WebTransport.
 */
export class OrzattyWasmClient {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Connects to a server url (e.g., "https://localhost:5000")
     */
    static connect(url: string, token: string): Promise<OrzattyWasmClient>;
    on(channel_id: number, callback: Function): void;
    send(channel_id: number, data: Uint8Array): Promise<void>;
}

export function start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_orzattywasmclient_free: (a: number, b: number) => void;
    readonly orzattywasmclient_connect: (a: number, b: number, c: number, d: number) => any;
    readonly orzattywasmclient_on: (a: number, b: number, c: any) => void;
    readonly orzattywasmclient_send: (a: number, b: number, c: number, d: number) => any;
    readonly start: () => void;
    readonly wasm_bindgen__closure__destroy__h74e6bd6f43229e6f: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h0c9bc67126ae64e3: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__haa83310f2622f82c: (a: number, b: number, c: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
