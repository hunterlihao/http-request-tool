//! 应用图标：启动时按矢量描述绘制，避免引入图片解码依赖

use std::sync::Arc;

const SIZE: usize = 64;
const SS: usize = 3; // 超采样倍数（抗锯齿）

/// 圆角方块范围（uv 坐标）
const RECT: (f32, f32, f32, f32) = (0.02, 0.02, 0.98, 0.98);
const RADIUS: f32 = 0.25;

/// 闪电多边形（uv 坐标）
const BOLT: [(f32, f32); 6] = [
    (0.585, 0.130),
    (0.280, 0.560),
    (0.465, 0.560),
    (0.405, 0.875),
    (0.720, 0.435),
    (0.530, 0.435),
];

/// 生成窗口 / 任务栏图标
pub fn window_icon() -> Arc<egui::IconData> {
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let n = (SS * SS) as f32;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let u = (x as f32 + (sx as f32 + 0.5) / SS as f32) / SIZE as f32;
                    let v = (y as f32 + (sy as f32 + 0.5) / SS as f32) / SIZE as f32;
                    if let Some((cr, cg, cb)) = sample(u, v) {
                        r += cr;
                        g += cg;
                        b += cb;
                        a += 1.0;
                    }
                }
            }
            let i = (y * SIZE + x) * 4;
            if a > 0.0 {
                rgba[i] = (r / a).round() as u8;
                rgba[i + 1] = (g / a).round() as u8;
                rgba[i + 2] = (b / a).round() as u8;
            }
            rgba[i + 3] = (a / n * 255.0).round() as u8;
        }
    }

    Arc::new(egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    })
}

/// 单点采样：返回 Some(rgb) 表示该点被图标覆盖
fn sample(u: f32, v: f32) -> Option<(f32, f32, f32)> {
    if !inside_round_rect(u, v) {
        return None;
    }
    if inside_bolt(u, v) {
        return Some((255.0, 255.0, 255.0));
    }
    // 竖向渐变：#FB923C -> #C23708
    let t = v.clamp(0.0, 1.0);
    Some((
        251.0 + (194.0 - 251.0) * t,
        146.0 + (55.0 - 146.0) * t,
        60.0 + (8.0 - 60.0) * t,
    ))
}

fn inside_round_rect(u: f32, v: f32) -> bool {
    let (x0, y0, x1, y1) = RECT;
    if u < x0 || u > x1 || v < y0 || v > y1 {
        return false;
    }
    let cx = u.clamp(x0 + RADIUS, x1 - RADIUS);
    let cy = v.clamp(y0 + RADIUS, y1 - RADIUS);
    let dx = u - cx;
    let dy = v - cy;
    dx * dx + dy * dy <= RADIUS * RADIUS
}

/// 射线法判断点是否在闪电多边形内
fn inside_bolt(u: f32, v: f32) -> bool {
    let mut inside = false;
    let mut j = BOLT.len() - 1;
    for i in 0..BOLT.len() {
        let (xi, yi) = BOLT[i];
        let (xj, yj) = BOLT[j];
        if (yi > v) != (yj > v) {
            let x = (xj - xi) * (v - yi) / (yj - yi) + xi;
            if u < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
