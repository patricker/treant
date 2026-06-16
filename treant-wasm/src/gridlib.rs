//! Shared helpers for the "grid pack" line/shape games.
//! Board is row-major `Vec<i8>`: `-1` = empty, `>= 0` = symbol/player id.

pub const MAX_DIM: usize = 12;

#[inline]
pub fn idx(r: usize, c: usize, cols: usize) -> usize {
    r * cols + c
}

/// Longest run of `sym` through cell `(r, c)` across all 4 line directions
/// (the cell itself counts as 1).
pub fn max_run(board: &[i8], cols: usize, rows: usize, r: usize, c: usize, sym: i8) -> usize {
    if board[idx(r, c, cols)] != sym {
        return 0;
    }
    let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
    let mut best = 1usize;
    for (dr, dc) in dirs {
        let mut count = 1usize;
        for sign in [1i32, -1i32] {
            let mut k = 1i32;
            loop {
                let nr = r as i32 + dr * sign * k;
                let nc = c as i32 + dc * sign * k;
                if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                    break;
                }
                if board[idx(nr as usize, nc as usize, cols)] == sym {
                    count += 1;
                    k += 1;
                } else {
                    break;
                }
            }
        }
        if count > best {
            best = count;
        }
    }
    best
}

/// The symbol of the first cell whose run reaches `len` (a "line of len"), if any.
pub fn line_symbol(board: &[i8], cols: usize, rows: usize, len: usize) -> Option<i8> {
    for r in 0..rows {
        for c in 0..cols {
            let s = board[idx(r, c, cols)];
            if s >= 0 && max_run(board, cols, rows, r, c, s) >= len {
                return Some(s);
            }
        }
    }
    None
}

/// Does `sym` have a run of at least `len` anywhere?
pub fn has_line(board: &[i8], cols: usize, rows: usize, sym: i8, len: usize) -> bool {
    for r in 0..rows {
        for c in 0..cols {
            if board[idx(r, c, cols)] == sym && max_run(board, cols, rows, r, c, sym) >= len {
                return true;
            }
        }
    }
    false
}

pub fn is_full(board: &[i8]) -> bool {
    board.iter().all(|&c| c >= 0)
}

/// Tiny center-favoring material score for `me` (keeps solver-carried games from
/// playing randomly in the opening without overriding tactical proven values).
pub fn center_eval(board: &[i8], cols: usize, rows: usize, me: i8) -> i64 {
    let (cr, cc) = ((rows as i64 - 1), (cols as i64 - 1));
    let mut s = 0i64;
    for r in 0..rows {
        for c in 0..cols {
            if board[idx(r, c, cols)] == me {
                let dr = (2 * r as i64 - cr).abs();
                let dc = (2 * c as i64 - cc).abs();
                s += (cr + cc) - (dr + dc);
            }
        }
    }
    s
}

/// Heuristic: sum of squared occupancy over `win_len`-windows that contain only
/// `me` (and empties). Higher = closer to completing a line for `me`.
pub fn window_eval(board: &[i8], cols: usize, rows: usize, me: i8, win_len: usize) -> i64 {
    let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
    let mut score = 0i64;
    for r in 0..rows {
        for c in 0..cols {
            for (dr, dc) in dirs {
                let er = r as i32 + dr * (win_len as i32 - 1);
                let ec = c as i32 + dc * (win_len as i32 - 1);
                if er < 0 || er >= rows as i32 || ec < 0 || ec >= cols as i32 {
                    continue;
                }
                let mut mine = 0i64;
                let mut blocked = false;
                for s in 0..win_len as i32 {
                    let v = board[idx((r as i32 + dr * s) as usize, (c as i32 + dc * s) as usize, cols)];
                    if v == me {
                        mine += 1;
                    } else if v >= 0 {
                        blocked = true;
                        break;
                    }
                }
                if !blocked && mine > 0 {
                    score += mine * mine;
                }
            }
        }
    }
    score
}

/// Order's "threat": squared progress of the best single symbol in each
/// `win_len`-window that contains exactly one symbol type. Higher = closer to a
/// line of *any* symbol (good for Order, bad for Chaos).
pub fn line_threat(board: &[i8], cols: usize, rows: usize, win_len: usize) -> i64 {
    let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
    let mut score = 0i64;
    for r in 0..rows {
        for c in 0..cols {
            for (dr, dc) in dirs {
                let er = r as i32 + dr * (win_len as i32 - 1);
                let ec = c as i32 + dc * (win_len as i32 - 1);
                if er < 0 || er >= rows as i32 || ec < 0 || ec >= cols as i32 {
                    continue;
                }
                let mut sym = -1i8;
                let mut count = 0i64;
                let mut mixed = false;
                for s in 0..win_len as i32 {
                    let v = board[idx((r as i32 + dr * s) as usize, (c as i32 + dc * s) as usize, cols)];
                    if v < 0 {
                        continue;
                    }
                    if sym < 0 {
                        sym = v;
                        count = 1;
                    } else if v == sym {
                        count += 1;
                    } else {
                        mixed = true;
                        break;
                    }
                }
                if !mixed && count > 0 {
                    score += count * count;
                }
            }
        }
    }
    score
}

/// Do any 4 cells of `sym` form the corners of a square (any size/orientation)?
pub fn has_square(board: &[i8], cols: usize, rows: usize, sym: i8) -> bool {
    let pts: Vec<(i32, i32)> = (0..rows as i32)
        .flat_map(|r| (0..cols as i32).map(move |c| (r, c)))
        .filter(|&(r, c)| board[idx(r as usize, c as usize, cols)] == sym)
        .collect();
    let occ = |r: i32, c: i32| -> bool {
        r >= 0
            && r < rows as i32
            && c >= 0
            && c < cols as i32
            && board[idx(r as usize, c as usize, cols)] == sym
    };
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            let (r1, c1) = pts[i];
            let (r2, c2) = pts[j];
            let (dr, dc) = (r2 - r1, c2 - c1);
            // treat (p1,p2) as an edge; perpendicular = (-dc, dr); check both sides
            for (pr, pc) in [(-dc, dr), (dc, -dr)] {
                if occ(r1 + pr, c1 + pc) && occ(r2 + pr, c2 + pc) {
                    return true;
                }
            }
        }
    }
    false
}
