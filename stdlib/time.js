import { now_internal, sleep_internal } from './std_internal.js';

export async function now() {
    return await now_internal();
}
export async function sleep(ms) {
    await sleep_internal(ms);
}


if (typeof main === 'function') { await main(); }
