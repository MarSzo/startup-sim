//! Packet -> bytes.

use super::codec::Writer;
use super::*;

impl Packet {
    pub fn type_id(&self) -> u8 {
        match self {
            Packet::Connect { .. } => ty::CONNECT,
            Packet::Welcome { .. } => ty::WELCOME,
            Packet::Reject { .. } => ty::REJECT,
            Packet::Input { .. } => ty::INPUT,
            Packet::Snapshot { .. } => ty::SNAPSHOT,
            Packet::PlayerInfo { .. } => ty::PLAYER_INFO,
            Packet::InfoRequest { .. } => ty::INFO_REQUEST,
            Packet::Ping { .. } => ty::PING,
            Packet::Pong { .. } => ty::PONG,
            Packet::Disconnect { .. } => ty::DISCONNECT,
            Packet::Say { .. } => ty::SAY,
            Packet::JobOffers { .. } => ty::JOB_OFFERS,
            Packet::Apply { .. } => ty::APPLY,
            Packet::Question { .. } => ty::QUESTION,
            Packet::Answer { .. } => ty::ANSWER,
            Packet::RecruitResult { .. } => ty::RECRUIT_RESULT,
            Packet::Mail { .. } => ty::MAIL,
            Packet::PortalAction { .. } => ty::PORTAL_ACTION,
            Packet::Inventory { .. } => ty::INVENTORY,
            Packet::ItemAction { .. } => ty::ITEM_ACTION,
            Packet::Computer { .. } => ty::COMPUTER,
            Packet::ComputerAction { .. } => ty::COMPUTER_ACTION,
            Packet::Chat { .. } => ty::CHAT,
            Packet::Stats { .. } => ty::STATS,
            Packet::Shelf { .. } => ty::SHELF,
            Packet::ShopTake { .. } => ty::SHOP_TAKE,
            Packet::Clock { .. } => ty::CLOCK,
            Packet::CommuteChoice { .. } => ty::COMMUTE_CHOICE,
            Packet::Calendar { .. } => ty::CALENDAR,
            Packet::CalendarBook { .. } => ty::CALENDAR_BOOK,
            Packet::Dialog { .. } => ty::DIALOG,
            Packet::DialogAnswer { .. } => ty::DIALOG_ANSWER,
            Packet::LunchMenu { .. } => ty::LUNCH_MENU,
            Packet::LunchOrder { .. } => ty::LUNCH_ORDER,
            Packet::CompanyOffers { .. } => ty::COMPANY_OFFERS,
            Packet::CompanyPeople { .. } => ty::COMPANY_PEOPLE,
            Packet::CompanyAction { .. } => ty::COMPANY_ACTION,
            Packet::Smoke { .. } => ty::SMOKE,
            Packet::Lights { .. } => ty::LIGHTS,
            Packet::Fridge { .. } => ty::FRIDGE,
            Packet::FridgeAction { .. } => ty::FRIDGE_ACTION,
            Packet::SkipWait { .. } => ty::SKIP_WAIT,
            Packet::Sound { .. } => ty::SOUND,
            Packet::TaskAction { .. } => ty::TASK_ACTION,
            Packet::TaskBoard { .. } => ty::TASK_BOARD,
            Packet::TaskDetail { .. } => ty::TASK_DETAIL,
            Packet::MailAction { .. } => ty::MAIL_ACTION,
            Packet::WorkMail { .. } => ty::WORK_MAIL,
            Packet::MailState { .. } => ty::MAIL_STATE,
            Packet::Voice { .. } => ty::VOICE,
            Packet::VoiceFrom { .. } => ty::VOICE_FROM,
            Packet::Doors { .. } => ty::DOORS,
            Packet::DoorAction { .. } => ty::DOOR_ACTION,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer(Vec::with_capacity(64));
        w.u16(MAGIC);
        w.u8(VERSION);
        w.u8(self.type_id());
        match self {
            Packet::Connect { nonce, nick, profile, ticket } => {
                w.u32(*nonce);
                w.str8(nick);
                w.u8(profile.gender);
                w.u8(profile.age);
                w.appearance(&profile.appearance);
                w.str16(&profile.city, MAX_CITY_BYTES);
                w.str16(&profile.email, MAX_EMAIL_BYTES);
                w.str16(ticket, MAX_TICKET_BYTES);
            }
            Packet::Welcome { nonce, player_id, token, tick_hz, input_hz, map_crc, server_tick } => {
                w.u32(*nonce);
                w.u16(*player_id);
                w.u32(*token);
                w.u8(*tick_hz);
                w.u8(*input_hz);
                w.u32(*map_crc);
                w.u32(*server_tick);
            }
            Packet::Reject { reason } => w.u8(*reason),
            Packet::Input { token, ack_tick, last_seq, inputs } => {
                w.u32(*token);
                w.u32(*ack_tick);
                w.u32(*last_seq);
                let n = inputs.len().min(MAX_INPUTS_PER_PACKET);
                w.u8(n as u8);
                for &i in &inputs[inputs.len() - n..] {
                    w.u8(i);
                }
            }
            Packet::Snapshot {
                tick,
                last_input_seq,
                frag_idx,
                frag_cnt,
                self_x,
                self_y,
                floor,
                room,
                self_lock,
                self_prev_input,
                self_access,
                self_slow,
                self_activity,
                entities,
            } => {
                w.u32(*tick);
                w.u32(*last_input_seq);
                w.u8(*frag_idx);
                w.u8(*frag_cnt);
                w.i32(*self_x);
                w.i32(*self_y);
                w.u8(*floor);
                w.u16(*room);
                w.u8(*self_lock);
                w.u8(*self_prev_input);
                w.u8(*self_access);
                w.u8(*self_slow);
                w.u8(*self_activity);
                let n = entities.len().min(MAX_ENTITIES_PER_SNAPSHOT);
                w.u8(n as u8);
                for e in &entities[..n] {
                    w.u16(e.id);
                    w.u8(e.kind);
                    w.i32(e.x);
                    w.i32(e.y);
                    w.u8(e.flags);
                    w.u8(e.held);
                    w.u8(e.activity);
                }
            }
            Packet::PlayerInfo { players } => {
                w.u8(players.len().min(255) as u8);
                for p in players.iter().take(255) {
                    w.u16(p.id);
                    w.str8(&p.nick);
                    w.u8(p.department);
                    w.u8(p.gender);
                    w.appearance(&p.appearance);
                }
            }
            Packet::InfoRequest { token, ids } => {
                w.u32(*token);
                w.u8(ids.len().min(255) as u8);
                for &id in ids.iter().take(255) {
                    w.u16(id);
                }
            }
            Packet::Ping { token, client_time } => {
                w.u32(*token);
                w.u32(*client_time);
            }
            Packet::Pong { client_time, server_tick } => {
                w.u32(*client_time);
                w.u32(*server_tick);
            }
            Packet::Disconnect { token, reason } => {
                w.u32(*token);
                w.u8(*reason);
            }
            Packet::Say { id, text } => {
                w.u16(*id);
                w.str16(text, MAX_SAY_BYTES);
            }
            Packet::JobOffers { offers } => {
                w.u8(offers.len().min(16) as u8);
                for o in offers.iter().take(16) {
                    w.u8(o.id);
                    w.u8(o.department);
                    w.u8(o.applied as u8);
                    w.u8(o.vacancies);
                    w.str16(&o.company, MAX_TEXT_BYTES);
                    w.str16(&o.title, MAX_TEXT_BYTES);
                    w.str16(&o.description, MAX_TEXT_BYTES);
                }
            }
            Packet::Apply { token, offer, motivation } => {
                w.u32(*token);
                w.u8(*offer);
                w.str16(motivation, MAX_TEXT_BYTES);
            }
            Packet::Question { attempt, index, total, text, options } => {
                w.u8(*attempt);
                w.u8(*index);
                w.u8(*total);
                w.str16(text, MAX_TEXT_BYTES);
                w.u8(options.len().min(MAX_OPTIONS) as u8);
                for o in options.iter().take(MAX_OPTIONS) {
                    w.str16(o, MAX_TEXT_BYTES);
                }
            }
            Packet::Answer { token, attempt, index, choice } => {
                w.u32(*token);
                w.u8(*attempt);
                w.u8(*index);
                w.u8(*choice);
            }
            Packet::RecruitResult { attempt, passed, score, total, department } => {
                w.u8(*attempt);
                w.u8(*passed as u8);
                w.u8(*score);
                w.u8(*total);
                w.u8(*department);
            }
            Packet::Mail { id, from, subject, body, action, arg } => {
                w.u8(*id);
                w.str16(from, MAX_TEXT_BYTES);
                w.str16(subject, MAX_TEXT_BYTES);
                w.str16(body, MAX_MAIL_BYTES);
                w.u8(*action);
                w.u8(*arg);
            }
            Packet::PortalAction { token, action, arg } => {
                w.u32(*token);
                w.u8(*action);
                w.u8(*arg);
            }
            Packet::Inventory { slots } => {
                w.u8(slots.len().min(8) as u8);
                for sl in slots.iter().take(8) {
                    w.u8(sl.kind);
                    w.u32(sl.id);
                    w.str16(&sl.label, MAX_TEXT_BYTES);
                }
            }
            Packet::ItemAction { token, action, slot } => {
                w.u32(*token);
                w.u8(*action);
                w.u8(*slot);
            }
            Packet::Computer { handle, owner, locked, convs } => {
                w.u16(*handle);
                w.u16(*owner);
                w.u8(*locked as u8);
                w.u8(convs.len().min(MAX_CONVS) as u8);
                for c in convs.iter().take(MAX_CONVS) {
                    w.u16(c.conv);
                    w.u8(c.unread);
                    w.str16(&c.title, MAX_NICK_BYTES + 8);
                }
            }
            Packet::ComputerAction { token, action, conv, arg, text } => {
                w.u32(*token);
                w.u8(*action);
                w.u16(*conv);
                w.u32(*arg);
                w.str16(text, MAX_CHAT_BYTES);
            }
            Packet::Doors { floor, tiles, lift_floor, lift_target, lift_moving } => {
                w.u8(*floor);
                w.u8(tiles.len().min(255) as u8);
                for (x, y) in tiles.iter().take(255) {
                    w.u8(*x);
                    w.u8(*y);
                }
                w.u8(*lift_floor);
                w.u8(*lift_target);
                w.u8(*lift_moving as u8);
            }
            Packet::DoorAction { token } => w.u32(*token),
            Packet::Stats { hunger, energy, stress, bladder, hygiene, flags, money } => {
                w.u8(*hunger);
                w.u8(*energy);
                w.u8(*stress);
                w.u8(*bladder);
                w.u8(*hygiene);
                w.u8(*flags);
                w.u32(*money);
            }
            Packet::Shelf { shelf, title, goods } => {
                w.u8(*shelf);
                w.str16(title, MAX_TEXT_BYTES);
                w.u8(goods.len().min(16) as u8);
                for g in goods.iter().take(16) {
                    w.u8(g.kind);
                    w.u32(g.price);
                    w.str16(&g.name, MAX_TEXT_BYTES);
                }
            }
            Packet::ShopTake { token, shelf, kind } => {
                w.u32(*token);
                w.u8(*shelf);
                w.u8(*kind);
            }
            Packet::Clock {
                day,
                minute,
                night,
                place,
                arrive,
                pay,
                pay_minutes,
                today_minutes,
                mode,
                depart,
                money,
                weather,
                company,
                founded,
                alarm,
                skip,
            } => {
                w.u16(*day);
                w.u16(*minute);
                w.u8(*night as u8);
                w.u8(*place);
                w.u16(*arrive);
                w.u32(*pay);
                w.u16(*pay_minutes);
                w.u16(*today_minutes);
                w.u8(*mode);
                w.u16(*depart);
                w.u32(*money);
                w.u8(*weather);
                w.str16(company, 64);
                w.u8(*founded as u8);
                w.u8(*alarm);
                w.u8(*skip);
            }
            Packet::Fridge { items, milk, water, juice } => {
                w.u8(items.len().min(16) as u8);
                for (k, label) in items.iter().take(16) {
                    w.u8(*k);
                    w.str16(label, 64);
                }
                w.u8(*milk);
                w.u8(*water);
                w.u8(*juice);
            }
            Packet::SkipWait { token } => w.u32(*token),
            Packet::TaskAction { token, nonce, action, task, arg, text } => {
                w.u32(*token);
                w.u16(*nonce);
                w.u8(*action);
                w.u16(*task);
                w.u8(*arg);
                w.str16(text, TASK_TEXT_MAX);
            }
            Packet::TaskBoard { dept, done, part, parts, members, tasks } => {
                w.u8(*dept);
                w.u16(*done);
                w.u8(*part);
                w.u8(*parts);
                w.u8(members.len().min(MAX_MEMBERS) as u8);
                for m in members.iter().take(MAX_MEMBERS) {
                    w.str8(m);
                }
                w.u8(tasks.len().min(255) as u8);
                for t in tasks.iter().take(255) {
                    w.u16(t.id);
                    w.u8(t.column);
                    w.u8(t.priority);
                    w.u8(t.comments);
                    w.str16(&t.title, TASK_TITLE_MAX);
                    w.str8(&t.author);
                    w.str8(&t.assignee);
                }
            }
            Packet::TaskDetail { id, desc, comments } => {
                w.u16(*id);
                w.str16(desc, TASK_TEXT_MAX);
                w.u8(comments.len().min(DETAIL_COMMENTS) as u8);
                for (nick, text) in comments.iter().take(DETAIL_COMMENTS) {
                    w.str8(nick);
                    w.str16(text, TASK_COMMENT_MAX);
                }
            }
            Packet::MailAction { token, nonce, action, id, to, subject, body } => {
                w.u32(*token);
                w.u16(*nonce);
                w.u8(*action);
                w.u16(*id);
                w.str8(to);
                w.str16(subject, MAIL_SUBJECT_MAX);
                w.str16(body, MAIL_BODY_MAX);
            }
            Packet::WorkMail { id, from, to, subject, body, day, minute } => {
                w.u16(*id);
                w.str8(from);
                w.str8(to);
                w.str16(subject, MAIL_SUBJECT_MAX);
                w.str16(body, MAIL_BODY_MAX);
                w.u16(*day);
                w.u16(*minute);
            }
            Packet::MailState { done, ids, trashed } => {
                w.u16(*done);
                for list in [ids, trashed] {
                    w.u8(list.len().min(MAX_MAIL_IDS) as u8);
                    for id in list.iter().take(MAX_MAIL_IDS) {
                        w.u16(*id);
                    }
                }
            }
            Packet::Voice { token, seq, whisper, data } => {
                w.u32(*token);
                w.u16(*seq);
                w.u8(*whisper);
                let n = data.len().min(MAX_VOICE_BYTES);
                w.u16(n as u16);
                w.0.extend_from_slice(&data[..n]);
            }
            Packet::VoiceFrom { speaker, seq, whisper, data } => {
                w.u16(*speaker);
                w.u16(*seq);
                w.u8(*whisper);
                let n = data.len().min(MAX_VOICE_BYTES);
                w.u16(n as u16);
                w.0.extend_from_slice(&data[..n]);
            }
            Packet::Sound { sounds } => {
                w.u8(sounds.len().min(MAX_SOUNDS) as u8);
                for (kind, x, y) in sounds.iter().take(MAX_SOUNDS) {
                    w.u8(*kind);
                    w.i32(*x);
                    w.i32(*y);
                }
            }
            Packet::FridgeAction { token, action, arg } => {
                w.u32(*token);
                w.u8(*action);
                w.u8(*arg);
            }
            Packet::Lights { floor, rooms } => {
                w.u8(*floor);
                w.u8(rooms.len().min(64) as u8);
                for room in rooms.iter().take(64) {
                    w.u16(*room);
                }
            }
            Packet::Smoke { floor, rooms } => {
                w.u8(*floor);
                w.u8(rooms.len().min(64) as u8);
                for (room, level) in rooms.iter().take(64) {
                    w.u16(*room);
                    w.u8(*level);
                }
            }
            Packet::CompanyOffers { name, part, parts, sets, offers } => {
                w.str16(name, 64);
                w.u8(*part);
                w.u8(*parts);
                w.u8(sets.len().min(16) as u8);
                for (id, label) in sets.iter().take(16) {
                    w.str8(id);
                    w.str16(label, 64);
                }
                w.u8(offers.len().min(16) as u8);
                for o in offers.iter().take(16) {
                    w.u8(o.id);
                    w.u8(o.places);
                    w.u8(o.department);
                    w.str8(&o.set);
                    w.str16(&o.title, 64);
                    w.str16(&o.description, MAX_TEXT_BYTES);
                }
            }
            Packet::CompanyPeople { candidates, staff } => {
                w.u8(candidates.len().min(20) as u8);
                for (pid, offer, score, total, nick) in candidates.iter().take(20) {
                    w.u16(*pid);
                    w.u8(*offer);
                    w.u8(*score);
                    w.u8(*total);
                    w.str16(nick, MAX_NICK_BYTES);
                }
                w.u8(staff.len().min(30) as u8);
                for (pid, dept, day, nick) in staff.iter().take(30) {
                    w.u16(*pid);
                    w.u8(*dept);
                    w.u16(*day);
                    w.str16(nick, MAX_NICK_BYTES);
                }
            }
            Packet::CompanyAction { token, action, target, value, text } => {
                w.u32(*token);
                w.u8(*action);
                w.u16(*target);
                w.u8(*value);
                w.str16(text, MAX_TEXT_BYTES);
            }
            Packet::CommuteChoice { token, mode } => {
                w.u32(*token);
                w.u8(*mode);
            }
            Packet::Calendar { mine_start, mine_topic, slots } => {
                w.u16(*mine_start);
                w.u8(*mine_topic);
                w.u8(slots.len().min(64) as u8);
                for (start, state) in slots.iter().take(64) {
                    w.u16(*start);
                    w.u8(*state);
                }
            }
            Packet::CalendarBook { token, start, topic } => {
                w.u32(*token);
                w.u16(*start);
                w.u8(*topic);
            }
            Packet::Dialog { id, npc, text, options } => {
                w.u8(*id);
                w.u16(*npc);
                w.str16(text, MAX_TEXT_BYTES);
                w.u8(options.len().min(MAX_OPTIONS) as u8);
                for o in options.iter().take(MAX_OPTIONS) {
                    w.str16(o, MAX_TEXT_BYTES);
                }
            }
            Packet::DialogAnswer { token, id, choice } => {
                w.u32(*token);
                w.u8(*id);
                w.u8(*choice);
            }
            Packet::LunchMenu { state, dish, arrives, dishes } => {
                w.u8(*state);
                w.u8(*dish);
                w.u16(*arrives);
                w.u8(dishes.len().min(12) as u8);
                for d in dishes.iter().take(12) {
                    w.u8(d.kind);
                    w.u32(d.price);
                    w.u8(d.eta);
                    w.str16(&d.name, MAX_NICK_BYTES * 4);
                    w.str16(&d.restaurant, MAX_NICK_BYTES * 4);
                }
            }
            Packet::LunchOrder { token, dish } => {
                w.u32(*token);
                w.u8(*dish);
            }
            Packet::Chat { conv, messages } => {
                w.u16(*conv);
                w.u8(messages.len().min(255) as u8);
                for m in messages.iter().take(255) {
                    w.u32(m.id);
                    w.u16(m.from);
                    w.str16(&m.nick, MAX_NICK_BYTES);
                    w.str16(&m.text, MAX_CHAT_BYTES);
                }
            }
        }
        w.0
    }
}
