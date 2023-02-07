mod precomputed_shares;

#[generic_tests::define(attrs(tokio::test, test_case::case))]
mod generic {
    use generic_ec::{hash_to_curve::FromHash, Point};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use round_based::simulation::Simulation;
    use sha2::Sha256;

    use cggmp21::key_share::Valid;
    use cggmp21::{security_level::ReasonablySecure, ExecutionId};

    use super::precomputed_shares::CACHED_SHARES;

    #[test_case::case(5; "n3")]
    #[test_case::case(10; "n10")]
    #[tokio::test]
    async fn key_refresh_works<E: generic_ec::Curve>(n: u16)
    where
        generic_ec::Scalar<E>: FromHash,
        Point<E>: generic_ec::coords::HasAffineX<E>,
    {
        let mut rng = rand_dev::DevRng::new();

        let shares = CACHED_SHARES
            .get_shares::<E>(n)
            .expect("retrieve cached shares");

        // Perform refresh

        let refresh_execution_id: [u8; 32] = rng.gen();
        let refresh_execution_id =
            ExecutionId::<E, ReasonablySecure>::from_bytes(&refresh_execution_id);
        let mut simulation = Simulation::<cggmp21::key_refresh::Msg<E, Sha256>>::new();
        let outputs = shares.into_iter().map(|share| {
            let party = simulation.add_party();
            let refresh_execution_id = refresh_execution_id.clone();
            let mut party_rng = ChaCha20Rng::from_seed(rng.gen());
            async move {
                cggmp21::key_refresh(share)
                    .set_execution_id(refresh_execution_id)
                    .start(&mut party_rng, party)
                    .await
            }
        });

        let key_shares = futures::future::try_join_all(outputs)
            .await
            .expect("keygen failed");

        // validate key shares

        for (i, key_share) in key_shares.iter().enumerate() {
            let i = i as u16;
            assert_eq!(i, key_share.core.i);
            assert_eq!(
                key_share.core.shared_public_key,
                key_shares[0].core.shared_public_key
            );
            assert_eq!(key_share.core.rid.as_ref(), key_shares[0].core.rid.as_ref());
            assert_eq!(
                key_share.core.public_shares,
                key_shares[0].core.public_shares
            );
            assert_eq!(
                Point::<E>::generator() * &key_share.core.x,
                key_share.core.public_shares[usize::from(i)]
            );
        }
        assert_eq!(
            key_shares[0].core.shared_public_key,
            key_shares[0].core.public_shares.iter().sum::<Point<E>>()
        );
        let _key_shares = key_shares
            .into_iter()
            .map(|s| s.try_into().expect("Key share did not validate"))
            .collect::<Vec<Valid<_>>>();
    }

    #[instantiate_tests(<generic_ec::curves::Secp256r1>)]
    mod secp256r1 {}
}