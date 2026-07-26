import { fetch_internal } from './std_internal.js';

export async function fetch(url) {
    return await fetch_internal(url);
}


if (typeof main === 'function') { await main(); }
