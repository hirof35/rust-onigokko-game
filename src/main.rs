use macroquad::prelude::*;
use macroquad::audio::{load_sound, play_sound, PlaySoundParams};
use std::collections::VecDeque;

// --- 定数と列挙型 ---
const TILE_SIZE: f32 = 40.0;
const MAP_WIDTH: usize = 20;
const MAP_HEIGHT: usize = 15;

#[derive(PartialEq, Clone, Copy)]
enum State {
    Title,
    Game,
    Result,
}

// シーン間で共有するデータ
struct GameData {
    last_score: i32,
    high_score: i32,
}

// --- マップデータ ---
const MAP_LAYOUT: [&str; 15] = [
    "11111111111111111111",
    "10000000010000000001",
    "10111110010011111001",
    "10000000000000000001",
    "10111111001111111001",
    "10000001000100000001",
    "11110001000100011111",
    "10000001111100000001",
    "10111100000000111101",
    "10000001111100000001",
    "10111111001111111001",
    "10000000000000000001",
    "10111110010011111001",
    "10000000010000000001",
    "11111111111111111111",
];

// --- 幾何学・アルゴリズム補助関数 ---

fn line_intersects_line(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> bool {
    let d = (p4.y - p3.y) * (p2.x - p1.x) - (p4.x - p3.x) * (p2.y - p1.y);
    if d == 0.0 { return false; }
    let ua = ((p4.x - p3.x) * (p1.y - p3.y) - (p4.y - p3.y) * (p1.x - p3.x)) / d;
    let ub = ((p2.x - p1.x) * (p1.y - p3.y) - (p2.y - p1.y) * (p1.x - p3.x)) / d;
    ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0
}

fn line_intersects_rect(p1: Vec2, p2: Vec2, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    if (p1.x >= rx && p1.x <= rx + rw && p1.y >= ry && p1.y <= ry + rh) ||
       (p2.x >= rx && p2.x <= rx + rw && p2.y >= ry && p2.y <= ry + rh) {
        return true;
    }
    let top1 = Vec2::new(rx, ry);
    let top2 = Vec2::new(rx + rw, ry);
    let bottom1 = Vec2::new(rx, ry + rh);
    let bottom2 = Vec2::new(rx + rw, ry + rh);
    
    line_intersects_line(p1, p2, top1, top2) ||
    line_intersects_line(p1, p2, top1, bottom1) ||
    line_intersects_line(p1, p2, top2, bottom2) ||
    line_intersects_line(p1, p2, bottom1, bottom2)
}

fn rect_intersects(r1_x: f32, r1_y: f32, r1_w: f32, r1_h: f32, r2_x: f32, r2_y: f32, r2_w: f32, r2_h: f32) -> bool {
    r1_x < r2_x + r2_w && r1_x + r1_w > r2_x && r1_y < r2_y + r2_h && r1_y + r1_h > r2_y
}

fn find_path(grid: &[[i32; 20]; 15], start: (usize, usize), target: (usize, usize)) -> Vec<(usize, usize)> {
    if start == target { return vec![start]; }
    let mut queue = VecDeque::new();
    let mut visited = [[false; 20]; 15];
    let mut parent = [[None; 20]; 15];
    
    queue.push_back(start);
    visited[start.1][start.0] = true;
    
    let mut found = false;
    while let Some((cx, cy)) = queue.pop_front() {
        if (cx, cy) == target { found = true; break; }
        
        for &(dx, dy) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0 && nx < 20 && ny >= 0 && ny < 15 {
                let nx = nx as usize;
                let ny = ny as usize;
                if grid[ny][nx] == 0 && !visited[ny][nx] {
                    visited[ny][nx] = true;
                    parent[ny][nx] = Some((cx, cy));
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    if !found { return vec![]; }
    
    let mut path = Vec::new();
    let mut curr = target;
    path.push(curr);
    while let Some(p) = parent[curr.1][curr.0] {
        path.push(p);
        curr = p;
    }
    path.reverse();
    path
}

// --- ゲーム本編のデータ構造 ---
struct GameScene {
    player_x: f32,
    player_y: f32,
    enemy_x: f32,
    enemy_y: f32,
    stamina: f32,
    was_spotted: bool,
    path: Vec<(usize, usize)>,
    path_update_timer: f32,
    survival_timer: f32,
    grid: [[i32; 20]; 15],
    walls: Vec<(f32, f32)>,
}

impl GameScene {
    fn new() -> Self {
        let mut grid = [[0; 20]; 15];
        let mut walls = Vec::new();
        for y in 0..15 {
            for x in 0..20 {
                if MAP_LAYOUT[y].chars().nth(x).unwrap() == '1' {
                    grid[y][x] = 1;
                    walls.push((x as f32 * TILE_SIZE, y as f32 * TILE_SIZE));
                }
            }
        }
        Self {
            player_x: 400.0,
            player_y: 300.0,
            enemy_x: 60.0,
            enemy_y: 60.0,
            stamina: 1.0,
            was_spotted: false,
            path: Vec::new(),
            path_update_timer: 0.0,
            survival_timer: 0.0,
            grid,
            walls,
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Rust SHADOW CHASE".to_owned(),
        window_width: (MAP_WIDTH as f32 * TILE_SIZE) as i32,
        window_height: (MAP_HEIGHT as f32 * TILE_SIZE) as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let bgm = load_sound("bgm.mp3").await.ok();
    let spotted_se = load_sound("damage.mp3").await.ok();

    let mut state = State::Title;
    let mut data = GameData { last_score: 0, high_score: 0 };
    let mut game = GameScene::new();
    let mut bgm_started = false;

    // Siv3DのDimgray相当の色定義
    let dim_gray = Color::from_rgba(105, 105, 105, 255);

    loop {
        let dt = get_frame_time();

        match state {
            State::Title => {
                if is_mouse_button_pressed(MouseButton::Left) || is_key_pressed(KeyCode::Enter) {
                    game = GameScene::new();
                    state = State::Game;
                    bgm_started = false;
                }
                
                clear_background(Color::from_rgba(25, 25, 51, 255));
                draw_text("Rust SHADOW CHASE", 160.0, 220.0, 50.0, ORANGE);
                draw_text("Press Enter or Click to Start", 250.0, 340.0, 20.0, WHITE);
                if data.high_score > 0 {
                    draw_text(&format!("HIGH SCORE: {}", data.high_score), 320.0, 420.0, 20.0, GOLD);
                }
            }
            
            State::Game => {
                if let Some(sound) = &bgm {
                    if !bgm_started {
                        // 【修正】 &sound に変更
                        play_sound(&sound, PlaySoundParams { looped: true, volume: 0.4 });
                        bgm_started = true;
                    }
                }

                let is_dashing = (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)) && game.stamina > 0.0;
                let current_speed = if is_dashing { 380.0 } else { 220.0 };
                game.stamina = (game.stamina + if is_dashing { -0.5 } else { 0.2 } * dt).clamp(0.0, 1.0);
                
                game.survival_timer += dt;

                let mut move_vec = Vec2::ZERO;
                if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) { move_vec.y -= 1.0; }
                if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) { move_vec.y += 1.0; }
                if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) { move_vec.x -= 1.0; }
                if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { move_vec.x += 1.0; }

                if move_vec != Vec2::ZERO {
                    // 斜め移動でも速度が一定になるように正規化
                    let delta = move_vec.normalize() * current_speed * dt;
                    let p_size = 30.0; // プレイヤーのサイズ

                    // --- X軸の移動と衝突解決 ---
                    game.player_x += delta.x;
                    for &(wx, wy) in &game.walls {
                        if rect_intersects(game.player_x, game.player_y, p_size, p_size, wx, wy, TILE_SIZE, TILE_SIZE) {
                            if delta.x > 0.0 {
                                // 右に動いて衝突したら、壁の左端に密着させる
                                game.player_x = wx - p_size;
                            } else if delta.x < 0.0 {
                                // 左に動いて衝突したら、壁の右端に密着させる
                                game.player_x = wx + TILE_SIZE;
                            }
                        }
                    }

                    // --- Y軸の移動と衝突解決 ---
                    game.player_y += delta.y;
                    for &(wx, wy) in &game.walls {
                        if rect_intersects(game.player_x, game.player_y, p_size, p_size, wx, wy, TILE_SIZE, TILE_SIZE) {
                            if delta.y > 0.0 {
                                // 下に動いて衝突したら、壁の上端に密着させる
                                game.player_y = wy - p_size;
                            } else if delta.y < 0.0 {
                                // 上に動いて衝突したら、壁の下端に密着させる
                                game.player_y = wy + TILE_SIZE;
                            }
                        }
                    }
                }

                // 3. 視界判定 (Line of Sight) と 近接察知
                let p_center = Vec2::new(game.player_x + 15.0, game.player_y + 15.0);
                let e_center = Vec2::new(game.enemy_x + 15.0, game.enemy_y + 15.0);
                
                // プレイヤーと鬼の距離を計算
                let distance = e_center.distance(p_center);
                // 察知範囲（例：120ピクセル = 3マス分くらい）
                let alert_radius = 120.0; 

                let mut blocked = true;

                if distance <= alert_radius {
                    // 範囲内にいるなら、壁があっても強制的に察知（blocked = false にする）
                    blocked = false;
                } else {
                    // 範囲外なら、通常通り視線が通っているかチェック
                    blocked = false; // 一旦通っていると仮定して、壁があれば true にする
                    for &(wx, wy) in &game.walls {
                        if line_intersects_rect(e_center, p_center, wx, wy, TILE_SIZE, TILE_SIZE) {
                            blocked = true;
                            break;
                        }
                    }
                }

                // 見つかった瞬間のSE
                if !game.was_spotted && !blocked {
                    if let Some(sound) = &spotted_se {
                        play_sound(sound, PlaySoundParams { looped: false, volume: 0.6 });
                    }
                }
                game.was_spotted = !blocked;

                game.path_update_timer -= dt;
                if !blocked && game.path_update_timer <= 0.0 {
                    let s_grid = ((e_center.x / TILE_SIZE) as usize, (e_center.y / TILE_SIZE) as usize);
                    let t_grid = ((p_center.x / TILE_SIZE) as usize, (p_center.y / TILE_SIZE) as usize);
                    game.path = find_path(&game.grid, s_grid, t_grid);
                    game.path_update_timer = 0.15;
                }

                if game.path.len() > 1 {
                    let target = Vec2::new(
                        game.path[1].0 as f32 * TILE_SIZE + 20.0,
                        game.path[1].1 as f32 * TILE_SIZE + 20.0
                    );
                    let enemy_speed = if blocked { 100.0 } else { 200.0 };
                    let to_target = target - e_center;
                    
                    if to_target != Vec2::ZERO {
                        let e_delta = to_target.normalize() * enemy_speed * dt;
                        game.enemy_x += e_delta.x;
                        game.enemy_y += e_delta.y;
                    }
                    
                    if e_center.distance(target) < 5.0 {
                        game.path.remove(0);
                    }
                }

                if rect_intersects(game.player_x, game.player_y, 30.0, 30.0, game.enemy_x, game.enemy_y, 30.0, 30.0) {
                    data.last_score = (game.survival_timer * 100.0) as i32;
                    if data.last_score > data.high_score {
                        data.high_score = data.last_score;
                    }
                    state = State::Result;
                }

                clear_background(BLACK);
                
                for &(wx, wy) in &game.walls {
                    // 【修正】DIMGRAY から dim_gray に変更
                    draw_rectangle(wx, wy, TILE_SIZE, TILE_SIZE, dim_gray);
                    draw_rectangle_lines(wx, wy, TILE_SIZE, TILE_SIZE, 1.0, GRAY);
                }

                let p_color = if is_dashing { ORANGE } else { SKYBLUE };
                draw_rectangle(game.player_x, game.player_y, 30.0, 30.0, p_color);

                let e_color = if game.was_spotted { RED } else { WHITE };
                draw_rectangle(game.enemy_x, game.enemy_y, 30.0, 30.0, e_color);
                if game.was_spotted {
                    draw_rectangle_lines(game.enemy_x - 2.0, game.enemy_y - 2.0, 34.0, 34.0, 2.0, RED);
                }

                let bar_w = 150.0 * game.stamina;
                let bar_color = if game.stamina < 0.2 { RED } else { ORANGE };
                draw_rectangle(20.0, screen_height() - 30.0, bar_w, 10.0, bar_color);
                draw_text(&format!("TIME: {:.1}s", game.survival_timer), 20.0, 40.0, 20.0, WHITE);
            }
            
            State::Result => {
                clear_background(Color::from_rgba(10, 10, 10, 255));
                draw_text("CAUGHT!", 300.0, 220.0, 50.0, RED);
                draw_text(&format!("Score: {}", data.last_score), 360.0, 290.0, 20.0, WHITE);

                let btn_x = screen_width() / 2.0 - 80.0;
                let btn_y = screen_height() / 2.0 + 40.0;
                let btn_w = 160.0;
                let btn_h = 40.0;

                draw_rectangle(btn_x, btn_y, btn_w, btn_h, DARKGRAY);
                draw_text("Back to Title", btn_x + 22.0, btn_y + 26.0, 18.0, WHITE);

                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    if mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h {
                        state = State::Title;
                    }
                }
            }
        }

        next_frame().await
    }
}