// https://tomassedovic.github.io/roguelike-tutorial/part-2-object-map.html
extern crate tcod;
extern crate rand;

use rand::Rng;
use std::cmp;
use tcod::colors::{Color};
use tcod::colors;
use tcod::console::*;
use tcod::map::{Map as FovMap, FovAlgorithm};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_THROUGH_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

const MAX_ROOM_MONSTERS: i32 = 3;

const PLAYER: usize = 0;

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32
}
impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x1: x, y1: y, x2: x + w, y2: y + h }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2) && (self.x2 >= other.x1) &&
            (self.y1 < other.y2) && (self.y2 >= other.y1)
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}
impl Tile {
    pub fn empty() -> Self {
        Tile {blocked: false, block_sight: false, explored: false}
    }

    pub fn wall() -> Self {
        Tile { blocked: true, block_sight: true, explored: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

type Map = Vec<Vec<Tile>>;

fn place_objects(room: Rect, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);
    for _i in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let mut monster = if rand::random::<f32>() < 0.8 {
            Object::new(x, y, 'o', "Orc", colors::DESATURATED_GREEN, true)
        } else {
            Object::new(x, y, 'T', "Troll", colors::DARKER_GREEN, true)
        };
        objects.push(monster);
    }
}

fn make_map(objects: &mut Vec<Object>) -> (Map, (i32, i32)) {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];
    let mut starting_position = (0, 0);

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);
        let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));
        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, objects);
            if rooms.is_empty() {
                starting_position = new_room.center();
            } else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                let (new_x, new_y) = new_room.center();
                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }
    let mut player = Object::new(starting_position.0, starting_position.1, '@', "Player", colors::WHITE, true);
    player.alive = true;
    objects.insert(PLAYER, player);
    (map, starting_position)
}

fn create_fov_map(map: &Map) -> FovMap {
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            fov_map.set(x, y, !map[x as usize][y as usize].block_sight,
                    !map[x as usize][y as usize].blocked);
        }
    }
    fov_map
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
  if map[x as usize][y as usize].blocked {
    return true;
  }
  objects.iter().any(|object| {
    object.blocks && object.pos() == (x, y)
  })
}

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
  let (x, y) = objects[id].pos();
  if !is_blocked(x + dx, y + dy, map, objects) {
    objects[id].set_pos(x + dx, y + dy);
  }
}

struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
}
impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x,
            y,
            char,
            color,
            name: name.into(),
            blocks,
            alive: false
        }
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
      (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
      self.x = x;
      self.y = y;
    }
}

fn main() {
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtco tutorial")
        .init();

    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);
    let mut objects = vec![];

    tcod::system::set_fps(LIMIT_FPS);
    let (mut map, (player_x, player_y)) = make_map(&mut objects);
    let mut fov_map = create_fov_map(&map);
    let mut previous_player_position = (-1, -1);

    while !root.window_closed() {
        {
            let player = &objects[PLAYER];
            let fov_recompute = previous_player_position != player.pos();
            render_all(&mut root, &mut con, &objects, &mut map, &mut fov_map, fov_recompute);
        }
        root.flush();

        for object in &objects {
            object.clear(&mut con);
        }
        {
          let mut player = &mut objects[PLAYER];
          previous_player_position = player.pos();
        }
        let player_action = handle_keys(&mut root, &map, &mut objects);
        if player_action == PlayerAction::Exit {
            break;
        }
    }
}

fn handle_keys(root: &mut Root, map: &Map, objects: &mut [Object]) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let player_alive = objects[PLAYER].alive;

    match (root.wait_for_keypress(true), player_alive) {
        (Key { code: Enter, alt: true, .. }, _) => {
            let fullscreen = root.is_fullscreen();
            root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        (Key { code: Escape, .. }, _) => {
            Exit
        },
        (Key { code: Up, .. }, true) => {
          move_by(PLAYER, 0, -1, map, objects);
          TookTurn
        },
        (Key { code: Down, .. }, true) => {
          move_by(PLAYER, 0, 1, map, objects);
          TookTurn
        },
        (Key { code: Left, .. }, true) => {
          move_by(PLAYER, -1, 0, map, objects);
          TookTurn
        },
        (Key { code: Right, .. }, true) => {
          move_by(PLAYER, 1, 0, map, objects);
          TookTurn
        },
        _ => DidntTakeTurn,
    }
}

fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &mut Map, fov_map: &mut FovMap, fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[PLAYER];
        fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_THROUGH_WALLS, FOV_ALGO);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = fov_map.is_in_fov(x, y);
            let wall = map[x as usize][y as usize].block_sight;
            let color = match (visible, wall) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                // inside fov:
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            let explored = &mut map[x as usize][y as usize].explored;
            if visible {
                *explored = true;
            }
            if *explored {
                con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }
    for object in objects {
        if fov_map.is_in_fov(object.x, object.y) {
            object.draw(con);
        }
    }
    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}
