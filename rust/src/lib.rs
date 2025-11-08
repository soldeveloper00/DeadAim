// src/lib.rs
use rand::Rng;
use std::f32;
use std::slice;

use wasm_bindgen::prelude::*;

// When compiled to wasm, enable console logging if you want
#[cfg(feature = "wasm")]
extern crate console_error_panic_hook;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub alive: bool,
}

// ---------- WASM / JS interop hooks (frontend must provide these) ----------
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    // Implement in JS: send token/reward to player's wallet (amount in smallest unit)
    #[wasm_bindgen(js_name = js_send_token)]
    fn js_send_token(wallet: &str, amount: u64);

    // Implement in JS: mint an NFT to wallet with provided metadata (JSON or URI)
    #[wasm_bindgen(js_name = js_mint_nft)]
    fn js_mint_nft(wallet: &str, metadata: &str);

    // Optional: log helper in JS
    #[wasm_bindgen(js_name = js_log)]
    fn js_log(s: &str);
}

#[cfg(not(target_arch = "wasm32"))]
mod native_stubs {
    // For native builds we provide stub implementations that can be replaced
    pub fn js_send_token(_wallet: &str, _amount: u64) {
        // native stub: you can replace with RPC client or FFI to wallet
        println!("(native stub) send_token called but not implemented");
    }
    pub fn js_mint_nft(_wallet: &str, _metadata: &str) {
        println!("(native stub) mint_nft called but not implemented");
    }
    pub fn js_log(s: &str) {
        println!("(native stub) {}", s);
    }
}
#[cfg(not(target_arch = "wasm32"))]
use native_stubs::{js_log, js_mint_nft, js_send_token};

// ---------- Initialization ----------
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn init() {
    // Better error messages in wasm
    console_error_panic_hook::set_once();
    // optional console log
    js_log("WASM module 'deadaim_rust' initialized");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init() {
    // native init
    js_log("Native module 'deadaim_rust' initialized");
}

// ---------- Core functions exposed to C++ (via pointer interfaces) ----------
// Note: C++ expects pointers to Enemy; we accept *const Enemy / *mut Enemy and count.

/// Find nearest alive enemy; returns index (0-based) or -1 if none.
/// Safe C ABI wrapper compatible with C++ (use with raw pointers).
#[no_mangle]
pub extern "C" fn find_nearest_enemy(
    player_x: f32,
    player_y: f32,
    enemies_ptr: *const Enemy,
    count: i32,
) -> i32 {
    // Safety: caller must ensure pointer + count is valid
    if enemies_ptr.is_null() || count <= 0 {
        return -1;
    }
    let enemies = unsafe { slice::from_raw_parts(enemies_ptr, count as usize) };

    let mut nearest_index: i32 = -1;
    let mut min_dist2: f32 = f32::MAX;

    for (i, e) in enemies.iter().enumerate() {
        if !e.alive {
            continue;
        }
        let dx = player_x - e.x;
        let dy = player_y - e.y;
        let dist2 = dx * dx + dy * dy;
        if dist2 < min_dist2 {
            min_dist2 = dist2;
            nearest_index = i as i32;
        }
    }

    nearest_index
}

/// Shoot enemy at index => mark alive = false
#[no_mangle]
pub extern "C" fn shoot_enemy(index: i32, enemies_ptr: *mut Enemy) {
    if enemies_ptr.is_null() || index < 0 {
        return;
    }
    unsafe {
        let e_ptr = enemies_ptr.offset(index as isize);
        (*e_ptr).alive = false;
    }
}

/// Move enemies randomly. `speed` is max delta per call.
#[no_mangle]
pub extern "C" fn move_enemies_randomly(
    enemies_ptr: *mut Enemy,
    count: i32,
    speed: f32,
) {
    if enemies_ptr.is_null() || count <= 0 || speed <= 0.0 {
        return;
    }
    let enemies = unsafe { slice::from_raw_parts_mut(enemies_ptr, count as usize) };
    let mut rng = rand::thread_rng();

    for e in enemies.iter_mut() {
        if e.alive {
            // small random walk
            let dx: f32 = rng.gen_range(-speed..speed);
            let dy: f32 = rng.gen_range(-speed..speed);
            e.x += dx;
            e.y += dy;
            // clamp to reasonable bounds (e.g., grid 0..=GRID_SIZE-1). caller can clamp as well.
            if e.x.is_nan() || e.y.is_nan() {
                e.x = 0.0;
                e.y = 0.0;
            }
        }
    }
}

// ---------- Reward hooks (call frontend to perform actual blockchain ops) ----------

/// Reward player with fungible token amount (smallest unit). Frontend must implement js_send_token.
/// `wallet` is a null-terminated C string pointer expected from caller; to simplify from C++,
/// you can call this from the WASM/js layer. For native builds this is a stub.
#[no_mangle]
pub extern "C" fn reward_player(wallet_ptr: *const u8, wallet_len: usize, amount: u64) {
    if wallet_ptr.is_null() || wallet_len == 0 {
        js_log("reward_player: invalid wallet pointer/len");
        return;
    }
    // Convert C-style pointer+len to &str
    let wallet_slice = unsafe { std::slice::from_raw_parts(wallet_ptr, wallet_len) };
    if let Ok(wallet_str) = std::str::from_utf8(wallet_slice) {
        // call JS/native hook
        js_send_token(wallet_str, amount);
        js_log(&format!("reward_player: sent {} to {}", amount, wallet_str));
    } else {
        js_log("reward_player: wallet string not utf-8");
    }
}

/// Mint an NFT for a player: frontend must implement js_mint_nft(wallet, metadata)
#[no_mangle]
pub extern "C" fn mint_nft_for_player(wallet_ptr: *const u8, wallet_len: usize, meta_ptr: *const u8, meta_len: usize) {
    if wallet_ptr.is_null() || wallet_len == 0 {
        js_log("mint_nft_for_player: invalid wallet pointer");
        return;
    }
    let wallet_slice = unsafe { std::slice::from_raw_parts(wallet_ptr, wallet_len) };
    let meta_slice = unsafe { std::slice::from_raw_parts(meta_ptr, meta_len) };

    if let (Ok(wallet_str), Ok(meta_str)) = (std::str::from_utf8(wallet_slice), std::str::from_utf8(meta_slice)) {
        js_mint_nft(wallet_str, meta_str);
        js_log(&format!("mint_nft_for_player: minted for {} metadata={}", wallet_str, meta_str));
    } else {
        js_log("mint_nft_for_player: utf-8 conversion failed");
    }
}

// ---------- Convenience helpers for WASM/JS usage (optional) ----------
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_find_nearest_enemy(player_x: f32, player_y: f32, enemies: &JsValue) -> i32 {
    // This helper expects `enemies` as a JS TypedArray or array of objects matching Enemy layout.
    // For simplicity, we expect the JS side to manage marshalling. Here we'll just log.
    js_log("wasm_find_nearest_enemy called - prefer using pointer-based FFI from native C++");
    -1
}