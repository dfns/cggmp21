#![allow(
    non_snake_case,
    mixed_script_confusables,
    uncommon_codepoints,
    clippy::too_many_arguments,
    clippy::nonminimal_bool
)]
#![forbid(clippy::disallowed_methods)]
#![cfg_attr(not(test), forbid(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use {
    generic_ec, paillier_zk, paillier_zk::libpaillier, paillier_zk::libpaillier::unknown_order,
    round_based,
};

use generic_ec::{coords::HasAffineX, hash_to_curve::FromHash, Curve, Point, Scalar};
use key_share::AnyKeyShare;
use round_based::PartyIndex;
use security_level::SecurityLevel;
use signing::SigningBuilder;

mod errors;
mod execution_id;
pub mod key_refresh;
pub mod key_share;
pub mod keygen;
pub mod progress;
pub mod security_level;
pub mod signing;
pub mod supported_curves;
mod utils;
mod zk;

#[cfg(feature = "spof")]
pub mod trusted_dealer;

/// Defines default choice for digest and security level used across the crate
mod default_choice {
    pub type Digest = sha2::Sha256;
    pub type SecurityLevel = crate::security_level::ReasonablySecure;
}

pub use self::execution_id::ExecutionId;
pub use self::{
    key_refresh::KeyRefreshError,
    key_share::{IncompleteKeyShare, KeyShare},
    keygen::KeygenError,
    signing::{DataToSign, PartialSignature, Presignature, Signature, SigningError},
};

/// Distributed key generation protocol
///
/// Instantiates [KeygenBuilder] with [ReasonablySecure] security level
/// and sha2-256 digest. You can switch to threshold DKG by using
/// [`set_threshold`]
///
/// [KeygenBuilder]: keygen::KeygenBuilder
/// [ReasonablySecure]: security_level::ReasonablySecure
/// [`set_threshold`]: keygen::GenericKeygenBuilder::set_threshold
pub fn keygen<E>(i: u16, n: u16) -> keygen::KeygenBuilder<E>
where
    E: Curve,
    Scalar<E>: FromHash,
{
    keygen::KeygenBuilder::new(i, n)
}

/// Protocol for finalizing the keygen by generating aux info.
///
/// PregeneratedPrimes can be obtained with [`key_refresh::PregeneratedPrimes::generate`]
///
/// Index `i` of party should be the same as index inside the keyshare you are
/// going to use this aux info with
pub fn aux_info_gen<'a, E, L>(
    i: u16,
    n: u16,
    pregenerated: key_refresh::PregeneratedPrimes<L>,
) -> key_refresh::AuxInfoGenerationBuilder<'a, E, L>
where
    E: Curve,
    L: SecurityLevel,
{
    key_refresh::GenericKeyRefreshBuilder::new_aux_gen(i, n, pregenerated)
}

/// Protocol for performing key refresh. Can be used to perform initial refresh
/// with aux info generation, or for a refersh of a complete keyshare.
///
/// Doesn't work with threshold key shares at this point.
///
/// PregeneratedPrimes can be obtained with [`key_refresh::PregeneratedPrimes::generate`]
pub fn key_refresh<E, L>(
    key_share: &impl AnyKeyShare<E>,
    pregenerated: key_refresh::PregeneratedPrimes<L>,
) -> key_refresh::KeyRefreshBuilder<E, L>
where
    E: Curve,
    L: SecurityLevel,
{
    key_refresh::KeyRefreshBuilder::new(key_share, pregenerated)
}

pub fn signing<'r, E>(
    i: PartyIndex,
    parties_indexes_at_keygen: &'r [PartyIndex],
    key_share: &'r KeyShare<E>,
) -> SigningBuilder<'r, E>
where
    E: Curve,
    Point<E>: HasAffineX<E>,
    Scalar<E>: FromHash,
{
    SigningBuilder::new(i, parties_indexes_at_keygen, key_share)
}

#[cfg(test)]
mod tests {
    use digest::Digest;
    use generic_ec::Curve;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::security_level::SecurityLevel;

    macro_rules! ensure_certain_types_impl_serde {
        ($($type:ty),+,) => {
            fn impls_serde<T: Serialize + DeserializeOwned>() {}

            #[allow(dead_code)]
            fn ensure_types_impl_serde<E: Curve, L: SecurityLevel, D: Digest>() {$(
                impls_serde::<$type>();
            )+}
        }
    }

    ensure_certain_types_impl_serde! {
        crate::key_share::KeyShare<E>,
        crate::key_share::IncompleteKeyShare<E>,
        crate::key_share::AuxInfo,

        crate::key_share::DirtyKeyShare<E>,
        crate::key_share::DirtyIncompleteKeyShare<E>,
        crate::key_share::DirtyAuxInfo,

        crate::keygen::msg::non_threshold::Msg<E, L, D>,
        crate::keygen::msg::threshold::Msg<E, L, D>,

        crate::key_refresh::msg::aux_only::Msg<D, L>,
        crate::key_refresh::msg::non_threshold::Msg<E, D, L>,

        crate::signing::msg::Msg<E, D>,
        crate::signing::Presignature<E>,
        crate::signing::PartialSignature<E>,
        crate::signing::Signature<E>,
    }
}
