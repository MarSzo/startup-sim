//! Where things happen outside the building (the ground floor map's
//! "places"): the street and the tram line, the stops, the parking bays and
//! the bike rack, where the police and the fire brigade stop - plus a few
//! fixed points inside (the treats tray, the founder's spot, shop shelves).
//! Read from the map files, so moving the building needs no code changes.

use crate::map::{Map, Places, Rect, Tile};
use crate::sim::{Pos, TILE_UNITS};

#[derive(Debug, Clone)]
pub struct Outside {
    /// Map width in tiles (vehicles drive in from past the east edge).
    pub width: i32,
    pub street_y: i32,
    pub tram_y: i32,
    /// West end of the sidewalk: going home on foot.
    pub walk_home: Tile,
    /// Walking in from the west in the morning.
    pub walk_arrival: Tile,
    /// The taxi stand on the sidewalk (the taxi stops on the street by it).
    pub taxi: Tile,
    pub tram_stop: Tile,
    /// Parking bays: x and the top row of a 2-tile-deep bay.
    pub car_bays: Vec<Tile>,
    pub bike_rack: Vec<Tile>,
    /// Where the officer / the fire crew get out (the car stops on the
    /// street by it).
    pub police: Tile,
    pub fire: Tile,
}

fn tile(p: Option<[i32; 2]>, what: &str) -> Result<Tile, String> {
    p.map(|[x, y]| Tile { x, y }).ok_or_else(|| format!("ground floor places: missing '{what}'"))
}

impl Outside {
    pub fn from_map(m: &Map) -> Result<Outside, String> {
        let p: &Places = &m.places;
        let o = Outside {
            width: m.width,
            street_y: p.street_y.ok_or("ground floor places: missing 'street_y'")?,
            tram_y: p.tram_y.ok_or("ground floor places: missing 'tram_y'")?,
            walk_home: tile(p.walk_home, "walk_home")?,
            walk_arrival: tile(p.walk_arrival, "walk_arrival")?,
            taxi: tile(p.taxi, "taxi")?,
            tram_stop: tile(p.tram_stop, "tram_stop")?,
            car_bays: p.car_bays.iter().map(|&[x, y]| Tile { x, y }).collect(),
            bike_rack: p.bike_rack.iter().map(|&[x, y]| Tile { x, y }).collect(),
            police: tile(p.police, "police")?,
            fire: tile(p.fire, "fire")?,
        };
        if o.car_bays.is_empty() || o.bike_rack.is_empty() {
            return Err("ground floor places: no car bays / bike rack".into());
        }
        for t in [o.walk_home, o.walk_arrival, o.taxi, o.tram_stop, o.police, o.fire] {
            if m.is_blocked(t.x, t.y) {
                return Err(format!("ground floor places: {t:?} is blocked"));
            }
        }
        Ok(o)
    }

    /// A point on the street at column `x`.
    pub fn street(&self, x: i32) -> Pos {
        Pos::tile_center(x, self.street_y)
    }

    /// Past the east edge of the map, on the street (vehicles come in here).
    pub fn street_east(&self) -> Pos {
        self.street(self.width - 2)
    }

    /// Off the map to the west, `tiles` past the edge.
    pub fn street_west_off(&self, tiles: i32) -> Pos {
        Pos { x: -tiles * TILE_UNITS, y: self.street(0).y }
    }
}

/// Shop shelves (id -> area) from the floor's places.
pub fn shelf_areas(p: &Places) -> Vec<(u8, Rect)> {
    p.shelves
        .iter()
        .filter_map(|(k, a)| k.parse().ok().map(|id| (id, Rect { x: a[0], y: a[1], w: a[2], h: a[3] })))
        .collect()
}
