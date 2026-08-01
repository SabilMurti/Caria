//! Native Codegen Compiler - Cranelift JIT Integration v0.106
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Module;
use std::collections::HashMap;
use std::ptr::NonNull;

pub struct CariaCompiler {
    pub(crate) module: JITModule,
    function_cache: HashMap<String, *const u8>,
}

impl CariaCompiler {
    pub fn new() -> Result<Self, String> {
        let mut flag_builder = cranelift_codegen::settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").map_err(|e| e.to_string())?;
        
        let isa_builder = cranelift_native::builder().map_err(|msg| format!("host machine not supported: {}", msg))?;
        let isa = isa_builder.finish(cranelift_codegen::settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA finish failed: {}", e))?;
        
        let jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let mut module = JITModule::new(jit_builder);
        
        Self::register_all_symbols(&mut module)?;
        
        Ok(Self {
            module,
            function_cache: HashMap::new(),
        })
    }

    fn register_all_symbols(module: &mut Module) -> Result<(), String> {
        macro_rules! add_ffi {
            ($name:expr, $fn:path) => {{
                let ptr = NonNull::new($fn as *mut u8).ok_or_else(|| concat!("null FFI pointer for ", stringify!($name)))?;
                let n = std::ffi::CString::new($name).map_err(|e| e.to_string())?;
                module.add_export(n.as_ptr(), cranelift_module::ExternalKind::Func(ptr))?;
            }};
        }

        add_ffi!("caria_last_error", caria_runtime::ffi_aot::caria_last_error);
        add_ffi!("caria_obj_alloc", caria_runtime::ffi_aot::caria_obj_alloc);
        add_ffi!("caria_string_split", caria_runtime::ffi_aot::caria_string_split);
        add_ffi!("caria_string_trim", caria_runtime::ffi_aot::caria_string_trim);
        add_ffi!("caria_string_trim_left", caria_runtime::ffi_aot::caria_string_trim_left);
        add_ffi!("caria_string_trim_right", caria_runtime::ffi_aot::caria_string_trim_right);
        add_ffi!("caria_string_lower", caria_runtime::ffi_aot::caria_string_lower);
        add_ffi!("caria_string_upper", caria_runtime::ffi_aot::caria_string_upper);
        add_ffi!("caria_string_contains", caria_runtime::ffi_aot::caria_string_contains);
        add_ffi!("caria_string_indexof", caria_runtime::ffi_aot::caria_string_indexof);
        add_ffi!("caria_string_lastindexof", caria_runtime::ffi_aot::caria_string_lastindexof);
        add_ffi!("caria_string_substring", caria_runtime::ffi_aot::caria_string_substring);
        add_ffi!("caria_string_replace_first", caria_runtime::ffi_aot::caria_string_replace_first);
        add_ffi!("caria_string_replace_all", caria_runtime::ffi_aot::caria_string_replace_all);
        add_ffi!("caria_regex_validate", caria_runtime::ffi_aot::caria_regex_validate);
        add_ffi!("caria_regex_test", caria_runtime::ffi_aot::caria_regex_test);
        add_ffi!("caria_regex_match", caria_runtime::ffi_aot::caria_regex_match);
        add_ffi!("caria_regex_match_all", caria_runtime::ffi_aot::caria_regex_match_all);
        add_ffi!("caria_regex_split", caria_runtime::ffi_aot::caria_regex_split);
        add_ffi!("caria_regex_replace", caria_runtime::ffi_aot::caria_regex_replace);
        add_ffi!("caria_regex_escape", caria_runtime::ffi_aot::caria_regex_escape);
        add_ffi!("caria_regex_find", caria_runtime::ffi_aot::caria_regex_find);
        add_ffi!("caria_crypto_argon2_hash", caria_runtime::ffi_aot::caria_crypto_argon2_hash);
        add_ffi!("caria_crypto_argon2_verify", caria_runtime::ffi_aot::caria_crypto_argon2_verify);
        add_ffi!("caria_json_encode", caria_runtime::ffi_aot::caria_json_encode);
        add_ffi!("caria_hash_str", caria_runtime::ffi_aot::caria_hash_str);
        add_ffi!("caria_array_alloc", caria_runtime::ffi_aot::caria_array_alloc);
        add_ffi!("caria_array_free", caria_runtime::ffi_aot::caria_array_free);
        add_ffi!("caria_array_push", caria_runtime::ffi_aot::caria_array_push);
        add_ffi!("caria_mysql_connect", caria_runtime::ffi_aot::caria_mysql_connect);
        add_ffi!("caria_mysql_query", caria_runtime::ffi_aot::caria_mysql_query);
        add_ffi!("caria_sqlite_open", caria_runtime::ffi_aot::caria_sqlite_open);
        add_ffi!("caria_sqlite_query", caria_runtime::ffi_aot::caria_sqlite_query);
        add_ffi!("caria_sqlite_exec", caria_runtime::ffi_aot::caria_sqlite_exec);
        add_ffi!("caria_sqlite_prepared_exec", caria_runtime::ffi_aot::caria_sqlite_prepared_exec);
        add_ffi!("caria_sqlite_date", caria_runtime::ffi_aot::caria_sqlite_date);
        add_ffi!("caria_sqlite_close", caria_runtime::ffi_aot::caria_sqlite_close);
        add_ffi!("caria_http_get", caria_runtime::ffi_aot::caria_http_get);
        add_ffi!("caria_http_post", caria_runtime::ffi_aot::caria_http_post);
        add_ffi!("caria_http_put", caria_runtime::ffi_aot::caria_http_put);
        add_ffi!("caria_http_delete", caria_runtime::ffi_aot::caria_http_delete);
        add_ffi!("caria_http_send", caria_runtime::ffi_aot::caria_http_send);
        add_ffi!("caria_http_parse", caria_runtime::ffi_aot::caria_http_parse);
        add_ffi!("caria_http_parse_url", caria_runtime::ffi_aot::caria_http_parse_url);
        add_ffi!("caria_crypto_rsa_generate_keypair", caria_runtime::ffi_aot::caria_crypto_rsa_generate_keypair);
        add_ffi!("caria_crypto_rsa_encrypt", caria_runtime::ffi_aot::caria_crypto_rsa_encrypt);
        add_ffi!("caria_crypto_rsa_decrypt", caria_runtime::ffi_aot::caria_crypto_rsa_decrypt);
        add_ffi!("caria_crypto_ed25519_generate_keypair", caria_runtime::ffi_aot::caria_crypto_ed25519_generate_keypair);
        add_ffi!("caria_crypto_ed25519_sign", caria_runtime::ffi_aot::caria_crypto_ed25519_sign);
        add_ffi!("caria_crypto_ed25519_verify", caria_runtime::ffi_aot::caria_crypto_ed25519_verify);
        add_ffi!("caria_crypto_ed25519_public_key", caria_runtime::ffi_aot::caria_crypto_ed25519_public_key);
        add_ffi!("caria_crypto_ed25519_keypair_from_seed", caria_runtime::ffi_aot::caria_crypto_ed25519_keypair_from_seed);
        add_ffi!("caria_say_int", caria_runtime::ffi_aot::caria_say_int);
        add_ffi!("caria_say_txt", caria_runtime::ffi_aot::caria_say_txt);
        add_ffi!("caria_write_file", caria_runtime::ffi_aot::caria_write_file);
        add_ffi!("caria_run_cmd", caria_runtime::ffi_aot::caria_run_cmd);
        add_ffi!("caria_obj_set", caria_runtime::ffi_aot::caria_obj_set);
        add_ffi!("caria_obj_get", caria_runtime::ffi_aot::caria_obj_get);
        add_ffi!("caria_obj_free", caria_runtime::ffi_aot::caria_obj_free);
        add_ffi!("caria_array_get", caria_runtime::ffi_aot::caria_array_get);
        add_ffi!("caria_closure_alloc", caria_runtime::ffi_aot::caria_closure_alloc);
        add_ffi!("caria_time_now_ms", caria_runtime::ffi_aot::caria_time_now_ms);
        add_ffi!("caria_register_route", caria_runtime::ffi_aot::caria_register_route);
        add_ffi!("caria_start_server", caria_runtime::ffi_aot::caria_start_server);
        add_ffi!("caria_crypto_aes_encrypt", caria_runtime::ffi_aot::caria_crypto_aes_encrypt);
        add_ffi!("caria_crypto_aes_decrypt", caria_runtime::ffi_aot::caria_crypto_aes_decrypt);
        add_ffi!("caria_env_get", caria_runtime::ffi_aot::caria_env_get);
        add_ffi!("caria_redis_get", caria_runtime::ffi_aot::caria_redis_get);
        add_ffi!("caria_redis_set", caria_runtime::ffi_aot::caria_redis_set);
        add_ffi!("caria_log_write", caria_runtime::ffi_aot::caria_log_write);
        add_ffi!("caria_hash_make", caria_runtime::ffi_aot::caria_hash_make);
        add_ffi!("caria_hash_check", caria_runtime::ffi_aot::caria_hash_check);
        add_ffi!("caria_json_parse", caria_runtime::ffi_aot::caria_json_parse);
        add_ffi!("caria_crypto_random", caria_runtime::ffi_aot::caria_crypto_random);
        add_ffi!("caria_fs_read_binary", caria_runtime::ffi_aot::caria_fs_read_binary);
        add_ffi!("caria_crypto_random_bytes", caria_runtime::ffi_aot::caria_crypto_random_bytes);
        add_ffi!("caria_crypto_ed25519_derive", caria_runtime::ffi_aot::caria_crypto_ed25519_derive);
        add_ffi!("caria_crypto_ed25519_import_raw", caria_runtime::ffi_aot::caria_crypto_ed25519_import_raw);

        Ok(())
    }
}
