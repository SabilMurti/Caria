import { split_internal, join_str_internal, trim_internal, replace_internal, contains_internal, starts_with_internal, ends_with_internal, to_lower_internal, to_upper_internal } from './std_internal.js';

export async function split(s, sep) {
    return await split_internal(s, sep);
}
export async function join(parts, sep) {
    return await join_str_internal(parts, sep);
}
export async function trim(s) {
    return await trim_internal(s);
}
export async function replace(s, pattern, replacement) {
    return await replace_internal(s, pattern, replacement);
}
export async function contains(s, sub) {
    return await contains_internal(s, sub);
}
export async function startsWith(s, prefix) {
    return await starts_with_internal(s, prefix);
}
export async function endsWith(s, suffix) {
    return await ends_with_internal(s, suffix);
}
export async function toLower(s) {
    return await to_lower_internal(s);
}
export async function toUpper(s) {
    return await to_upper_internal(s);
}


if (typeof main === 'function') { await main(); }
