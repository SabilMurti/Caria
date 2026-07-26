import { map_internal, filter_internal, find_internal, push_internal, concat_internal, slice_internal, includes_internal } from './std_internal.js';

export async function map(list, cb) {
    return await map_internal(list, cb);
}
export async function filter(list, cb) {
    return await filter_internal(list, cb);
}
export async function find(list, cb) {
    return await find_internal(list, cb);
}
export async function push(list, item) {
    return await push_internal(list, item);
}
export async function concat(a, b) {
    return await concat_internal(a, b);
}
export async function slice(list, start, end) {
    return await slice_internal(list, start, end);
}
export async function includes(list, val) {
    return await includes_internal(list, val);
}


if (typeof main === 'function') { await main(); }
