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
        s.leave_puddle(0, Pos::tile_center(5, 5), crate::protocol::puddle::PEE);
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
    s.leave_puddle(w.floor, Pos::tile_center(w.tile.x, w.tile.y + 1), crate::protocol::puddle::PEE);
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
    assert_eq!(s.puddles[0].kind, crate::protocol::puddle::VOMIT);
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

/// Put player `id` on a free tile next to `(floor, tile)`.
fn stand_next_to(s: &mut Server, id: u16, floor: u8, t: crate::map::Tile) {
    let m = s.building.floor(floor).unwrap();
    let (x, y) =
        [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().map(|(dx, dy)| (t.x + dx, t.y + dy)).find(|&(x, y)| !m.is_blocked(x, y)).unwrap();
    let p = s.players.get_mut(&id).unwrap();
    p.body = Body::at(floor, Pos::tile_center(x, y));
    p.room = m.room_at_tile(x, y);
}

/// Run `n` whole server-side player ticks.
fn run_ticks(s: &mut Server, n: u32) {
    for _ in 0..n {
        s.tick += 1;
        let mut steps = s.simulate_players();
        s.react_to_steps(&mut steps);
    }
}

#[test]
fn five_cigarettes_in_a_row_come_back_up() {
    let mut s = server();
    add_player(&mut s, 1);
    let pack = crate::inventory::Item { count: 20, ..s.mint_item(item_kind::CIGARETTES, "") };
    s.players.get_mut(&1).unwrap().inventory.hands = Some(pack);
    for n in 1..=5 {
        s.use_held(1); // lit
        assert!(s.puddles.is_empty(), "cigarette {n}: fine so far");
        run_ticks(&mut s, crate::needs::SMOKE_TICKS + 1);
    }
    assert_eq!(s.puddles.len(), 1, "the fifth in a row: sick");
    assert_eq!(s.puddles[0].kind, crate::protocol::puddle::VOMIT);
    // With a break in between, the count starts again.
    run_ticks(&mut s, crate::drunk::VOMIT_TICKS + crate::mischief::CHAIN_GAP_TICKS + crate::needs::SMOKE_TICKS);
    for _ in 0..4 {
        s.use_held(1);
        run_ticks(&mut s, crate::needs::SMOKE_TICKS + 1);
    }
    assert_eq!(s.puddles.len(), 1, "four more: still fine");
}

#[test]
fn peeing_into_the_machine_and_a_mug_and_pooping_on_the_floor() {
    use crate::protocol::{activity, puddle};
    let mut s = server();
    add_player(&mut s, 1);
    add_player(&mut s, 2);
    let m = (s.machines[0].floor, s.machines[0].tile);
    stand_next_to(&mut s, 1, m.0, m.1);
    s.players.get_mut(&1).unwrap().needs.bladder = 0;
    s.handle_action(1, crate::protocol::action::MENU);
    let deeds = s.players[&1].deeds.clone();
    let machine = deeds.iter().position(|d| matches!(d, super::actions::Deed::PeeMachine(_))).expect("the machine in the menu");
    // Nothing to give yet.
    s.answer_mischief(1, crate::mischief::MENU_ID, machine as u8);
    assert!(s.says.iter().any(|l| l.text == crate::mischief::lines::NO_PEE));
    assert_eq!(s.machines[0].tainted, 0);
    s.players.get_mut(&1).unwrap().needs.bladder = 60 * crate::needs::SCALE;
    s.handle_action(1, crate::protocol::action::MENU);
    s.answer_mischief(1, crate::mischief::MENU_ID, machine as u8);
    assert_eq!(s.machines[0].tainted, crate::mischief::MACHINE_DOSES);
    assert_eq!(super::player::activity(&s.players[&1], s.tick), activity::PEEING);
    // The next coffee from it is "special".
    stand_next_to(&mut s, 2, m.0, m.1);
    s.players.get_mut(&2).unwrap().cup = crate::coffee::Cup::Brewing { machine: 0, until: s.tick + 1 };
    run_ticks(&mut s, 2);
    let coffee = s.players[&2].inventory.hands.clone().expect("coffee in hands");
    assert!(coffee.kind == item_kind::COFFEE && coffee.tainted);
    assert_eq!(s.machines[0].tainted, crate::mischief::MACHINE_DOSES - 1);
    let stress = s.players[&2].needs.stress;
    s.says.clear();
    s.use_held(2);
    let sick = !s.puddles.is_empty();
    let tasted = s.says.iter().any(|l| l.text == crate::mischief::lines::TASTE);
    assert!(sick ^ tasted, "either sick or just disgusted");
    assert!(s.players[&2].needs.stress > stress);
    // Into the mug somebody holds.
    let fresh = s.mint_item(item_kind::COFFEE, "");
    s.players.get_mut(&2).unwrap().inventory.hands = Some(fresh);
    s.players.get_mut(&1).unwrap().held_until = 0;
    s.players.get_mut(&1).unwrap().needs.bladder = 60 * crate::needs::SCALE;
    s.handle_action(1, crate::protocol::action::MENU);
    let cup = s.players[&1].deeds.iter().position(|d| *d == super::actions::Deed::PeeCup(2)).expect("Kuba's mug in the menu");
    s.says.clear();
    s.answer_mischief(1, crate::mischief::MENU_ID, cup as u8);
    assert!(s.players[&2].inventory.hands.as_ref().unwrap().tainted);
    assert!(s.says.iter().any(|l| l.speaker == 2 && l.text == crate::mischief::lines::OBLIVIOUS));
    // Pooping on the floor.
    s.players.get_mut(&1).unwrap().held_until = 0;
    s.players.get_mut(&1).unwrap().needs.bowels = 50 * crate::needs::SCALE;
    let puddles = s.puddles.len();
    s.handle_action(1, crate::protocol::action::MENU);
    s.answer_mischief(1, crate::mischief::MENU_ID, 1);
    assert_eq!(s.puddles.len(), puddles + 1);
    assert_eq!(s.puddles.last().unwrap().kind, puddle::POOP);
    assert_eq!(super::player::activity(&s.players[&1], s.tick), activity::POOPING);
    // The cleaner's round rinses the machine.
    s.cfg.cleaning_at = 0;
    s.cfg.cleaning_spread = 0;
    s.clock.ds = 10 * 60 * crate::clock::DS_PER_MIN;
    s.tick_cleaning();
    assert!(s.machines.iter().all(|m| m.tainted == 0));
}

#[test]
fn punches_knock_out_a_knife_calls_the_police_and_earns_a_reprimand() {
    use crate::protocol::activity;
    let mut s = server();
    add_player(&mut s, 1);
    add_player(&mut s, 2);
    s.players.get_mut(&2).unwrap().body.pos = Pos::tile_center(6, 5);
    // Ten punches, a second apart: out cold.
    for n in 1..=10 {
        s.players.get_mut(&1).unwrap().held_until = 0; // the guard is on his way; never mind
        s.handle_action(1, crate::protocol::action::ATTACK);
        assert_eq!(s.players[&2].needs.health_points(), 100 - 10 * n);
        s.handle_action(1, crate::protocol::action::ATTACK); // too soon: nothing
        assert_eq!(s.players[&2].needs.health_points(), 100 - 10 * n);
        s.tick += crate::mischief::PUNCH_COOLDOWN;
    }
    let p2 = &s.players[&2];
    assert!(p2.knocked_out && super::player::activity(p2, s.tick) == activity::KNOCKED_OUT);
    assert_eq!(s.players[&1].assault, Some(false));
    assert!(s.npcs.iter().any(|n| n.role == crate::npc::Role::Guard && n.chasing() == Some(1)), "the guard runs after them");
    assert!(s.police_calls.is_empty(), "no police for fists");
    // Lying down: not hit again.
    s.handle_action(1, crate::protocol::action::ATTACK);
    assert_eq!(s.players[&2].needs.health_points(), 0);
    // A minute later: up again with some health.
    s.tick += crate::mischief::KNOCKOUT_TICKS;
    run_ticks(&mut s, 1);
    assert!(!s.players[&2].knocked_out && s.players[&2].needs.health_points() >= 30);
    // A knife (F): a stab, the police and a reprimand.
    s.players.get_mut(&1).unwrap().contract = true;
    let knife = s.mint_item(item_kind::KNIFE, "");
    s.players.get_mut(&1).unwrap().inventory.hands = Some(knife);
    s.players.get_mut(&1).unwrap().held_until = 0;
    s.players.get_mut(&2).unwrap().needs.health = crate::needs::MAX;
    let before = s.players[&2].needs.health_points();
    s.use_held(1);
    assert_eq!(s.players[&2].needs.health_points(), before - 35);
    assert!(s.police_calls.iter().any(|c| c.target == 1));
    assert_eq!(s.players[&1].reprimands, 1);
    assert_eq!(s.players[&1].assault, Some(true));
    // Caught by the guard: held, and no longer wanted.
    let guard = s.npcs.iter().find(|n| n.role == crate::npc::Role::Guard).unwrap().id;
    s.caught(guard, 1);
    assert!(s.players[&1].assault.is_none() && s.players[&1].held_until > s.tick);
}

#[test]
fn the_cupboard_shows_mugs_and_knives() {
    let mut s = server();
    add_player(&mut s, 1);
    let k = s.kitchen.as_ref().unwrap();
    let (floor, cupboard) = (k.floor, k.cupboard);
    stand_next_to(&mut s, 1, floor, cupboard);
    s.open_cupboard(1);
    assert_eq!(s.players[&1].cupboard, vec![item_kind::CUP, item_kind::KNIFE, 0]);
    s.handle_dialog_answer(1, crate::mischief::CUPBOARD_ID, 1);
    assert!(s.players[&1].inventory.has(item_kind::KNIFE), "a knife in the pocket");
    assert_eq!(s.kitchen.as_ref().unwrap().knives, crate::kitchen::KNIVES - 1);
    // Back in the cupboard (from the hands).
    s.players.get_mut(&1).unwrap().inventory.take_out(0).unwrap();
    s.put_knife_back(1);
    assert!(s.players[&1].inventory.hands_free());
    assert_eq!(s.kitchen.as_ref().unwrap().knives, crate::kitchen::KNIVES);
}

#[test]
fn pani_wiesia_greets_newcomers_and_the_cashier_asks_about_the_hot_dog() {
    let mut s = server();
    add_player(&mut s, 1);
    let m = s.building.floor(0).unwrap();
    let lobby = m.room_by_name("Wiatrołap").unwrap().id;
    let hall = m.room_by_name("Hol").unwrap().id;
    let stairs = m.room_by_name("Klatka schodowa").unwrap().id;
    s.greet_entering(&[(1, 0, stairs, hall)]);
    assert!(s.says.is_empty(), "from the stairwell: already inside");
    s.greet_entering(&[(1, 0, lobby, hall)]);
    assert!(s.says.iter().any(|l| l.text.starts_with("Dzień dobry, p1! ")), "{:?}", s.says.iter().map(|l| &l.text).collect::<Vec<_>>());
    s.says.clear();
    s.greet_entering(&[(1, 0, lobby, hall)]);
    assert!(s.says.is_empty(), "not again right away");
    // At the counter with goods to pay for.
    let goods = crate::inventory::Item { unpaid: true, ..s.mint_item(item_kind::BEER, "") };
    let p = s.players.get_mut(&1).unwrap();
    p.inventory.hands = Some(goods);
    p.body = Body::at(0, Pos::tile_center(20, 53));
    s.tick_cashier();
    s.tick_cashier();
    let asked = s.says.iter().filter(|l| l.text == crate::npc::lines::CASHIER_HOTDOG).count();
    assert_eq!(asked, 1, "once");
    s.players.get_mut(&1).unwrap().body.pos = Pos::tile_center(24, 47);
    s.tick_cashier();
    s.players.get_mut(&1).unwrap().body.pos = Pos::tile_center(20, 53);
    s.tick_cashier();
    assert_eq!(s.says.iter().filter(|l| l.text == crate::npc::lines::CASHIER_HOTDOG).count(), 2, "again after stepping away");
}
