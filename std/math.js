import { abs_internal, min_internal, max_internal, round_internal, random_internal } from './std_internal.js';

export async function abs(x) {
    return await abs_internal(x);
}
export async function min(x, y) {
    return await min_internal(x, y);
}
export async function max(x, y) {
    return await max_internal(x, y);
}
export async function round(x) {
    return await round_internal(x);
}
export async function random() {
    return await random_internal();
}


if (typeof main === 'function') { await main(); }
