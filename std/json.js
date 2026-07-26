import { stringify_internal, parse_internal } from './std_internal.js';

export async function stringify(val) {
    return await stringify_internal(val);
}
export async function parse(s) {
    return await parse_internal(s);
}


if (typeof main === 'function') { await main(); }
