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

/// A player job hunting on the portal.
fn on_portal(s: &mut Server, id: u16) {
    add_player(s, id);
    s.players.get_mut(&id).unwrap().stage = Stage::Portal(Box::default());
}

fn inbox_subjects(s: &Server, id: u16) -> Vec<String> {
    match &s.players[&id].stage {
        Stage::Portal(d) => d.inbox.iter().map(|m| m.subject.clone()).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn asking_too_much_gets_a_no_and_a_mandate_is_for_students_under_26() {
    use crate::protocol::employment;
    let mut s = server();
    on_portal(&mut s, 1);
    on_portal(&mut s, 2);
    on_portal(&mut s, 3);
    s.players.get_mut(&3).unwrap().profile.age = 30;
    let max = s.position(1).unwrap().salary[1];
    s.handle_apply(1, 1, max + 1, employment::EMPLOYMENT, false);
    s.handle_apply(2, 1, max, employment::MANDATE, true);
    s.handle_apply(3, 1, max, employment::MANDATE, true); // 30 years old: no mandate
    match &s.players[&3].stage {
        Stage::Portal(d) => assert!(d.applied.is_empty(), "rejected at once"),
        _ => unreachable!(),
    }
    s.tick += s.cfg.recruitment.invite_delay_secs * 20 + 1;
    s.deliver_replies(1);
    s.deliver_replies(2);
    assert_eq!(inbox_subjects(&s, 1), vec![crate::pay::lines::TOO_MUCH_SUBJECT]);
    assert!(inbox_subjects(&s, 2)[0].starts_with("Zaproszenie na rozmowę"));
    // Hired: what was agreed goes with them to HR.
    s.hire(2, 1);
    assert_eq!(s.players[&2].terms, Some(crate::pay::Terms { agreed: max, form: employment::MANDATE, offered: 0 }));
}

#[test]
fn the_contract_says_less_sign_it_or_get_seen_out() {
    use crate::pay::{self, Terms};
    use crate::protocol::employment;
    let mut s = server();
    let hr = s.npcs.iter().find(|n| n.role == crate::npc::Role::Hr).unwrap().id;
    for id in [1, 2] {
        add_player(&mut s, id);
        let pass = s.mint_item(item_kind::GUEST_PASS, "");
        let p = s.players.get_mut(&id).unwrap();
        p.inventory.add(pass).unwrap();
        p.position = Some(1);
        p.department = 1;
    }
    s.players.get_mut(&1).unwrap().terms = Some(Terms { agreed: 10_000, form: employment::B2B, offered: 0 });
    // Shown: lower than agreed (B2B: +20% on top), the same every time.
    s.show_contract(hr, 1);
    let offered = s.players[&1].terms.unwrap().offered;
    assert!((9_000..=10_800).contains(&offered) && offered.is_multiple_of(100), "{offered}");
    s.show_contract(hr, 1);
    assert_eq!(s.players[&1].terms.unwrap().offered, offered);
    let money = s.players[&1].money;
    s.handle_dialog_answer(1, pay::CONTRACT_ID, 0);
    let p = &s.players[&1];
    assert!(p.contract && p.salary == offered && p.employment == employment::B2B);
    assert_eq!(p.pay_rate, pay::hourly(offered));
    assert_eq!(p.money, money, "B2B: no advance");
    assert!(p.inventory.has(item_kind::EMPLOYEE_CARD) && !p.inventory.has(item_kind::GUEST_PASS));
    // Player 2 turns it down: HR walks them out, Pani Wiesia takes the pass,
    // and it's the job portal again (the place is free again).
    let places = s.places(1);
    s.show_contract(hr, 2);
    s.handle_dialog_answer(2, pay::CONTRACT_ID, 1);
    assert_eq!(s.npcs.iter().find(|n| n.id == hr).unwrap().escorting(), Some(2));
    s.says.clear();
    s.saw_out(2);
    assert!(!s.players[&2].inventory.has(item_kind::GUEST_PASS));
    assert!(s.says.iter().any(|l| l.text == pay::lines::PASS_BACK));
    s.tick += 200;
    s.tick_to_portal();
    assert!(matches!(s.players[&2].stage, Stage::Portal(_)));
    assert_eq!(inbox_subjects(&s, 2), vec!["Rezygnacja z umowy"]);
    assert_eq!((s.places(1), s.players[&2].position), (places + 1, None));
}

#[test]
fn reception_asks_about_lunch_and_pani_maria_never_stops_talking() {
    let mut s = server();
    add_player(&mut s, 1);
    add_player(&mut s, 2);
    s.clock.ds = 10 * 60 * crate::clock::DS_PER_MIN;
    let r = s.npcs.iter().find(|n| n.role == crate::npc::Role::Receptionist).unwrap().body;
    let p = s.players.get_mut(&1).unwrap();
    p.contract = true;
    p.body = Body::at(r.floor, Pos { x: r.pos.x, y: r.pos.y + 2 * 256 });
    for _ in 0..30 {
        s.tick += 1;
        s.tick_reception();
    }
    let asked = s.says.iter().filter(|l| l.text == crate::pay::lines::LUNCH).count();
    assert_eq!(asked, 1, "once a day");
    // Pani Maria: a story to each person near her, then a break, then more.
    s.says.clear();
    let m = s.npcs.iter().find(|n| n.role == crate::npc::Role::Cleaner).unwrap().body;
    for id in [1, 2] {
        s.players.get_mut(&id).unwrap().body = Body::at(m.floor, Pos { x: m.pos.x + 256, y: m.pos.y });
    }
    let stories = |s: &Server| s.says.iter().filter(|l| crate::cleaning::lines::STORIES.contains(&l.text.as_str())).count();
    for _ in 0..(20 * 20) {
        s.tick += 1;
        s.tick_maria();
    }
    assert_eq!(stories(&s), 2, "one each, 12 s apart");
    for _ in 0..(60 * 20) {
        s.tick += 1;
        s.tick_maria();
    }
    assert_eq!(stories(&s), 4, "and again after a minute");
}

#[test]
fn the_hr_app_plans_leave_and_a_day_off_is_spent_at_home_paid() {
    use crate::hr;
    let mut s = server();
    let hr_npc = s.npcs.iter().find(|n| n.role == crate::npc::Role::Hr).unwrap().id;
    add_player(&mut s, 1);
    let pass = s.mint_item(item_kind::GUEST_PASS, "");
    let p = s.players.get_mut(&1).unwrap();
    p.inventory.add(pass).unwrap();
    p.position = Some(1);
    p.department = 1;
    p.terms = Some(crate::pay::Terms { agreed: 8_400, form: crate::protocol::employment::EMPLOYMENT, offered: 0 });
    s.show_contract(hr_npc, 1);
    s.handle_dialog_answer(1, crate::pay::CONTRACT_ID, 0);
    let p = &s.players[&1];
    assert!(p.hr.annexes[0].text.starts_with("Umowa: Programista/ka, umowa o pracę"), "{:?}", p.hr.annexes);
    assert_eq!(p.hr.leave_days, hr::START_LEAVE_DAYS);
    // Leave for tomorrow: approved (one day less).
    let tomorrow = p.day + 1;
    s.handle_hr_action(1, hr::action::REQUEST, tomorrow as u16);
    assert_eq!(s.players[&1].hr.leave_days, hr::START_LEAVE_DAYS - 1);
    assert_eq!(s.players[&1].hr.requests[0].status, hr::status::APPROVED);
    // Evening, night, morning: the day off - at home, paid 8 hours.
    s.players.get_mut(&1).unwrap().stage = Stage::Home { arrive_at: None };
    let money = s.players[&1].money;
    s.clock.ds = (crate::clock::OPEN_MIN - 1) * crate::clock::DS_PER_MIN;
    while s.clock.minute() != crate::clock::OPEN_MIN {
        s.tick_clock();
    }
    let p = &s.players[&1];
    assert_eq!(p.day, tomorrow);
    assert!(p.hr.on_leave && p.depart_at.is_none(), "staying at home");
    assert_eq!(p.money, money + hr::LEAVE_HOURS * p.pay_rate);
    assert!(matches!(s.clock_packet(p), Packet::Clock { leave: true, .. }));
}

#[test]
fn the_remote_switches_the_tv_and_the_boombox_plays_where_it_is() {
    use crate::media;
    let mut s = server();
    add_player(&mut s, 1);
    assert_eq!(s.screens.len(), 1, "one TV, in the chill room");
    let (floor, tile, room) = (s.screens[0].floor, s.screens[0].tile, s.screens[0].room);
    assert_eq!(s.building.floor(floor).unwrap().room_name(room), "Chill room");
    // The remote and the boombox lie in the chill room from the start.
    let kinds: Vec<u8> = s.dropped.iter().map(|d| d.item.kind).collect();
    assert!(kinds.contains(&item_kind::REMOTE) && kinds.contains(&item_kind::BOOMBOX));
    // Pick up the remote (it's gone from the floor), in front of the TV.
    let at = s.dropped.iter().position(|d| d.item.kind == item_kind::REMOTE).unwrap();
    let remote = s.dropped.remove(at).item;
    let m = s.building.floor(floor).unwrap();
    let spot = Pos::tile_center(tile.x, tile.y - 2);
    let p = s.players.get_mut(&1).unwrap();
    p.inventory.hands = Some(remote);
    p.body = Body::at(floor, spot);
    p.room = m.room_at(spot.x, spot.y);
    s.tick = 500;
    s.use_held(1); // the channel list
    s.answer_media(1, media::TV_DIALOG, 3);
    assert_eq!((s.screens[0].channel, s.screens[0].started), (4, 500), "Mecz");
    assert!(s.says.iter().any(|l| l.text == media::lines::tv_on("Mecz")));
    s.answer_media(1, media::TV_DIALOG, media::CHANNELS.len() as u8);
    assert_eq!(s.screens[0].channel, 0, "off");
    // Somewhere else the remote does nothing.
    s.players.get_mut(&1).unwrap().room = 0;
    s.says.clear();
    s.use_held(1);
    assert!(s.says.iter().any(|l| l.text == media::lines::NOT_HERE));
    // The boombox: a track, carried around, put down, gone.
    let at = s.dropped.iter().position(|d| d.item.kind == item_kind::BOOMBOX).unwrap();
    let boombox = s.dropped.remove(at).item;
    s.players.get_mut(&1).unwrap().inventory.put_away().unwrap(); // the remote into a pocket
    s.players.get_mut(&1).unwrap().inventory.hands = Some(boombox);
    s.answer_media(1, media::BOOMBOX_DIALOG, 1);
    assert_eq!(s.music, Some((2, 500)), "Lo-fi");
    assert_eq!(s.boombox_at(), Some((floor, spot, 1)), "plays where the holder is");
    s.handle_item_action(1, crate::protocol::item_action::DROP, 0);
    assert_eq!(s.boombox_at().map(|b| b.2), Some(0), "on the floor");
    s.dropped.retain(|d| d.item.kind != item_kind::BOOMBOX);
    s.tick_media();
    assert_eq!(s.music, None, "gone: silence");
    // Next morning it's back (the remote too, unless somebody has it).
    s.ensure_media_items();
    let kinds: Vec<u8> = s.dropped.iter().map(|d| d.item.kind).collect();
    assert!(kinds.contains(&item_kind::BOOMBOX) && !kinds.contains(&item_kind::REMOTE), "{kinds:?}");
}

#[test]
fn the_storeroom_key_is_free_only_while_the_receptionist_is_away() {
    use crate::supplies::{self, lines};
    let mut s = server();
    add_player(&mut s, 1);
    let (hf, hook) = s.supplies.hook.expect("a key hook at the reception");
    stand_next_to(&mut s, 1, hf, hook);
    let body = s.players[&1].body;
    // She's at her desk: no key.
    assert!(s.use_supplies(1, &body));
    assert!(s.says.iter().any(|l| l.text == lines::KEY_NO));
    assert!(s.supplies.key_on_hook);
    // Lunch break: she's off to the kitchenette - the key is free.
    s.clock.ds = supplies::BREAK_FROM * crate::clock::DS_PER_MIN;
    s.tick_lunch_break();
    assert!(s.supplies.on_break);
    s.use_supplies(1, &body);
    assert!(s.players[&1].inventory.has(item_kind::STORE_KEY) && !s.supplies.key_on_hook);
    assert_eq!(s.players[&1].body.access & crate::map::access::KEY, crate::map::access::KEY, "the key opens the storeroom");
    // The storeroom door needs it.
    let (sf, room) = s.supplies.storeroom.unwrap();
    let m = s.building.floor(sf).unwrap();
    let door =
        (0..m.height).flat_map(|y| (0..m.width).map(move |x| (x, y))).find(|&(x, y)| m.tile_type(x, y) == Some("storeroom_door")).unwrap();
    assert!(
        m.blocks(door.0, door.1, crate::map::access::CARD, crate::map::dir::DOWN)
            && !m.blocks(door.0, door.1, crate::map::access::KEY, crate::map::dir::DOWN)
    );
    // Inside: cola from the shelf, until it runs out.
    let shelf = s.supplies.shelves[0].1;
    stand_next_to(&mut s, 1, sf, shelf);
    assert_eq!(s.players[&1].room, room);
    let body = s.players[&1].body;
    for _ in 0..supplies::STOREROOM_STOCK {
        s.use_supplies(1, &body);
        s.answer_supplies(1, supplies::DIALOG, 0);
        let p = s.players.get_mut(&1).unwrap();
        let has = p.inventory.has(item_kind::COLA);
        assert!(has, "a cola");
        p.inventory.remove_kind(item_kind::COLA);
    }
    s.says.clear();
    s.use_supplies(1, &body);
    s.answer_supplies(1, supplies::DIALOG, 0);
    assert!(s.says.iter().any(|l| l.text == lines::EMPTY), "all gone for today");
    // Back at the hook: hang it up. The next morning: all full again.
    s.players.get_mut(&1).unwrap().inventory.take_out(0).ok();
    stand_next_to(&mut s, 1, hf, hook);
    let body = s.players[&1].body;
    let key_in_hands = s.players[&1].inventory.held_kind() == item_kind::STORE_KEY;
    if !key_in_hands {
        let at = s.players[&1].inventory.pockets.iter().position(|i| i.as_ref().is_some_and(|i| i.kind == item_kind::STORE_KEY)).unwrap();
        s.players.get_mut(&1).unwrap().inventory.take_out(at).unwrap();
    }
    s.use_supplies(1, &body);
    assert!(s.supplies.key_on_hook && !s.players[&1].inventory.has(item_kind::STORE_KEY));
    s.restock_supplies();
    assert_eq!(s.supplies.stock.left(item_kind::COLA), supplies::STOREROOM_STOCK);
}

#[test]
fn medicines_help_and_a_rolled_cigarette_is_as_good_as_the_rolling() {
    use crate::supplies::{self, lines};
    let mut s = server();
    add_player(&mut s, 1);
    let (cf, cabinet) = s.supplies.cabinet.expect("a first-aid cabinet");
    stand_next_to(&mut s, 1, cf, cabinet);
    let body = s.players[&1].body;
    s.players.get_mut(&1).unwrap().needs.health = 50 * crate::needs::SCALE;
    s.use_supplies(1, &body);
    assert_eq!(s.players[&1].supply_menu[0], item_kind::PAINKILLER);
    s.answer_supplies(1, supplies::DIALOG, 0);
    let at = s.players[&1].inventory.pockets.iter().position(|i| i.as_ref().is_some_and(|i| i.kind == item_kind::PAINKILLER)).unwrap();
    s.players.get_mut(&1).unwrap().inventory.take_out(at).unwrap();
    s.use_held(1);
    assert_eq!(s.players[&1].needs.health_points(), 70);
    assert_eq!(s.supplies.stock.left(item_kind::PAINKILLER), supplies::MEDICINE_STOCK - 1);
    // Tobacco from the shop (paid): roll one - a bad one crumbles, a good one smokes.
    let pack = crate::inventory::Item { count: 10, ..s.mint_item(item_kind::TOBACCO, "") };
    s.players.get_mut(&1).unwrap().inventory.hands = Some(pack);
    s.handle_roll(1, 12);
    s.handle_roll(1, 92);
    let rolls: Vec<(u8, String)> =
        s.players[&1].inventory.items().filter(|i| i.kind == item_kind::ROLLED).map(|i| (i.quality, i.label.clone())).collect();
    assert_eq!(rolls, vec![(12, "Skręt (rozsypujący się)".into()), (92, "Skręt (idealny)".into())]);
    assert_eq!(s.players[&1].inventory.hands.as_ref().unwrap().count, 8, "two rolls from the pack");
    s.players.get_mut(&1).unwrap().inventory.put_away().unwrap(); // the pack into a pocket
    let slot = |s: &Server, q: u8| s.players[&1].inventory.pockets.iter().position(|i| i.as_ref().is_some_and(|i| i.quality == q)).unwrap();
    let bad = slot(&s, 12);
    s.players.get_mut(&1).unwrap().inventory.take_out(bad).unwrap();
    s.says.clear();
    s.use_held(1);
    assert!(s.says.iter().any(|l| l.text == lines::CRUMBLED) && s.players[&1].rest.is_none());
    let good = slot(&s, 92);
    s.players.get_mut(&1).unwrap().inventory.take_out(good).unwrap();
    s.use_held(1);
    assert!(matches!(s.players[&1].rest, Some((crate::needs::Rest::Smoking { .. }, _, _))), "smoking the good one");
}
