import { parse_int_internal, parse_float_internal, str_internal } from './std_internal.js';

export async function parseInt(s) {
    return await parse_int_internal(s);
}
export async function parseFloat(s) {
    return await parse_float_internal(s);
}
export async function str(val) {
    return await str_internal(val);
}


if (typeof main === 'function') { await main(); }
