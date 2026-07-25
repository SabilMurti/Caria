
import { debounce } from 'lodash-es';

import { calculate } from './std/math.js';

async function loadData() {
    const raw = await readFile(`config.toml`);
    const options = { "timeout": 3000, "retries": 3 };
    const items = [1, 2, 3, 4];
    const safeName = user?.profile?.name ?? `Guest`;
    console.log(safeName);
    return raw;
}


if (typeof main === 'function') { main(); }