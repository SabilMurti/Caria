import { url_encode_internal, url_decode_internal, url_parse_query_internal } from './std_internal.js';

export async function encode(s) {
    return await url_encode_internal(s);
}
export async function decode(s) {
    return await url_decode_internal(s);
}
export async function parseQuery(query_str) {
    return await url_parse_query_internal(query_str);
}


if (typeof main === 'function') { await main(); }
