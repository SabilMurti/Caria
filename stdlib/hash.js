import { keys_internal, values_internal, entries_internal, has_key_internal, get_or_default_internal } from './std_internal.js';

export async function keys(map) {
    return await keys_internal(map);
}
export async function values(map) {
    return await values_internal(map);
}
export async function entries(map) {
    return await entries_internal(map);
}
export async function hasKey(map, key) {
    return await has_key_internal(map, key);
}
export async function getOrDefault(map, key, default_val) {
    return await get_or_default_internal(map, key, default_val);
}


if (typeof main === 'function') { await main(); }
