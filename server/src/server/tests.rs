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
    let before = s.positions.clone();
    // 256 would wrap to offer 0 with an `as u8` cast.
    s.company_set_places(256, 3);
    assert_eq!(s.positions, before);
}

#[test]
fn an_accident_leaves_a_puddle_until_the_office_closes() {
    let mut s = server();
    add_player(&mut s, 1);
    let at = Pos::tile_center(5, 5);
    s.players.get_mut(&1).unwrap().needs.bladder = crate::needs::MAX - 1;
    let mut steps = s.simulate_players();
    s.react_to_steps(&mut steps);
    assert_eq!(s.puddles.len(), 1, "a puddle where it happened");
    assert_eq!((s.puddles[0].floor, s.puddles[0].pos), (0, at));
    assert!((DROP_HANDLE_BASE..NPC_ID_BASE).contains(&s.puddles[0].handle));
    // Still there in the evening, gone once the office closes.
    s.clock.ds = (crate::clock::CLOSE_MIN - 1) * crate::clock::DS_PER_MIN;
    s.tick_clock();
    assert_eq!(s.puddles.len(), 1);
    while s.clock.minute() != crate::clock::CLOSE_MIN {
        s.tick_clock();
    }
    assert!(s.puddles.is_empty(), "mopped up at 22:00");
}

#[test]
fn puddles_are_capped_and_never_share_a_handle() {
    let mut s = server();
    for _ in 0..1000 {
        s.leave_puddle(0, Pos::tile_center(5, 5), false);
        let item = s.mint_item(item_kind::FRUIT, "Jabłko");
        s.drop_at(0, Pos::tile_center(5, 5), item);
    }
    assert!(s.puddles.len() <= 256, "{} puddles", s.puddles.len());
    let handles: HashSet<u16> = s.puddles.iter().map(|p| p.handle).chain(s.dropped.iter().map(|d| d.handle)).collect();
    assert_eq!(handles.len(), s.puddles.len() + s.dropped.len(), "duplicate entity handles");
}

#[test]
fn the_cleaner_mops_up_a_puddle_on_her_round() {
    let mut s = server();
    s.cfg.cleaning_at = 10 * 60;
    s.cfg.cleaning_spread = 0;
    s.clock.ds = 10 * 60 * crate::clock::DS_PER_MIN;
    let ws = crate::computer::find_workstations(&s.building);
    let w = ws.iter().find(|w| w.department == 1).unwrap();
    s.leave_puddle(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1), false);
    let mut said = false;
    for _ in 0..20 * 120 {
        s.tick += 1;
        s.tick_cleaning();
        let events = s.tick_npcs();
        s.apply_npc_events(events);
        said |= s.says.iter().any(|l| l.text == crate::cleaning::lines::PUDDLE);
        s.says.clear();
        if s.puddles.is_empty() {
            break;
        }
    }
    assert!(s.puddles.is_empty(), "the puddle is mopped up");
    assert!(said, "and she has a word about it");
}

/// A beer (or another drink) in hands, then F.
fn drink(s: &mut Server, id: u16, kind: u8) {
    let item = s.mint_item(kind, "test");
    s.players.get_mut(&id).unwrap().inventory.hands = Some(item);
    s.use_held(id);
}

#[test]
fn five_beers_throw_up_and_more_puts_you_to_sleep() {
    use crate::protocol::{activity, sound};
    let mut s = server();
    add_player(&mut s, 1);
    for _ in 0..4 {
        drink(&mut s, 1, item_kind::BEER);
    }
    assert!(s.puddles.is_empty(), "four beers: still standing");
    assert!(s.sounds.iter().any(|x| x.0 == sound::BURP), "a burp after a beer");
    assert_eq!(s.players[&1].needs.alcohol_points(), 60);
    drink(&mut s, 1, item_kind::BEER);
    assert_eq!(s.puddles.len(), 1, "the fifth: thrown up");
    assert!(s.puddles[0].vomit);
    assert!(s.sounds.iter().any(|x| x.0 == sound::VOMIT));
    assert_eq!(super::player::activity(&s.players[&1], s.tick), activity::VOMITING);
    drink(&mut s, 1, item_kind::WINE); // 65 + 30
    assert!(!s.players[&1].passed_out);
    drink(&mut s, 1, item_kind::MALPKA);
    let p = &s.players[&1];
    assert!(p.passed_out, "drank on after throwing up: asleep");
    assert_eq!(super::player::activity(p, s.tick), activity::PASSED_OUT);
    assert_eq!(s.puddles.len(), 1, "only one puddle");
    // A minute later they wake up with less in the blood.
    s.tick += crate::drunk::PASS_OUT_TICKS;
    s.simulate_players();
    let p = &s.players[&1];
    assert!(!p.passed_out && p.needs.alcohol_points() <= 60);
    assert!(s.says.iter().any(|l| l.text == crate::drunk::lines::WAKE_UP));
}

#[test]
fn the_board_tests_with_the_breathalyser_and_three_reprimands_fire() {
    let mut s = server();
    let (boss, worker) = (1, 2);
    add_player(&mut s, boss);
    add_player(&mut s, worker);
    s.company.founder = Some(boss);
    s.players.get_mut(&boss).unwrap().department = crate::company::BOARD_DEPARTMENT;
    let w = s.players.get_mut(&worker).unwrap();
    w.department = 1;
    w.contract = true;
    w.body.pos = Pos::tile_center(6, 5); // right next to the boss
                                         // Not on the board: no test.
    drink(&mut s, worker, item_kind::BREATHALYSER);
    assert!(s.says.iter().any(|l| l.text == crate::drunk::lines::NOT_BOARD));
    // Sober: a reading, no question.
    s.says.clear();
    drink(&mut s, boss, item_kind::BREATHALYSER);
    assert!(s.says.iter().any(|l| l.text.contains("0,00 ‰")), "{:?}", s.says.iter().map(|l| &l.text).collect::<Vec<_>>());
    assert!(s.players[&boss].reprimand_ask.is_none());
    for n in 1..=3u8 {
        s.players.get_mut(&worker).unwrap().needs.drink_alcohol(20);
        s.says.clear();
        drink(&mut s, boss, item_kind::BREATHALYSER);
        assert!(s.says.iter().any(|l| l.text.contains("pod wpływem")));
        let (ask, target) = s.players[&boss].reprimand_ask.expect("asked whether to reprimand");
        assert_eq!(target, worker);
        assert!(!s.answer_reprimand(boss, ask.wrapping_add(1), 0), "another dialog");
        assert!(s.answer_reprimand(boss, ask, 0));
        assert_eq!(s.players[&worker].reprimands, n);
        // Sober again for the next test.
        s.players.get_mut(&worker).unwrap().needs.alcohol = 0;
    }
    assert!(!s.players[&worker].contract, "the third reprimand: fired");
}
