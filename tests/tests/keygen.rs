#[generic_tests::define(attrs(tokio::test, test_case::case))]
mod generic {
    use generic_ec::{Curve, Point};
    use rand::{seq::SliceRandom, Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use rand_dev::DevRng;
    use round_based::simulation::Simulation;
    use sha2::Sha256;

    use cggmp21::keygen::{NonThresholdMsg, ThresholdMsg};
    use cggmp21::{
        key_share::reconstruct_secret_key, security_level::SecurityLevel128, ExecutionId,
    };

    #[test_case::case(3, false; "n3")]
    #[test_case::case(5, false; "n5")]
    #[test_case::case(7, false; "n7")]
    #[test_case::case(10, false; "n10")]
    #[test_case::case(10, true; "n10-reliable")]
    #[tokio::test]
    async fn keygen_works<E: Curve>(n: u16, reliable_broadcast: bool) {
        let mut rng = DevRng::new();

        let mut simulation = Simulation::<NonThresholdMsg<E, SecurityLevel128, Sha256>>::new();

        let eid: [u8; 32] = rng.gen();
        let eid = ExecutionId::new(&eid);

        let mut outputs = vec![];
        for i in 0..n {
            let party = simulation.add_party();
            let mut party_rng = ChaCha20Rng::from_seed(rng.gen());

            outputs.push(async move {
                cggmp21::keygen(eid, i, n)
                    .enforce_reliable_broadcast(reliable_broadcast)
                    .start(&mut party_rng, party)
                    .await
            })
        }

        let key_shares = futures::future::try_join_all(outputs)
            .await
            .expect("keygen failed");

        for (i, key_share) in (0u16..).zip(&key_shares) {
            assert_eq!(key_share.i, i);
            assert_eq!(key_share.shared_public_key, key_shares[0].shared_public_key);
            assert_eq!(key_share.public_shares, key_shares[0].public_shares);
            assert_eq!(
                Point::<E>::generator() * &key_share.x,
                key_share.public_shares[usize::from(i)]
            );
        }
        assert_eq!(
            key_shares[0].shared_public_key,
            key_shares[0].public_shares.iter().sum::<Point<E>>()
        );
    }

    #[test_case::case(2, 3, false; "t2n3")]
    #[test_case::case(5, 7, false; "t5n7")]
    #[test_case::case(5, 7, true; "t5n7-reliable")]
    #[tokio::test]
    async fn threshold_keygen_works<E: Curve>(t: u16, n: u16, reliable_broadcast: bool) {
        let mut rng = DevRng::new();

        let mut simulation = Simulation::<ThresholdMsg<E, SecurityLevel128, Sha256>>::new();

        let eid: [u8; 32] = rng.gen();
        let eid = ExecutionId::new(&eid);

        let mut outputs = vec![];
        for i in 0..n {
            let party = simulation.add_party();
            let mut party_rng = ChaCha20Rng::from_seed(rng.gen());

            outputs.push(async move {
                cggmp21::keygen(eid, i, n)
                    .enforce_reliable_broadcast(reliable_broadcast)
                    .set_threshold(t)
                    .start(&mut party_rng, party)
                    .await
            })
        }

        let key_shares = futures::future::try_join_all(outputs)
            .await
            .expect("keygen failed");

        for (i, key_share) in (0u16..).zip(&key_shares) {
            assert_eq!(key_share.i, i);
            assert_eq!(key_share.shared_public_key, key_shares[0].shared_public_key);
            assert_eq!(key_share.public_shares, key_shares[0].public_shares);
            assert_eq!(
                Point::<E>::generator() * &key_share.x,
                key_share.public_shares[usize::from(i)]
            );
        }

        // Choose `t` random key shares and reconstruct a secret key
        let t_shares = key_shares
            .choose_multiple(&mut rng, t.into())
            .cloned()
            .collect::<Vec<_>>();

        let sk = reconstruct_secret_key(&t_shares).unwrap();
        assert_eq!(Point::generator() * sk, key_shares[0].shared_public_key);
    }

    #[instantiate_tests(<cggmp21::supported_curves::Secp256k1>)]
    mod secp256k1 {}
    #[instantiate_tests(<cggmp21::supported_curves::Secp256r1>)]
    mod secp256r1 {}
    #[instantiate_tests(<cggmp21::supported_curves::Stark>)]
    mod stark {}
}
