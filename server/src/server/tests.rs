//! Server internals that the end-to-end tests can't reach cheaply.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Instant;

use super::items::DROP_HANDLE_BASE;
use super::player::{Player, Stage, Talk};
use super::{Config, Server};
use crate::board::{self, Meeting};
use crate::building::{default_building_path, Building};
use crate::inventory::kind as item_kind;
use crate::npc::NPC_ID_BASE;
use crate::protocol::{Packet, Profile};
use crate::recruitment::{default_recruitment_path, Recruitment};
use crate::sim::{Body, Pos};

fn server() -> Server {
    let building = Building::load(&default_building_path()).unwrap();
    let recruitment = Recruitment::load(&default_recruitment_path()).unwrap();
    let cfg = Config::new(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)), recruitment);
    Server::new(building, cfg).unwrap()
}

/// A player standing in the building (no real client behind the address).
fn add_player(s: &mut Server, id: u16) {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 9));
    let body = Body::at(0, Pos::tile_center(5, 5));
    let p = Player::new(id, u32::from(id), 0, addr, format!("p{id}"), Profile::default(), Stage::Working, body, Instant::now());
    s.players.insert(id, p);
}

#[test]
fn floor_items_are_capped_and_handles_stay_unique() {
    let mut s = server();
    for _ in 0..3000 {
        let item = s.mint_item(item_kind::FRUIT, "Jabłko");
        s.drop_at(0, Pos::tile_center(5, 5), item);
    }
    assert!(s.dropped.len() <= 1024, "{} items on the floor", s.dropped.len());
    let handles: HashSet<u16> = s.dropped.iter().map(|d| d.handle).collect();
    assert_eq!(handles.len(), s.dropped.len(), "duplicate entity handles");
    assert!(handles.iter().all(|h| (DROP_HANDLE_BASE..NPC_ID_BASE).contains(h)));
}

#[test]
fn player_ids_never_enter_the_entity_handle_range() {
    let mut s = server();
    s.next_id = DROP_HANDLE_BASE - 1;
    assert_eq!(s.alloc_id(), DROP_HANDLE_BASE - 1);
    assert_eq!(s.alloc_id(), 1, "ids wrap before the item handles");
}

#[test]
fn a_conversation_survives_other_meetings_going_away() {
    let mut s = server();
    let (me, other) = (1, 2);
    add_player(&mut s, me);
    add_player(&mut s, other);
    let day = s.clock.day;
    let meeting = |owner, start, state| Meeting { day, start, owner, topic: board::topic::RAISE, state };
    s.meetings.push(meeting(other, 13 * 60, board::State::Booked));
    s.meetings.push(meeting(me, 14 * 60, board::State::Talking(0)));
    let npc = s.npcs.first().map_or(NPC_ID_BASE, |n| n.id);
    s.players.get_mut(&me).unwrap().talk = Some(Talk { day, start: 14 * 60, npc, id: 1, good: 0 });

    // The other booking is cancelled mid-conversation: the list shifts.
    s.meetings.retain(|m| m.owner != other);

    assert!(matches!(s.dialog_packet(me), Some(Packet::Dialog { id: 1, .. })));
    s.end_talk(me, None);
    assert_eq!(s.meetings[0].state, board::State::Done);
    assert!(s.players[&me].talk.is_none());
}

#[test]
fn company_actions_ignore_offer_ids_that_do_not_fit_a_byte() {
    let mut s = server();
    let before = s.vacancies.clone();
    // 256 would wrap to offer 0 with an `as u8` cast.
    s.company_set_places(256, 3);
    assert_eq!(s.vacancies, before);
}
