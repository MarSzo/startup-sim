//! Golden packet samples shared with the GDScript client tests.

use super::*;

pub fn golden_samples() -> Vec<(&'static str, Packet)> {
    vec![
        (
            "connect",
            Packet::Connect {
                nonce: 0xDEADBEEF,
                nick: "Zażółć".into(),
                profile: Profile {
                    gender: gender::FEMALE,
                    age: 27,
                    city: "Łódź".into(),
                    email: "ola@poczta.pl".into(),
                    appearance: Appearance { skin: 1, hair_style: 4, hair_color: 2, shirt: 9, pants: 3 },
                },
                ticket: "0a1b2c".into(),
            },
        ),
        (
            "welcome",
            Packet::Welcome { nonce: 0xDEADBEEF, player_id: 7, token: 0x01020304, tick_hz: 20, input_hz: 60, map_crc: 0xCAFEBABE, server_tick: 1234 },
        ),
        ("reject", Packet::Reject { reason: reject::SERVER_FULL }),
        ("input", Packet::Input { token: 0x01020304, ack_tick: 1200, last_seq: 99, inputs: vec![0, 1, 9, 6] }),
        (
            "snapshot",
            Packet::Snapshot {
                tick: 1234,
                last_input_seq: 99,
                frag_idx: 0,
                frag_cnt: 1,
                self_x: 10_000,
                self_y: -5,
                floor: 1,
                room: 6,
                self_lock: 2,
                self_prev_input: 17,
                self_access: 5,
                self_slow: 1,
                self_activity: 3,
                entities: vec![
                    EntityState { id: 3, kind: kind::PLAYER, x: 4096, y: 8192, flags: 0b101, held: 3, activity: 1 },
                    EntityState { id: 65535, kind: kind::NPC, x: -1, y: 2_000_000, flags: 0x40, held: 0, activity: 0 },
                ],
            },
        ),
        (
            "player_info",
            Packet::PlayerInfo {
                players: vec![
                    PlayerInfoEntry {
                        id: 3,
                        nick: "Ala".into(),
                        department: 1,
                        gender: gender::FEMALE,
                        appearance: Appearance { skin: 2, hair_style: 1, hair_color: 3, shirt: 4, pants: 0 },
                    },
                    PlayerInfoEntry { id: 4, nick: "bot_07".into(), department: 0, gender: gender::MALE, appearance: Appearance::default() },
                ],
            },
        ),
        ("info_request", Packet::InfoRequest { token: 0x01020304, ids: vec![3, 4, 500] }),
        ("ping", Packet::Ping { token: 0x01020304, client_time: 777_000 }),
        ("pong", Packet::Pong { client_time: 777_000, server_tick: 1234 }),
        ("disconnect", Packet::Disconnect { token: 0x01020304, reason: disconnect::TIMEOUT }),
        ("say", Packet::Say { id: 61440, text: "Dzień dobry! Proszę za mną.".into() }),
        (
            "job_offers",
            Packet::JobOffers {
                offers: vec![
                    OfferInfo {
                        id: 1,
                        department: 1,
                        applied: true,
                        vacancies: 2,
                        company: "Startup Sim sp. z o.o.".into(),
                        title: "Programista/ka".into(),
                        description: "Owocowe czwartki.".into(),
                    },
                    OfferInfo {
                        id: 12,
                        department: 0,
                        applied: false,
                        vacancies: 0,
                        company: "Pizzeria u Stefana".into(),
                        title: "Dostawca/Dostawczyni".into(),
                        description: "Własny rower.".into(),
                    },
                ],
            },
        ),
        ("apply", Packet::Apply { token: 0x01020304, offer: 2, motivation: "Lubię kawę i wyzwania.".into() }),
        (
            "question",
            Packet::Question {
                attempt: 3,
                index: 1,
                total: 3,
                text: "Co oznacza kod HTTP 404?".into(),
                options: vec!["Nie znaleziono zasobu".into(), "Skończyła się kawa".into(), "Wszystko w porządku".into()],
            },
        ),
        ("answer", Packet::Answer { token: 0x01020304, attempt: 3, index: 1, choice: 2 }),
        ("recruit_result", Packet::RecruitResult { attempt: 3, passed: true, score: 2, total: 3, department: 1 }),
        (
            "mail",
            Packet::Mail {
                id: 2,
                from: "Startup Sim — Rekrutacja".into(),
                subject: "Zaproszenie na rozmowę".into(),
                body: "Cześć Ola! Zapraszamy na rozmowę online.".into(),
                action: portal_action::JOIN_INTERVIEW,
                arg: 1,
            },
        ),
        ("portal_action", Packet::PortalAction { token: 0x01020304, action: portal_action::GO_TO_OFFICE, arg: 0 }),
        (
            "inventory",
            Packet::Inventory {
                slots: vec![
                    SlotInfo { kind: 3, id: 77, label: "Laptop: Ola".into() },
                    SlotInfo { kind: 2, id: 76, label: "Ola · IT / Produkt".into() },
                    SlotInfo::default(),
                    SlotInfo::default(),
                ],
            },
        ),
        ("item_action", Packet::ItemAction { token: 0x01020304, action: item_action::TAKE_OUT, slot: 2 }),
        (
            "computer",
            Packet::Computer {
                handle: 0xE001,
                owner: 3,
                locked: false,
                convs: vec![
                    ConvEntry { conv: 1, unread: 0, title: "#ogólny".into() },
                    ConvEntry { conv: 17, unread: 2, title: "#it-produkt".into() },
                    ConvEntry { conv: 0x8004, unread: 1, title: "Kuba".into() },
                ],
            },
        ),
        (
            "computer_action",
            Packet::ComputerAction { token: 0x01020304, action: computer_action::SEND, conv: 17, arg: 42, text: "Kto zjadł mój jogurt?".into() },
        ),
        ("stats", Packet::Stats { hunger: 35, energy: 80, stress: 12, bladder: 64, hygiene: 22, flags: STATS_DIRTY_HANDS, money: 187_50 }),
        ("doors", Packet::Doors { floor: 1, tiles: vec![(46, 27), (54, 31)], lift_floor: 0, lift_target: 1, lift_moving: true }),
        ("door_action", Packet::DoorAction { token: 0x01020304 }),
        (
            "shelf",
            Packet::Shelf {
                shelf: 1,
                title: "Kanapki".into(),
                goods: vec![
                    ShelfItem { kind: 10, price: 12_00, name: "Kanapka z serem".into() },
                    ShelfItem { kind: 11, price: 14_00, name: "Kanapka z szynką".into() },
                ],
            },
        ),
        ("shop_take", Packet::ShopTake { token: 0x01020304, shelf: 1, kind: 11 }),
        (
            "clock",
            Packet::Clock {
                day: 2,
                minute: 8 * 60 + 12,
                night: false,
                place: place::COMMUTING,
                arrive: 9 * 60 + 5,
                pay: 230_00,
                pay_minutes: 460,
                today_minutes: 0,
                mode: 2,
                depart: 8 * 60 + 40,
                money: 186_00,
                weather: 3,
                company: "Pixel Pierogi sp. z o.o.".into(),
                founded: true,
                alarm: 1,
                skip: 1,
            },
        ),
        ("commute_choice", Packet::CommuteChoice { token: 0x01020304, mode: 5 }),
        (
            "calendar",
            Packet::Calendar { mine_start: 14 * 60, mine_topic: 1, slots: vec![(600, slot::PAST), (630, slot::FREE), (660, slot::TAKEN), (840, slot::MINE)] },
        ),
        ("calendar_book", Packet::CalendarBook { token: 0x01020304, start: 14 * 60, topic: 2 }),
        (
            "dialog",
            Packet::Dialog {
                id: 3,
                npc: 61444,
                text: "Podwyżka? Proszę mnie przekonać.".into(),
                options: vec!["Pracuję tu od początku.".into(), "Bo kawa podrożała.".into()],
            },
        ),
        ("dialog_answer", Packet::DialogAnswer { token: 0x01020304, id: 3, choice: 1 }),
        (
            "lunch_menu",
            Packet::LunchMenu {
                state: 1,
                dish: 28,
                arrives: 12 * 60 + 40,
                dishes: vec![Dish { kind: 28, price: 24_00, eta: 40, name: "Pierogi ruskie".into(), restaurant: "Pierogarnia u Zosi".into() }],
            },
        ),
        ("lunch_order", Packet::LunchOrder { token: 0x01020304, dish: 29 }),
        (
            "company_offers",
            Packet::CompanyOffers {
                name: "Pixel Pierogi sp. z o.o.".into(),
                part: 0,
                parts: 1,
                sets: vec![("programming".into(), "Programowanie".into()), ("general".into(), "Ogólne".into())],
                offers: vec![
                    CompanyOffer { id: 1, places: 2, department: 1, set: "programming".into(), title: "Programista/ka".into(), description: "Piszemy w Ruście.".into() },
                    CompanyOffer { id: 20, places: 0, department: 2, set: "general".into(), title: "Office manager".into(), description: String::new() },
                ],
            },
        ),
        (
            "company_people",
            Packet::CompanyPeople {
                candidates: vec![(7, 1, 3, 3, "Bob".into())],
                staff: vec![(3, 1, 2, "Ala".into()), (4, 2, 5, "Kuba".into())],
            },
        ),
        ("company_action", Packet::CompanyAction { token: 0x01020304, action: 3, target: 1, value: 2, text: String::new() }),
        ("smoke", Packet::Smoke { floor: 1, rooms: vec![(9, 40), (33, 200)] }),
        ("lights", Packet::Lights { floor: 1, rooms: vec![5, 12] }),
        ("fridge", Packet::Fridge { items: vec![(11, "Kanapka z szynką (Ola)".into())], milk: 7, water: 4, juice: 2 }),
        ("fridge_action", Packet::FridgeAction { token: 0x01020304, action: 2, arg: 0 }),
        ("skip_wait", Packet::SkipWait { token: 0x01020304 }),
        ("task_action", Packet::TaskAction { token: 0x01020304, nonce: 7, action: 1, task: 0, arg: 2, text: "Naprawić logowanie\nPo zmianie hasła.".into() }),
        (
            "task_board",
            Packet::TaskBoard {
                dept: 1,
                done: 7,
                part: 0,
                parts: 1,
                members: vec!["Ola".into(), "Kuba".into()],
                tasks: vec![TaskCard { id: 3, column: 1, priority: 2, comments: 1, title: "Naprawić logowanie".into(), author: "Ola".into(), assignee: "Kuba".into() }],
            },
        ),
        ("task_detail", Packet::TaskDetail { id: 3, desc: "Po zmianie hasła.".into(), comments: vec![("Kuba".into(), "Zrobione".into())] }),
        ("mail_action", Packet::MailAction { token: 0x01020304, nonce: 4, action: 1, id: 0, to: "Kuba".into(), subject: "Kawa?".into(), body: "O 12 w kuchni.".into() }),
        ("work_mail", Packet::WorkMail { id: 5, from: "HR".into(), to: "Kuba".into(), subject: "Witamy!".into(), body: "Miłego pierwszego dnia.".into(), day: 2, minute: 540 }),
        ("mail_state", Packet::MailState { done: 4, ids: vec![1, 2, 5], trashed: vec![2] }),
        ("voice", Packet::Voice { token: 0x01020304, seq: 9, whisper: 1, data: vec![0x10, 0x00, 0x05, 0x7f, 0x80] }),
        ("voice_from", Packet::VoiceFrom { speaker: 3, seq: 9, whisper: 0, data: vec![0x10, 0x00, 0x05, 0x7f, 0x80] }),
        ("sound", Packet::Sound { sounds: vec![(1, 12_288, -256), (17, 0, 65_536)] }),
        (
            "chat",
            Packet::Chat {
                conv: 17,
                messages: vec![
                    ChatEntry { id: 5, from: 3, nick: "Ola".into(), text: "Deploy w piątek?".into() },
                    ChatEntry { id: 6, from: 4, nick: "Kuba".into(), text: "Nigdy w życiu.".into() },
                ],
            },
        ),
    ]
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
