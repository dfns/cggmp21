use anyhow::{bail, Context, Result};
use cggmp21::{key_share::KeyShare, rug::Integer, security_level::SecurityLevel};
use generic_ec::Curve;
use rand::RngCore;
use serde_json::{Map, Value};

pub mod external_verifier;

lazy_static::lazy_static! {
    pub static ref CACHED_SHARES: PrecomputedKeyShares =
        PrecomputedKeyShares::from_serialized(
            include_str!("../../test-data/precomputed_shares.json")
        ).unwrap();
    pub static ref CACHED_PRIMES: PregeneratedPrimes =
        PregeneratedPrimes::from_serialized(
            include_str!("../../test-data/pregenerated_primes.json")
        ).unwrap();
}

pub struct PrecomputedKeyShares {
    shares: Map<String, Value>,
}

impl PrecomputedKeyShares {
    pub fn empty() -> Self {
        Self {
            shares: Default::default(),
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_serialized(shares: &str) -> Result<Self> {
        let shares = serde_json::from_str(shares).context("parse shares")?;
        Ok(Self { shares })
    }

    pub fn to_serialized(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.shares).context("serialize shares")
    }

    pub fn get_shares<E: Curve, L: SecurityLevel>(
        &self,
        t: Option<u16>,
        n: u16,
    ) -> Result<Vec<KeyShare<E, L>>> {
        let key_shares = self
            .shares
            .get(&format!("t={t:?},n={n},curve={}", E::CURVE_NAME))
            .context("shares not found")?;
        serde_json::from_value(key_shares.clone()).context("parse key shares")
    }

    pub fn add_shares<E: Curve, L: SecurityLevel>(
        &mut self,
        t: Option<u16>,
        n: u16,
        shares: &[KeyShare<E, L>],
    ) -> Result<()> {
        if usize::from(n) != shares.len() {
            bail!("expected {n} key shares, only {} provided", shares.len());
        }
        let key_shares = serde_json::to_value(shares).context("serialize shares")?;
        self.shares
            .insert(format!("t={t:?},n={n},curve={}", E::CURVE_NAME), key_shares);
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PregeneratedPrimes {
    // It would be better to use key_refresh::PregeneratedPrimes here, but
    // adding serialization to that is an enormous pain in the ass
    primes: Vec<Integer>,
    bitsize: u32,
}

impl PregeneratedPrimes {
    pub fn from_serialized(repr: &str) -> Result<Self> {
        serde_json::from_str(repr).context("parse primes")
    }

    pub fn to_serialized(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("serialize primes")
    }

    /// Iterate over numbers, producing pregenerated pairs for key refresh
    pub fn iter<L>(&self) -> impl Iterator<Item = cggmp21::key_refresh::PregeneratedPrimes<L>> + '_
    where
        L: cggmp21::security_level::SecurityLevel,
    {
        if self.bitsize != 4 * L::SECURITY_BITS {
            panic!("Attempting to use generated primes while expecting wrong bit size");
        }
        self.primes.chunks(2).map(|s| {
            let p = &s[0];
            let q = &s[1];
            cggmp21::key_refresh::PregeneratedPrimes::new(p.clone(), q.clone())
                .expect("primes have wrong bit size")
        })
    }

    /// Generate enough primes so that you can do `amount` of key refreshes
    pub fn generate<R, L>(amount: usize, rng: &mut R) -> Self
    where
        L: cggmp21::security_level::SecurityLevel,
        R: RngCore,
    {
        let bitsize = 4 * L::SECURITY_BITS;
        let primes = (0..amount)
            .flat_map(|_| {
                let p = generate_blum_prime(rng, bitsize);
                let q = generate_blum_prime(rng, bitsize);
                [p, q]
            })
            .collect();

        Self { primes, bitsize }
    }
}

/// Generates a blum prime
///
/// CGGMP21 requires using safe primes, however blum primes do not break correctness of the protocol
/// and they can be generated faster.
///
/// Only to be used in the tests.
pub fn generate_blum_prime(rng: &mut impl rand::RngCore, bits_size: u32) -> Integer {
    loop {
        let mut n: Integer = Integer::random_bits(
            bits_size,
            &mut cggmp21::fast_paillier::utils::external_rand(rng),
        )
        .into();
        n.set_bit(bits_size - 1, true);
        n.next_prime_mut();
        if n.mod_u(4) == 3 {
            break n;
        }
    }
}
