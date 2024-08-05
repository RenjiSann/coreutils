use super::{
    ChecksumError, EvaluationError, HashAlgorithm, ALGORITHM_OPTIONS_BLAKE2B,
    ALGORITHM_OPTIONS_BLAKE3, ALGORITHM_OPTIONS_BSD, ALGORITHM_OPTIONS_CRC, ALGORITHM_OPTIONS_MD5,
    ALGORITHM_OPTIONS_SHA1, ALGORITHM_OPTIONS_SHA224, ALGORITHM_OPTIONS_SHA256,
    ALGORITHM_OPTIONS_SHA384, ALGORITHM_OPTIONS_SHA512, ALGORITHM_OPTIONS_SHAKE128,
    ALGORITHM_OPTIONS_SHAKE256, ALGORITHM_OPTIONS_SM3, ALGORITHM_OPTIONS_SYSV,
    SUPPORTED_ALGORITHMS,
};
use crate::{
    error::{UResult, USimpleError},
    sum::{
        Blake2b, Blake3, Digest, Md5, Sha1, Sha224, Sha256, Sha384, Sha3_224, Sha3_256, Sha3_384,
        Sha3_512, Sha512, Shake128, Shake256, Sm3, BSD, CRC, SYSV,
    },
};

#[derive(Debug, Default)]
pub struct ChecksumAlgoBuilder {
    /// The name of the CLI `--algo` if provided
    cli_algo_argument: Option<String>,
    algo: Option<String>,
    length: Option<usize>,
}

