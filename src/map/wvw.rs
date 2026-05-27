//! WvW map identity, landmark data, and zone-name resolution.
//! Ported from `axipulse/src/shared/wvwLandmarks.ts` and `mapUtils.ts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WvwMap {
    EternalBattlegrounds,
    GreenBorderlands,
    BlueBorderlands,
    RedBorderlands,
    EdgeOfTheMists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandmarkType { Keep, Tower, Camp, Ruins, Named }

#[derive(Debug, Clone, Copy)]
pub struct Landmark {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub kind: LandmarkType,
}

const ZONE_PREFIXES: &[&str] = &[
    "Detailed WvW - ",
    "World vs World - ",
    "WvW - ",
];

fn strip_prefix(zone: &str) -> &str {
    for p in ZONE_PREFIXES {
        if let Some(rest) = zone.strip_prefix(p) {
            return rest;
        }
    }
    zone
}

pub fn resolve_map_from_zone(zone: &str) -> Option<WvwMap> {
    let clean = strip_prefix(zone).to_lowercase();
    if clean.contains("eternal") || clean == "ebg" {
        Some(WvwMap::EternalBattlegrounds)
    } else if clean.contains("edge of the mists") || clean == "eotm" {
        Some(WvwMap::EdgeOfTheMists)
    } else if clean.contains("green") {
        Some(WvwMap::GreenBorderlands)
    } else if clean.contains("blue") {
        Some(WvwMap::BlueBorderlands)
    } else if clean.contains("red") {
        Some(WvwMap::RedBorderlands)
    } else {
        None
    }
}

pub fn landmarks(map: WvwMap) -> &'static [Landmark] {
    match map {
        WvwMap::EternalBattlegrounds => EBG,
        WvwMap::GreenBorderlands => GREEN_ALPINE,
        WvwMap::BlueBorderlands => BLUE_ALPINE,
        WvwMap::RedBorderlands => RED_DESERT,
        WvwMap::EdgeOfTheMists => EDGE_OF_THE_MISTS,
    }
}

