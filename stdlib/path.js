import { join_internal } from './std_internal.js';

export async function join(a, b) {
    return await join_internal(a, b);
}


if (typeof main === 'function') { await main(); }
