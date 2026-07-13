//! Host functions for ICU (Internationalization) support via ICU4X.
//!
//! These functions are called from generated Zig code via C ABI.
//! They implement locale-sensitive String methods:
//! - localeCompare
//! - normalize (NFC/NFD/NFKC/NFKD)
//! - toLocaleUpperCase
//! - toLocaleLowerCase

use crate::sdk::{HostStr, JsStr};

/// str.localeCompare(other) → i64 (-1, 0, or 1)
///
/// Uses ICU4X Collator with default locale for proper locale-aware comparison.
fn host_icu_locale_compare_inner(a: HostStr, b: HostStr) -> i64 {
    use icu_collator::Collator;
    use icu_collator::options::CollatorOptions;
    use icu_locale_core::locale;

    let prefs = locale!("en").into();
    let options = CollatorOptions::default();
    let collator = match Collator::try_new(prefs, options) {
        Ok(c) => c,
        Err(_) => {
            // Fallback to byte-wise comparison if collator creation fails
            return match a.as_ref().cmp(b.as_ref()) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
        }
    };

    match collator.compare(a.as_ref(), b.as_ref()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_icu_locale_compare(
    a_ptr: *const u8,
    a_len: usize,
    b_ptr: *const u8,
    b_len: usize,
) -> i64 {
    let a = unsafe { HostStr::from_raw(a_ptr, a_len) };
    let b = unsafe { HostStr::from_raw(b_ptr, b_len) };
    host_icu_locale_compare_inner(a, b)
}

/// str.normalize(form) → normalized string
///
/// Supports NFC (default), NFD, NFKC, NFKD normalization forms.
/// Uses ICU4X ComposingNormalizer for NFC/NFKC and DecomposingNormalizer for NFD/NFKD.
fn host_icu_normalize_inner(input: HostStr, form: HostStr) -> String {
    use icu_normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};

    let form_str = form.as_ref();
    match form_str {
        "NFD" => {
            let normalizer = DecomposingNormalizerBorrowed::new_nfd();
            normalizer.normalize(input.as_ref()).into_owned()
        }
        "NFKC" => {
            let normalizer = ComposingNormalizerBorrowed::new_nfkc();
            normalizer.normalize(input.as_ref()).into_owned()
        }
        "NFKD" => {
            let normalizer = DecomposingNormalizerBorrowed::new_nfkd();
            normalizer.normalize(input.as_ref()).into_owned()
        }
        _ => {
            // NFC (default)
            let normalizer = ComposingNormalizerBorrowed::new_nfc();
            normalizer.normalize(input.as_ref()).into_owned()
        }
    }
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_icu_normalize(
    input_ptr: *const u8,
    input_len: usize,
    form_ptr: *const u8,
    form_len: usize,
) -> JsStr {
    let input = unsafe { HostStr::from_raw(input_ptr, input_len) };
    let form = unsafe { HostStr::from_raw(form_ptr, form_len) };
    let result = host_icu_normalize_inner(input, form);
    if result.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result)
    }
}

/// str.toLocaleUpperCase() → uppercased string
///
/// Uses ICU4X CaseMapper for locale-aware uppercase conversion.
fn host_icu_to_locale_upper_case_inner(input: HostStr) -> String {
    use icu_casemap::CaseMapperBorrowed;
    use icu_locale_core::langid;

    let cm = CaseMapperBorrowed::new();
    cm.uppercase_to_string(input.as_ref(), &langid!("und"))
        .into_owned()
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_icu_to_locale_upper_case(
    input_ptr: *const u8,
    input_len: usize,
) -> JsStr {
    let input = unsafe { HostStr::from_raw(input_ptr, input_len) };
    let result = host_icu_to_locale_upper_case_inner(input);
    if result.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result)
    }
}

/// str.toLocaleLowerCase() → lowercased string
///
/// Uses ICU4X CaseMapper for locale-aware lowercase conversion.
fn host_icu_to_locale_lower_case_inner(input: HostStr) -> String {
    use icu_casemap::CaseMapperBorrowed;
    use icu_locale_core::langid;

    let cm = CaseMapperBorrowed::new();
    cm.lowercase_to_string(input.as_ref(), &langid!("und"))
        .into_owned()
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_icu_to_locale_lower_case(
    input_ptr: *const u8,
    input_len: usize,
) -> JsStr {
    let input = unsafe { HostStr::from_raw(input_ptr, input_len) };
    let result = host_icu_to_locale_lower_case_inner(input);
    if result.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result)
    }
}
