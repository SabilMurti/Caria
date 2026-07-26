import { get_var_internal } from './std_internal.js';

export async function getVar(name) {
    return await get_var_internal(name);
}


if (typeof main === 'function') { await main(); }