impl ChecksumAlgoBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn maybe_cli_algo_name<S: Into<String>>(mut self, algo: Option<S>) -> Self {
        self.cli_algo_argument = algo.map(S::into);
        self
    }

    pub fn maybe_algo_name<S: Into<String>>(mut self, algo: Option<S>) -> Self {
        self.algo = algo.map(S::into);
        self
    }

    pub fn algo_name<S: Into<String>>(mut self, algo: S) -> Self {
        self.algo = Some(algo.into());
        self
    }

    pub fn maybe_algo_length(mut self, length: Option<usize>) -> Self {
        self.length = length;
        self
    }

    pub fn algo_length(mut self, length: usize) -> Self {
        self.length = Some(length);
        self
    }

    pub fn try_build(&self) -> Result<HashAlgorithm, EvaluationError> {
        let Some(name) = self.algo.clone() else {
            // No algo name was found.
            return Err(EvaluationError::ChecksumError(Box::new(
                ChecksumError::NeedAlgorithmToHash,
            )));
        };
        if self
            .cli_algo_argument
            .as_ref()
            .is_some_and(|cli| *cli != name)
        {
            // Provided algorithm conflicts with the algorithm given in CLI.
            return Err(EvaluationError::ImproperlyFormattedLine);
        }

        if !SUPPORTED_ALGORITHMS.contains(&name.as_str()) {
            // Not supported algo, leave early without failing.
            return Err(EvaluationError::Skipped);
        }

        match name.as_str() {
            ALGORITHM_OPTIONS_SYSV => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SYSV,
                create_fn: Box::new(|| Box::new(SYSV::new())),
                bits: 512,
            }),
            ALGORITHM_OPTIONS_BSD => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_BSD,
                create_fn: Box::new(|| Box::new(BSD::new())),
                bits: 1024,
            }),
            ALGORITHM_OPTIONS_CRC => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_CRC,
                create_fn: Box::new(|| Box::new(CRC::new())),
                bits: 256,
            }),
            ALGORITHM_OPTIONS_MD5 | "md5sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_MD5,
                create_fn: Box::new(|| Box::new(Md5::new())),
                bits: 128,
            }),
            ALGORITHM_OPTIONS_SHA1 | "sha1sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHA1,
                create_fn: Box::new(|| Box::new(Sha1::new())),
                bits: 160,
            }),
            ALGORITHM_OPTIONS_SHA224 | "sha224sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHA224,
                create_fn: Box::new(|| Box::new(Sha224::new())),
                bits: 224,
            }),
            ALGORITHM_OPTIONS_SHA256 | "sha256sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHA256,
                create_fn: Box::new(|| Box::new(Sha256::new())),
                bits: 256,
            }),
            ALGORITHM_OPTIONS_SHA384 | "sha384sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHA384,
                create_fn: Box::new(|| Box::new(Sha384::new())),
                bits: 384,
            }),
            ALGORITHM_OPTIONS_SHA512 | "sha512sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SHA512,
                create_fn: Box::new(|| Box::new(Sha512::new())),
                bits: 512,
            }),
            ALGORITHM_OPTIONS_BLAKE2B | "b2sum" => {
                // Set default length to 512 if None
                let bits = self.length.unwrap_or(512);
                if bits == 512 {
                    Ok(HashAlgorithm {
                        name: ALGORITHM_OPTIONS_BLAKE2B,
                        create_fn: Box::new(move || Box::new(Blake2b::new())),
                        bits: 512,
                    })
                } else {
                    Ok(HashAlgorithm {
                        name: ALGORITHM_OPTIONS_BLAKE2B,
                        create_fn: Box::new(move || Box::new(Blake2b::with_output_bytes(bits))),
                        bits,
                    })
                }
            }
            ALGORITHM_OPTIONS_BLAKE3 | "b3sum" => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_BLAKE3,
                create_fn: Box::new(|| Box::new(Blake3::new())),
                bits: 256,
            }),
            ALGORITHM_OPTIONS_SM3 => Ok(HashAlgorithm {
                name: ALGORITHM_OPTIONS_SM3,
                create_fn: Box::new(|| Box::new(Sm3::new())),
                bits: 512,
            }),
            ALGORITHM_OPTIONS_SHAKE128 | "shake128sum" => {
                let bits = self
                    .length
                    .ok_or_else(|| USimpleError::new(1, "--bits required for SHAKE128"))
                    .map_err(|e| EvaluationError::ChecksumError(e))?;
                Ok(HashAlgorithm {
                    name: ALGORITHM_OPTIONS_SHAKE128,
                    create_fn: Box::new(|| Box::new(Shake128::new())),
                    bits,
                })
            }
            ALGORITHM_OPTIONS_SHAKE256 | "shake256sum" => {
                let bits = self
                    .length
                    .ok_or_else(|| USimpleError::new(1, "--bits required for SHAKE256"))
                    .map_err(|e| EvaluationError::ChecksumError(e))?;
                Ok(HashAlgorithm {
                    name: ALGORITHM_OPTIONS_SHAKE256,
                    create_fn: Box::new(|| Box::new(Shake256::new())),
                    bits,
                })
            }
            //ALGORITHM_OPTIONS_SHA3 | "sha3" => (
            _ if name.starts_with("sha3") => {
                create_sha3(self.length).map_err(|e| EvaluationError::ChecksumError(e))
            }

            _ => Err(EvaluationError::ChecksumError(
                ChecksumError::UnknownAlgorithm.into(),
            )),
        }
    }
}

/// Creates a SHA3 hasher instance based on the specified bits argument.
///
/// # Returns
///
/// Returns a UResult of a tuple containing the algorithm name, the hasher instance, and
/// the output length in bits or an Err if an unsupported output size is provided, or if
/// the `--bits` flag is missing.
pub fn create_sha3(bits: Option<usize>) -> UResult<HashAlgorithm> {
    match bits {
        Some(224) => Ok(HashAlgorithm {
            name: "SHA3_224",
            create_fn: Box::new(|| Box::new(Sha3_224::new())),
            bits: 224,
        }),
        Some(256) => Ok(HashAlgorithm {
            name: "SHA3_256",
            create_fn: Box::new(|| Box::new(Sha3_256::new())),
            bits: 256,
        }),
        Some(384) => Ok(HashAlgorithm {
            name: "SHA3_384",
            create_fn: Box::new(|| Box::new(Sha3_384::new())),
            bits: 384,
        }),
        Some(512) => Ok(HashAlgorithm {
            name: "SHA3_512",
            create_fn: Box::new(|| Box::new(Sha3_512::new())),
            bits: 512,
        }),

        Some(_) => Err(ChecksumError::InvalidOutputSizeForSha3.into()),
        None => Err(ChecksumError::BitsRequiredForSha3.into()),
    }
}
