//! Bytes -> packet; never panics on hostile input.

use super::codec::Reader;
use super::*;

impl Packet {
    pub fn decode(b: &[u8]) -> Result<Packet, DecodeError> {
        let mut r = Reader::new(b);
        if r.u16()? != MAGIC {
            return Err(DecodeError::BadMagic);
        }
        let version = r.u8()?;
        if version != VERSION {
            return Err(DecodeError::BadVersion(version));
        }
        let t = r.u8()?;
        let p = match t {
            ty::CONNECT => {
                let (nonce, nick) = (r.u32()?, r.str8()?);
                let (gender, age, appearance) = (r.u8()?, r.u8()?, r.appearance()?);
                let city = r.str16(MAX_CITY_BYTES)?;
                let email = r.str16(MAX_EMAIL_BYTES)?;
                let ticket = r.str16(MAX_TICKET_BYTES)?;
                Packet::Connect { nonce, nick, profile: Profile { gender, age, city, email, appearance }, ticket }
            }
            ty::WELCOME => Packet::Welcome {
                nonce: r.u32()?,
                player_id: r.u16()?,
                token: r.u32()?,
                tick_hz: r.u8()?,
                input_hz: r.u8()?,
                map_crc: r.u32()?,
                server_tick: r.u32()?,
            },
            ty::REJECT => Packet::Reject { reason: r.u8()? },
            ty::INPUT => {
                let token = r.u32()?;
                let ack_tick = r.u32()?;
                let last_seq = r.u32()?;
                let n = r.u8()? as usize;
                if n > MAX_INPUTS_PER_PACKET {
                    return Err(DecodeError::Invalid("too many inputs"));
                }
                Packet::Input { token, ack_tick, last_seq, inputs: r.take(n)?.to_vec() }
            }
            ty::SNAPSHOT => {
                let tick = r.u32()?;
                let last_input_seq = r.u32()?;
                let frag_idx = r.u8()?;
                let frag_cnt = r.u8()?;
                let self_x = r.i32()?;
                let self_y = r.i32()?;
                let floor = r.u8()?;
                let room = r.u16()?;
                let self_lock = r.u8()?;
                let self_prev_input = r.u8()?;
                let self_access = r.u8()?;
                let self_slow = r.u8()?;
                let self_activity = r.u8()?;
                let n = r.u8()? as usize;
                let mut entities = Vec::with_capacity(n);
                for _ in 0..n {
                    entities.push(EntityState { id: r.u16()?, kind: r.u8()?, x: r.i32()?, y: r.i32()?, flags: r.u8()?, held: r.u8()?, activity: r.u8()? });
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
                }
            }
            ty::PLAYER_INFO => {
                let n = r.u8()? as usize;
                let mut players = Vec::with_capacity(n);
                for _ in 0..n {
                    players.push(PlayerInfoEntry {
                        id: r.u16()?,
                        nick: r.str8()?,
                        department: r.u8()?,
                        gender: r.u8()?,
                        appearance: r.appearance()?,
                    });
                }
                Packet::PlayerInfo { players }
            }
            ty::INFO_REQUEST => {
                let token = r.u32()?;
                let n = r.u8()? as usize;
                let mut ids = Vec::with_capacity(n);
                for _ in 0..n {
                    ids.push(r.u16()?);
                }
                Packet::InfoRequest { token, ids }
            }
            ty::PING => Packet::Ping { token: r.u32()?, client_time: r.u32()? },
            ty::PONG => Packet::Pong { client_time: r.u32()?, server_tick: r.u32()? },
            ty::DISCONNECT => Packet::Disconnect { token: r.u32()?, reason: r.u8()? },
            ty::SAY => Packet::Say { id: r.u16()?, text: r.str16(MAX_SAY_BYTES)? },
            ty::JOB_OFFERS => {
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many offers"));
                }
                let mut offers = Vec::with_capacity(n);
                for _ in 0..n {
                    offers.push(OfferInfo {
                        id: r.u8()?,
                        department: r.u8()?,
                        applied: r.u8()? != 0,
                        vacancies: r.u8()?,
                        company: r.str16(MAX_TEXT_BYTES)?,
                        title: r.str16(MAX_TEXT_BYTES)?,
                        description: r.str16(MAX_TEXT_BYTES)?,
                    });
                }
                Packet::JobOffers { offers }
            }
            ty::APPLY => Packet::Apply { token: r.u32()?, offer: r.u8()?, motivation: r.str16(MAX_TEXT_BYTES)? },
            ty::QUESTION => {
                let (attempt, index, total) = (r.u8()?, r.u8()?, r.u8()?);
                let text = r.str16(MAX_TEXT_BYTES)?;
                let n = r.u8()? as usize;
                if n > MAX_OPTIONS {
                    return Err(DecodeError::Invalid("too many options"));
                }
                let mut options = Vec::with_capacity(n);
                for _ in 0..n {
                    options.push(r.str16(MAX_TEXT_BYTES)?);
                }
                Packet::Question { attempt, index, total, text, options }
            }
            ty::ANSWER => Packet::Answer { token: r.u32()?, attempt: r.u8()?, index: r.u8()?, choice: r.u8()? },
            ty::RECRUIT_RESULT => {
                let attempt = r.u8()?;
                let passed = match r.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(DecodeError::Invalid("bad bool")),
                };
                Packet::RecruitResult { attempt, passed, score: r.u8()?, total: r.u8()?, department: r.u8()? }
            }
            ty::MAIL => Packet::Mail {
                id: r.u8()?,
                from: r.str16(MAX_TEXT_BYTES)?,
                subject: r.str16(MAX_TEXT_BYTES)?,
                body: r.str16(MAX_MAIL_BYTES)?,
                action: r.u8()?,
                arg: r.u8()?,
            },
            ty::PORTAL_ACTION => Packet::PortalAction { token: r.u32()?, action: r.u8()?, arg: r.u8()? },
            ty::INVENTORY => {
                let n = r.u8()? as usize;
                if n > 8 {
                    return Err(DecodeError::Invalid("too many slots"));
                }
                let mut slots = Vec::with_capacity(n);
                for _ in 0..n {
                    slots.push(SlotInfo { kind: r.u8()?, id: r.u32()?, label: r.str16(MAX_TEXT_BYTES)? });
                }
                Packet::Inventory { slots }
            }
            ty::ITEM_ACTION => Packet::ItemAction { token: r.u32()?, action: r.u8()?, slot: r.u8()? },
            ty::COMPUTER => {
                let (handle, owner, locked) = (r.u16()?, r.u16()?, r.u8()? != 0);
                let n = r.u8()? as usize;
                if n > MAX_CONVS {
                    return Err(DecodeError::Invalid("too many conversations"));
                }
                let mut convs = Vec::with_capacity(n);
                for _ in 0..n {
                    convs.push(ConvEntry { conv: r.u16()?, unread: r.u8()?, title: r.str16(MAX_NICK_BYTES + 8)? });
                }
                Packet::Computer { handle, owner, locked, convs }
            }
            ty::COMPUTER_ACTION => Packet::ComputerAction {
                token: r.u32()?,
                action: r.u8()?,
                conv: r.u16()?,
                arg: r.u32()?,
                text: r.str16(MAX_CHAT_BYTES)?,
            },
            ty::DOORS => {
                let floor = r.u8()?;
                let n = r.u8()? as usize;
                let mut tiles = Vec::with_capacity(n);
                for _ in 0..n {
                    tiles.push((r.u8()?, r.u8()?));
                }
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many lifts"));
                }
                let mut lifts = Vec::with_capacity(n);
                for _ in 0..n {
                    lifts.push(Lift { floor: r.u8()?, target: r.u8()?, moving: r.u8()? != 0 });
                }
                Packet::Doors { floor, tiles, lifts }
            }
            ty::DOOR_ACTION => Packet::DoorAction { token: r.u32()? },
            ty::STATS => Packet::Stats {
                hunger: r.u8()?,
                energy: r.u8()?,
                stress: r.u8()?,
                bladder: r.u8()?,
                hygiene: r.u8()?,
                flags: r.u8()?,
                money: r.u32()?,
            },
            ty::SHELF => {
                let shelf = r.u8()?;
                let title = r.str16(MAX_TEXT_BYTES)?;
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many goods"));
                }
                let mut goods = Vec::with_capacity(n);
                for _ in 0..n {
                    goods.push(ShelfItem { kind: r.u8()?, price: r.u32()?, name: r.str16(MAX_TEXT_BYTES)? });
                }
                Packet::Shelf { shelf, title, goods }
            }
            ty::SHOP_TAKE => Packet::ShopTake { token: r.u32()?, shelf: r.u8()?, kind: r.u8()? },
            ty::CLOCK => Packet::Clock {
                day: r.u16()?,
                minute: r.u16()?,
                night: r.u8()? != 0,
                place: r.u8()?,
                arrive: r.u16()?,
                pay: r.u32()?,
                pay_minutes: r.u16()?,
                today_minutes: r.u16()?,
                mode: r.u8()?,
                depart: r.u16()?,
                money: r.u32()?,
                weather: r.u8()?,
                company: r.str16(64)?,
                founded: r.u8()? != 0,
                alarm: r.u8()?,
                skip: r.u8()?,
            },
            ty::FRIDGE => {
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many fridge items"));
                }
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push((r.u8()?, r.str16(64)?));
                }
                Packet::Fridge { items, milk: r.u8()?, water: r.u8()?, juice: r.u8()? }
            }
            ty::SKIP_WAIT => Packet::SkipWait { token: r.u32()? },
            ty::TASK_ACTION => Packet::TaskAction {
                token: r.u32()?,
                nonce: r.u16()?,
                action: r.u8()?,
                task: r.u16()?,
                arg: r.u8()?,
                text: r.str16(TASK_TEXT_MAX)?,
            },
            ty::TASK_BOARD => {
                let (dept, done, part, parts) = (r.u8()?, r.u16()?, r.u8()?, r.u8()?);
                let n = r.u8()? as usize;
                if n > MAX_MEMBERS {
                    return Err(DecodeError::Invalid("too many members"));
                }
                let mut members = Vec::with_capacity(n);
                for _ in 0..n {
                    members.push(r.str8()?);
                }
                let n = r.u8()? as usize;
                let mut tasks = Vec::with_capacity(n);
                for _ in 0..n {
                    tasks.push(TaskCard {
                        id: r.u16()?,
                        column: r.u8()?,
                        priority: r.u8()?,
                        comments: r.u8()?,
                        title: r.str16(TASK_TITLE_MAX)?,
                        author: r.str8()?,
                        assignee: r.str8()?,
                    });
                }
                Packet::TaskBoard { dept, done, part, parts, members, tasks }
            }
            ty::TASK_DETAIL => {
                let (id, desc) = (r.u16()?, r.str16(TASK_TEXT_MAX)?);
                let n = r.u8()? as usize;
                if n > DETAIL_COMMENTS {
                    return Err(DecodeError::Invalid("too many comments"));
                }
                let mut comments = Vec::with_capacity(n);
                for _ in 0..n {
                    comments.push((r.str8()?, r.str16(TASK_COMMENT_MAX)?));
                }
                Packet::TaskDetail { id, desc, comments }
            }
            ty::MAIL_ACTION => Packet::MailAction {
                token: r.u32()?,
                nonce: r.u16()?,
                action: r.u8()?,
                id: r.u16()?,
                to: r.str8()?,
                subject: r.str16(MAIL_SUBJECT_MAX)?,
                body: r.str16(MAIL_BODY_MAX)?,
            },
            ty::WORK_MAIL => Packet::WorkMail {
                id: r.u16()?,
                from: r.str8()?,
                to: r.str8()?,
                subject: r.str16(MAIL_SUBJECT_MAX)?,
                body: r.str16(MAIL_BODY_MAX)?,
                day: r.u16()?,
                minute: r.u16()?,
            },
            ty::MAIL_STATE => {
                let done = r.u16()?;
                let mut lists = [Vec::new(), Vec::new()];
                for list in &mut lists {
                    let n = r.u8()? as usize;
                    if n > MAX_MAIL_IDS {
                        return Err(DecodeError::Invalid("too many mail ids"));
                    }
                    for _ in 0..n {
                        list.push(r.u16()?);
                    }
                }
                let [ids, trashed] = lists;
                Packet::MailState { done, ids, trashed }
            }
            ty::VOICE => {
                let (token, seq, whisper) = (r.u32()?, r.u16()?, r.u8()?);
                Packet::Voice { token, seq, whisper, data: r.voice()? }
            }
            ty::DEPARTMENTS => {
                let n = r.u8()? as usize;
                if n > MAX_DEPARTMENTS {
                    return Err(DecodeError::Invalid("too many departments"));
                }
                let mut list = Vec::with_capacity(n);
                for _ in 0..n {
                    list.push(DepartmentInfo { id: r.u8()?, short: r.str8()?, name: r.str8()? });
                }
                Packet::Departments { list }
            }
            ty::VOICE_FROM => {
                let (speaker, seq, whisper) = (r.u16()?, r.u16()?, r.u8()?);
                Packet::VoiceFrom { speaker, seq, whisper, data: r.voice()? }
            }
            ty::SOUND => {
                let n = r.u8()? as usize;
                if n > MAX_SOUNDS {
                    return Err(DecodeError::Invalid("too many sounds"));
                }
                let mut sounds = Vec::with_capacity(n);
                for _ in 0..n {
                    sounds.push((r.u8()?, r.i32()?, r.i32()?));
                }
                Packet::Sound { sounds }
            }
            ty::FRIDGE_ACTION => Packet::FridgeAction { token: r.u32()?, action: r.u8()?, arg: r.u8()? },
            ty::LIGHTS => {
                let floor = r.u8()?;
                let n = r.u8()? as usize;
                if n > 64 {
                    return Err(DecodeError::Invalid("too many rooms"));
                }
                let mut rooms = Vec::with_capacity(n);
                for _ in 0..n {
                    rooms.push(r.u16()?);
                }
                Packet::Lights { floor, rooms }
            }
            ty::SMOKE => {
                let floor = r.u8()?;
                let n = r.u8()? as usize;
                if n > 64 {
                    return Err(DecodeError::Invalid("too many rooms"));
                }
                let mut rooms = Vec::with_capacity(n);
                for _ in 0..n {
                    rooms.push((r.u16()?, r.u8()?));
                }
                Packet::Smoke { floor, rooms }
            }
            ty::COMPANY_OFFERS => {
                let (name, part, parts) = (r.str16(64)?, r.u8()?, r.u8()?);
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many question sets"));
                }
                let mut sets = Vec::with_capacity(n);
                for _ in 0..n {
                    sets.push((r.str8()?, r.str16(64)?));
                }
                let n = r.u8()? as usize;
                if n > 16 {
                    return Err(DecodeError::Invalid("too many offers"));
                }
                let mut offers = Vec::with_capacity(n);
                for _ in 0..n {
                    offers.push(CompanyOffer {
                        id: r.u8()?,
                        places: r.u8()?,
                        department: r.u8()?,
                        set: r.str8()?,
                        title: r.str16(64)?,
                        description: r.str16(MAX_TEXT_BYTES)?,
                    });
                }
                Packet::CompanyOffers { name, part, parts, sets, offers }
            }
            ty::COMPANY_PEOPLE => {
                let n = r.u8()? as usize;
                if n > 20 {
                    return Err(DecodeError::Invalid("too many candidates"));
                }
                let mut candidates = Vec::with_capacity(n);
                for _ in 0..n {
                    candidates.push((r.u16()?, r.u8()?, r.u8()?, r.u8()?, r.str16(MAX_NICK_BYTES)?));
                }
                let n = r.u8()? as usize;
                if n > 30 {
                    return Err(DecodeError::Invalid("too many staff"));
                }
                let mut staff = Vec::with_capacity(n);
                for _ in 0..n {
                    staff.push((r.u16()?, r.u8()?, r.u16()?, r.str16(MAX_NICK_BYTES)?));
                }
                Packet::CompanyPeople { candidates, staff }
            }
            ty::COMPANY_ACTION => Packet::CompanyAction {
                token: r.u32()?,
                action: r.u8()?,
                target: r.u16()?,
                value: r.u8()?,
                text: r.str16(MAX_TEXT_BYTES)?,
            },
            ty::COMMUTE_CHOICE => Packet::CommuteChoice { token: r.u32()?, mode: r.u8()? },
            ty::CALENDAR => {
                let (mine_start, mine_topic) = (r.u16()?, r.u8()?);
                let n = r.u8()? as usize;
                if n > 64 {
                    return Err(DecodeError::Invalid("too many slots"));
                }
                let mut slots = Vec::with_capacity(n);
                for _ in 0..n {
                    slots.push((r.u16()?, r.u8()?));
                }
                Packet::Calendar { mine_start, mine_topic, slots }
            }
            ty::CALENDAR_BOOK => Packet::CalendarBook { token: r.u32()?, start: r.u16()?, topic: r.u8()? },
            ty::DIALOG => {
                let (id, npc, text) = (r.u8()?, r.u16()?, r.str16(MAX_TEXT_BYTES)?);
                let n = r.u8()? as usize;
                if n > MAX_OPTIONS {
                    return Err(DecodeError::Invalid("too many options"));
                }
                let mut options = Vec::with_capacity(n);
                for _ in 0..n {
                    options.push(r.str16(MAX_TEXT_BYTES)?);
                }
                Packet::Dialog { id, npc, text, options }
            }
            ty::DIALOG_ANSWER => Packet::DialogAnswer { token: r.u32()?, id: r.u8()?, choice: r.u8()? },
            ty::LUNCH_MENU => {
                let (state, dish, arrives) = (r.u8()?, r.u8()?, r.u16()?);
                let n = r.u8()? as usize;
                if n > 12 {
                    return Err(DecodeError::Invalid("too many dishes"));
                }
                let mut dishes = Vec::with_capacity(n);
                for _ in 0..n {
                    dishes.push(Dish {
                        kind: r.u8()?,
                        price: r.u32()?,
                        eta: r.u8()?,
                        name: r.str16(MAX_NICK_BYTES * 4)?,
                        restaurant: r.str16(MAX_NICK_BYTES * 4)?,
                    });
                }
                Packet::LunchMenu { state, dish, arrives, dishes }
            }
            ty::LUNCH_ORDER => Packet::LunchOrder { token: r.u32()?, dish: r.u8()? },
            ty::CHAT => {
                let conv = r.u16()?;
                let n = r.u8()? as usize;
                let mut messages = Vec::with_capacity(n.min(64));
                for _ in 0..n {
                    messages.push(ChatEntry {
                        id: r.u32()?,
                        from: r.u16()?,
                        nick: r.str16(MAX_NICK_BYTES)?,
                        text: r.str16(MAX_CHAT_BYTES)?,
                    });
                }
                Packet::Chat { conv, messages }
            }
            other => return Err(DecodeError::UnknownType(other)),
        };
        if !r.at_end() {
            return Err(DecodeError::Invalid("trailing bytes"));
        }
        Ok(p)
    }
}
