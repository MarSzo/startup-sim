//! Round trips, layout and hostile input.

use super::*;

#[test]
fn roundtrip_all_samples() {
    for (name, p) in golden_samples() {
        let bytes = p.encode();
        assert!(bytes.len() <= MAX_PACKET, "{name}");
        assert_eq!(Packet::decode(&bytes).as_ref(), Ok(&p), "{name}");
    }
}

#[test]
fn header_layout() {
    let b = Packet::Reject { reason: 1 }.encode();
    assert_eq!(b, vec![0x54, 0x53, VERSION, ty::REJECT, 1]);
}

#[test]
fn rejects_garbage() {
    assert_eq!(Packet::decode(&[]), Err(DecodeError::TooShort));
    assert_eq!(Packet::decode(&[0, 0, 1, 1]), Err(DecodeError::BadMagic));
    assert_eq!(Packet::decode(&[0x54, 0x53, 99, 1]), Err(DecodeError::BadVersion(99)));
    assert_eq!(Packet::decode(&[0x54, 0x53, VERSION, 200]), Err(DecodeError::UnknownType(200)));
    let mut b = Packet::Ping { token: 1, client_time: 2 }.encode();
    b.push(0);
    assert!(Packet::decode(&b).is_err(), "trailing byte");
}

#[test]
fn truncated_packets_never_panic() {
    for (_, p) in golden_samples() {
        let b = p.encode();
        for n in 0..b.len() {
            assert!(Packet::decode(&b[..n]).is_err());
        }
    }
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = fastrand::Rng::with_seed(1);
    for _ in 0..20_000 {
        let len = rng.usize(0..64);
        let mut b: Vec<u8> = (0..len).map(|_| rng.u8(..)).collect();
        if len >= 4 && rng.bool() {
            b[0] = 0x54;
            b[1] = 0x53;
            b[2] = VERSION;
            b[3] = rng.u8(1..=43);
        }
        let _ = Packet::decode(&b);
    }
}

#[test]
fn long_speech_is_truncated_on_char_boundary() {
    let p = Packet::Say { id: 1, text: "ż".repeat(200) }; // 400 bytes
    let b = p.encode();
    assert!(b.len() <= MAX_PACKET);
    match Packet::decode(&b).unwrap() {
        Packet::Say { text, .. } => assert_eq!(text, "ż".repeat(MAX_SAY_BYTES / 2)),
        _ => unreachable!(),
    }
}

#[test]
fn nick_is_truncated_on_char_boundary() {
    let p = Packet::Connect { nonce: 1, nick: "ąąąąąąąąąą".into(), profile: Profile::default(), ticket: String::new() }; // 20 bytes
    match Packet::decode(&p.encode()).unwrap() {
        Packet::Connect { nick, .. } => assert_eq!(nick, "ąąąąąąąą"),
        _ => unreachable!(),
    }
}

#[test]
fn snapshot_fragments_fit_mtu() {
    let ents: Vec<EntityState> = (0..240)
        .map(|i| EntityState { id: i, kind: kind::PLAYER, x: i as i32 * 100, y: -(i as i32), flags: 0, held: 0, activity: 0 })
        .collect();
    let frags = snapshot_fragments(5, 6, SelfState { x: 1, y: 2, room: 3, ..Default::default() }, &ents);
    assert_eq!(frags.len(), 3);
    let mut seen = 0;
    for (i, f) in frags.iter().enumerate() {
        let b = f.encode();
        assert!(b.len() <= MAX_PACKET, "fragment {i} is {} B", b.len());
        match Packet::decode(&b).unwrap() {
            Packet::Snapshot { frag_idx, frag_cnt, entities, .. } => {
                assert_eq!((frag_idx as usize, frag_cnt), (i, 3));
                seen += entities.len();
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(seen, 240);
    let full = &frags[0].encode();
    assert_eq!(full.len(), SNAPSHOT_FIXED_LEN + MAX_ENTITIES_PER_SNAPSHOT * ENTITY_LEN);
}

#[test]
fn empty_room_still_sends_one_fragment() {
    let frags = snapshot_fragments(1, 0, SelfState::default(), &[]);
    assert_eq!(frags.len(), 1);
}
