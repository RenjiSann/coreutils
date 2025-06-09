use std::{cmp::Ordering, sync::OnceLock};

use icu_collator::{CollatorBorrowed, options::CollatorOptions};
use icu_locale::{Locale, locale};
use thiserror::Error;

use crate::error::UError;

#[derive(Error, Debug)]
pub enum CollationError {
    #[error("test")]
    Test,
}
impl UError for CollationError {
    fn code(&self) -> i32 {
        1
    }
}

const DEFAULT_LOCALE: Locale = locale!("en-US-posix");

/// Deduce the locale from the current environment
fn get_collating_locale() -> &'static Locale {
    static COLLATING_LOCALE: OnceLock<Locale> = OnceLock::new();

    COLLATING_LOCALE.get_or_init(|| {
        // Look at 3 environment variables in the following order
        //
        // 1. LC_ALL
        // 2. LC_COLLATE
        // 3. LANG
        //
        // Or fallback on Posix locale

        let locale_var = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_COLLATE"))
            .or_else(|_| std::env::var("LANG"));

        if let Ok(locale) = locale_var {
            if let Some(simple) = locale.split(&['-', '@']).next() {
                let bcp47 = simple.replace("_", "-");
                if let Ok(locale) = Locale::try_from_str(&bcp47) {
                    return locale;
                }
            }
        }
        // Default POSIX locale representing LC_ALL=C
        DEFAULT_LOCALE
    })
}

static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();

pub fn initialize_collator() -> Result<(), CollationError> {
    let options = CollatorOptions::default();

    // TODO: Use `get_or_try_init` when stabilized
    COLLATOR.get_or_init(|| {
        CollatorBorrowed::try_new(get_collating_locale().into(), options)
            .expect("should never fail while we use compiled data")
    });
    Ok(())
}

pub fn locale_compare<T: AsRef<[u8]>>(left: T, right: T) -> Ordering {
    if get_collating_locale() == &DEFAULT_LOCALE {
        // If no collator can be found from the locale env, use simple string comparison
        left.as_ref().cmp(right.as_ref())
    } else if let Some(collator) = COLLATOR.get() {
        collator.compare_utf8(left.as_ref(), right.as_ref())
    } else {
        panic!("COLLATOR should be initialized first")
    }
}
