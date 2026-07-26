import { uuid_internal, sha256_internal } from './std_internal.js';

export async function uuid() {
    return await uuid_internal();
}
export async function sha256(data) {
    return await sha256_internal(data);
}


if (typeof main === 'function') { await main(); }
