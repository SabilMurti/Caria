// Calesteria standard library runtime helpers

// math.caria
export function abs_internal(x) {
    return Math.abs(x);
}
export function min_internal(x, y) {
    return Math.min(x, y);
}
export function max_internal(x, y) {
    return Math.max(x, y);
}
export function round_internal(x) {
    return Math.round(x);
}
export function random_internal() {
    return Math.random();
}

// io.caria
import * as fs from 'fs';
export function read_text_internal(path) {
    try {
        const content = fs.readFileSync(path, 'utf8');
        return { tag: 'Ok', value: content };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}
export function write_text_internal(path, content) {
    try {
        fs.writeFileSync(path, content, 'utf8');
        return { tag: 'Ok', value: null };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}
export function exists_internal(path) {
    return fs.existsSync(path);
}

// net.caria
export async function fetch_internal(url) {
    const res = await fetch(url);
    return await res.text();
}

// json.caria
export function stringify_internal(val) {
    return JSON.stringify(val);
}
export function parse_internal(s) {
    try {
        return JSON.parse(s);
    } catch (e) {
        return null;
    }
}

// time.caria
export function now_internal() {
    return Date.now();
}
export function sleep_internal(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// crypto.caria
import { randomUUID, createHash } from 'crypto';
export function uuid_internal() {
    return randomUUID();
}
export function sha256_internal(data) {
    return createHash('sha256').update(data).digest('hex');
}

// env.caria
export function get_var_internal(name) {
    return process.env[name] || null;
}

// path.caria
import * as path from 'path';
export function join_internal(a, b) {
    return path.join(a, b);
}

// core.caria
export function parse_int_internal(s) {
    const val = parseInt(s, 10);
    return isNaN(val) ? null : val;
}
export function parse_float_internal(s) {
    const val = parseFloat(s);
    return isNaN(val) ? null : val;
}
export function str_internal(val) {
    return String(val);
}
// string.caria helpers
export function split_internal(s, sep) {
    return s.split(sep);
}
export function join_str_internal(parts, sep) {
    return parts.join(sep);
}
export function trim_internal(s) {
    return s.trim();
}
export function replace_internal(s, pattern, replacement) {
    return s.replace(pattern, replacement);
}
export function contains_internal(s, sub) {
    return s.includes(sub);
}
export function starts_with_internal(s, prefix) {
    return s.startsWith(prefix);
}
export function ends_with_internal(s, suffix) {
    return s.endsWith(suffix);
}
export function to_lower_internal(s) {
    return s.toLowerCase();
}
export function to_upper_internal(s) {
    return s.toUpperCase();
}

// array.caria helpers
export function map_internal(list, cb) {
    return list.map(cb);
}
export function filter_internal(list, cb) {
    return list.filter(cb);
}
export function find_internal(list, cb) {
    return list.find(cb) || null;
}
export function push_internal(list, item) {
    return [...list, item];
}
export function concat_internal(a, b) {
    return a.concat(b);
}
export function slice_internal(list, start, end) {
    return list.slice(start, end);
}
export function includes_internal(list, val) {
    return list.includes(val);
}

// hash.caria helpers
export function keys_internal(map) {
    return Object.keys(map);
}
export function values_internal(map) {
    return Object.values(map);
}
export function entries_internal(map) {
    return Object.entries(map);
}
export function has_key_internal(map, key) {
    return key in map;
}
export function get_or_default_internal(map, key, default_val) {
    return map[key] !== undefined ? map[key] : default_val;
}

// regex.caria helpers
export function regex_test_internal(pattern, s) {
    return new RegExp(pattern).test(s);
}
export function regex_match_internal(pattern, s) {
    const m = s.match(new RegExp(pattern));
    return m ? Array.from(m) : null;
}
export function regex_replace_all_internal(pattern, s, replacement) {
    return s.replace(new RegExp(pattern, 'g'), replacement);
}

// url.caria helpers
export function url_encode_internal(s) {
    return encodeURIComponent(s);
}
export function url_decode_internal(s) {
    return decodeURIComponent(s);
}
export function url_parse_query_internal(query_str) {
    const params = new URLSearchParams(query_str);
    const obj = {};
    for (const [k, v] of params.entries()) {
        obj[k] = v;
    }
    return obj;
}

// fs.caria helpers
export function read_dir_internal(path) {
    try {
        return { tag: 'Ok', value: fs.readdirSync(path) };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}
export function create_dir_internal(path) {
    try {
        fs.mkdirSync(path, { recursive: true });
        return { tag: 'Ok', value: null };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}
export function remove_file_internal(path) {
    try {
        fs.unlinkSync(path);
        return { tag: 'Ok', value: null };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}
export function remove_dir_internal(path) {
    try {
        fs.rmSync(path, { recursive: true, force: true });
        return { tag: 'Ok', value: null };
    } catch (e) {
        return { tag: 'Err', value: e.message };
    }
}


