import { read_text_internal, write_text_internal, exists_internal } from './std_internal.js';

export async function readText(path) {
    return await read_text_internal(path);
}
export async function writeText(path, content) {
    return await write_text_internal(path, content);
}
export async function exists(path) {
    return await exists_internal(path);
}


if (typeof main === 'function') { await main(); }
