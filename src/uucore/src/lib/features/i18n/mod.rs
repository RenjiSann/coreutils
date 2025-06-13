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

/// The encoding specified by the locale, if specified
/// Currently only supports UTF-8 for the sake of simplicity.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UEncoding {
    Utf8,
}

const DEFAULT_LOCALE: Locale = locale!("en-US-posix");

/// Deduce the locale from the current environment
fn get_collating_locale() -> &'static (Locale, Option<UEncoding>) {
    static COLLATING_LOCALE: OnceLock<(Locale, Option<UEncoding>)> = OnceLock::new();

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

        if let Ok(locale_var_str) = locale_var {
            let mut split = locale_var_str.split(&['.', '@']);

            if let Some(simple) = split.next() {
                let bcp47 = simple.replace("_", "-");
                let locale = Locale::try_from_str(&bcp47).unwrap_or(DEFAULT_LOCALE);

                // If locale parsing failed, parse the encoding part of the
                // locale. Treat the special case of the given locale being "C"
                // which becomes the default locale.
                let encoding = if (locale != DEFAULT_LOCALE || bcp47 == "C")
                    && split.next() == Some("UTF-8")
                {
                    Some(UEncoding::Utf8)
                } else {
                    None
                };
                return (locale, encoding);
            } else {
                return (DEFAULT_LOCALE, None);
            };
        }
        // Default POSIX locale representing LC_ALL=C
        (DEFAULT_LOCALE, None)
    })
}

pub fn get_locale_encoding() -> Option<UEncoding> {
    get_collating_locale().1
}

static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();

pub fn initialize_collator() -> Result<(), CollationError> {
    let options = CollatorOptions::default();

    // TODO: Use `get_or_try_init` when stabilized
    COLLATOR.get_or_init(|| {
        CollatorBorrowed::try_new((&get_collating_locale().0).into(), options)
            .expect("should never fail while we use compiled data")
    });
    Ok(())
}

pub fn locale_compare<T: AsRef<[u8]>>(left: T, right: T) -> Ordering {
    if get_collating_locale().0 == DEFAULT_LOCALE {
        // If no collator can be found from the locale env, use simple string comparison
        left.as_ref().cmp(right.as_ref())
    } else if let Some(collator) = COLLATOR.get() {
        collator.compare_utf8(left.as_ref(), right.as_ref())
    } else {
        panic!("COLLATOR should be initialized first")
    }
}