const EBG: &[Landmark] = &[
    Landmark { name: "Stonemist Castle",  x: 370.0, y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Overlook",          x: 400.0, y: 230.0, kind: LandmarkType::Keep },
    Landmark { name: "Lowlands",          x: 151.0, y: 569.0, kind: LandmarkType::Keep },
    Landmark { name: "Valley",            x: 592.0, y: 567.0, kind: LandmarkType::Keep },
    Landmark { name: "Mendon's Gap",      x: 290.0, y: 175.0, kind: LandmarkType::Tower },
    Landmark { name: "Veloka Slope",      x: 470.0, y: 200.0, kind: LandmarkType::Tower },
    Landmark { name: "Speldan Clearcut",  x: 206.0, y: 200.0, kind: LandmarkType::Tower },
    Landmark { name: "Wildcreek Run",     x: 221.0, y: 446.0, kind: LandmarkType::Tower },
    Landmark { name: "Aldon's Ledge",     x: 106.0, y: 487.0, kind: LandmarkType::Tower },
    Landmark { name: "Klovan Gully",      x: 283.0, y: 557.0, kind: LandmarkType::Tower },
    Landmark { name: "Jerrifer's Slough", x: 198.0, y: 636.0, kind: LandmarkType::Tower },
    Landmark { name: "Quentin Lake",      x: 441.0, y: 592.0, kind: LandmarkType::Tower },
    Landmark { name: "Langor Gulch",      x: 581.0, y: 657.0, kind: LandmarkType::Tower },
    Landmark { name: "Bravost Escarpment",x: 635.0, y: 487.0, kind: LandmarkType::Tower },
    Landmark { name: "Durios Gulch",      x: 512.0, y: 445.0, kind: LandmarkType::Tower },
    Landmark { name: "Ogrewatch Cut",     x: 468.0, y: 307.0, kind: LandmarkType::Tower },
    Landmark { name: "Anzalias Pass",     x: 287.0, y: 314.0, kind: LandmarkType::Tower },
    Landmark { name: "Pangloss Rise",     x: 541.0, y: 229.0, kind: LandmarkType::Camp },
    Landmark { name: "Danelon Passage",   x: 485.0, y: 673.0, kind: LandmarkType::Camp },
    Landmark { name: "Golanta Clearing",  x: 290.0, y: 644.0, kind: LandmarkType::Camp },
    Landmark { name: "Umberglade Woods",  x: 595.0, y: 402.0, kind: LandmarkType::Camp },
    Landmark { name: "Rogue's Quarry",    x: 143.0, y: 397.0, kind: LandmarkType::Camp },
];

const GREEN_ALPINE: &[Landmark] = &[
    Landmark { name: "Dreadfall Bay",         x: 48.0,  y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Shadaran Hills",        x: 501.0, y: 419.0, kind: LandmarkType::Keep },
    Landmark { name: "Garrison",              x: 257.0, y: 325.0, kind: LandmarkType::Keep },
    Landmark { name: "Bluebriar",             x: 182.0, y: 515.0, kind: LandmarkType::Tower },
    Landmark { name: "Sunnyhill",             x: 132.0, y: 251.0, kind: LandmarkType::Tower },
    Landmark { name: "Redlake",               x: 364.0, y: 530.0, kind: LandmarkType::Tower },
    Landmark { name: "Cragtop",               x: 385.0, y: 241.0, kind: LandmarkType::Tower },
    Landmark { name: "Titanpaw",              x: 262.0, y: 73.0,  kind: LandmarkType::Camp },
    Landmark { name: "Bluevale Refuge",       x: 95.0,  y: 540.0, kind: LandmarkType::Camp },
    Landmark { name: "Faithleap",             x: 85.0,  y: 276.0, kind: LandmarkType::Camp },
    Landmark { name: "Foghaven",              x: 455.0, y: 270.0, kind: LandmarkType::Camp },
    Landmark { name: "Hero's Lodge",          x: 263.0, y: 660.0, kind: LandmarkType::Camp },
    Landmark { name: "Redwater Lowlands",     x: 453.0, y: 549.0, kind: LandmarkType::Camp },
    Landmark { name: "Temple of the Fallen",  x: 259.0, y: 515.0, kind: LandmarkType::Ruins },
    Landmark { name: "Cohen's Overlook",      x: 312.0, y: 393.0, kind: LandmarkType::Ruins },
    Landmark { name: "Gertzz's Estate",       x: 217.0, y: 382.0, kind: LandmarkType::Ruins },
    Landmark { name: "Patrick's Ascent",      x: 320.0, y: 468.0, kind: LandmarkType::Ruins },
    Landmark { name: "Norfolk's Hollow",      x: 197.0, y: 460.0, kind: LandmarkType::Ruins },
];

const BLUE_ALPINE: &[Landmark] = &[
    Landmark { name: "Ascension Bay",         x: 48.0,  y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Askalion Hills",        x: 501.0, y: 419.0, kind: LandmarkType::Keep },
    Landmark { name: "Garrison",              x: 257.0, y: 325.0, kind: LandmarkType::Keep },
    Landmark { name: "Redbriar",              x: 182.0, y: 515.0, kind: LandmarkType::Tower },
    Landmark { name: "Woodhaven",             x: 132.0, y: 251.0, kind: LandmarkType::Tower },
    Landmark { name: "Greenlake",             x: 364.0, y: 530.0, kind: LandmarkType::Tower },
    Landmark { name: "Dawn's Eyrie",          x: 385.0, y: 241.0, kind: LandmarkType::Tower },
    Landmark { name: "Spiritholme",           x: 262.0, y: 73.0,  kind: LandmarkType::Camp },
    Landmark { name: "Redvale Refuge",        x: 95.0,  y: 540.0, kind: LandmarkType::Camp },
    Landmark { name: "Godslore",              x: 85.0,  y: 276.0, kind: LandmarkType::Camp },
    Landmark { name: "Stargrove",             x: 455.0, y: 270.0, kind: LandmarkType::Camp },
    Landmark { name: "Champion's Demesne",    x: 263.0, y: 660.0, kind: LandmarkType::Camp },
    Landmark { name: "Greenwater Lowlands",   x: 453.0, y: 549.0, kind: LandmarkType::Camp },
    Landmark { name: "Temple of Lost Prayers",x: 259.0, y: 515.0, kind: LandmarkType::Ruins },
    Landmark { name: "Orchard Overlook",      x: 312.0, y: 393.0, kind: LandmarkType::Ruins },
    Landmark { name: "Bauer's Estate",        x: 217.0, y: 382.0, kind: LandmarkType::Ruins },
    Landmark { name: "Carver's Ascent",       x: 320.0, y: 468.0, kind: LandmarkType::Ruins },
    Landmark { name: "Battle's Hollow",       x: 197.0, y: 460.0, kind: LandmarkType::Ruins },
];

const RED_DESERT: &[Landmark] = &[
    Landmark { name: "Blistering Undercroft", x: 28.0,  y: 409.0, kind: LandmarkType::Keep },
    Landmark { name: "Stoic Rampart",         x: 370.0, y: 272.0, kind: LandmarkType::Keep },
    Landmark { name: "Osprey's Palace",       x: 700.0, y: 427.0, kind: LandmarkType::Keep },
    Landmark { name: "O'del Academy",         x: 151.0, y: 134.0, kind: LandmarkType::Tower },
    Landmark { name: "Eternal Necropolis",    x: 590.0, y: 155.0, kind: LandmarkType::Tower },
    Landmark { name: "Crankshaft Depot",      x: 485.0, y: 610.0, kind: LandmarkType::Tower },
    Landmark { name: "Parched Outpost",       x: 251.0, y: 579.0, kind: LandmarkType::Tower },
    Landmark { name: "Hamm's Lab",            x: 367.0, y: 130.0, kind: LandmarkType::Camp },
    Landmark { name: "Bauer Farmstead",       x: 654.0, y: 569.0, kind: LandmarkType::Camp },
    Landmark { name: "McLain's Encampment",   x: 90.0,  y: 576.0, kind: LandmarkType::Camp },
    Landmark { name: "Roy's Refuge",          x: 704.0, y: 259.0, kind: LandmarkType::Camp },
    Landmark { name: "Boettiger's Hideaway",  x: 23.0,  y: 256.0, kind: LandmarkType::Camp },
    Landmark { name: "Dustwhisper Well",      x: 376.0, y: 707.0, kind: LandmarkType::Camp },
    Landmark { name: "Higgins's Ascent",      x: 415.0, y: 547.0, kind: LandmarkType::Ruins },
    Landmark { name: "Bearce's Dwelling",     x: 301.0, y: 440.0, kind: LandmarkType::Ruins },
    Landmark { name: "Zak's Overlook",        x: 433.0, y: 444.0, kind: LandmarkType::Ruins },
    Landmark { name: "Darra's Maze",          x: 289.0, y: 513.0, kind: LandmarkType::Ruins },
    Landmark { name: "Tilly's Encampment",    x: 369.0, y: 365.0, kind: LandmarkType::Ruins },
];

// Derived from GW2 API /v2/wvw/objectives (map_id 968). Objective coords
// are in continent-pixel space; pixel coords below are
//   px = (coord[0] - 5994) / 3072 * 2667
//   py = (coord[1] - 8446) / 3072 * 2734.5
// (matching the EI combat-replay size of 3556x3646 scaled by 0.75x).
const EDGE_OF_THE_MISTS: &[Landmark] = &[
    Landmark { name: "Overgrown Fane",           x: 1318.0, y: 217.0,  kind: LandmarkType::Keep },
    Landmark { name: "Arid Fortress",            x: 690.0,  y: 1753.0, kind: LandmarkType::Keep },
    Landmark { name: "Thunder Hollow",           x: 2271.0, y: 1941.0, kind: LandmarkType::Keep },
    Landmark { name: "Inferno's Needle",         x: 1331.0, y: 1935.0, kind: LandmarkType::Tower },
    Landmark { name: "Tytone Perch",             x: 1838.0, y: 1080.0, kind: LandmarkType::Tower },
    Landmark { name: "Stonegaze Spire",          x: 839.0,  y: 1086.0, kind: LandmarkType::Tower },
    Landmark { name: "Inferno's Needle Reactor", x: 1331.0, y: 1621.0, kind: LandmarkType::Camp },
    Landmark { name: "Tytone Perch Reactor",     x: 1553.0, y: 1251.0, kind: LandmarkType::Camp },
    Landmark { name: "Stonegaze Spire Reactor",  x: 1108.0, y: 1240.0, kind: LandmarkType::Camp },
];
