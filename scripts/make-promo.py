# -*- coding: utf-8 -*-
"""HttpTool 宣传片（1920×1080 / 30fps / 23s）——「信号」方向。

与系列其它三支（剪贴板/置顶/密码管理器）共用一套语言：石墨黑底、纸白卡片、
中英分段排版（中文雅黑 / ASCII Consolas / 拉丁 Bahnschrift）、编号轴 + 顶部进度、
粒噪、碳复写残影、片尾唯一一枚朱红印章。

但这支的签名动作不同：**竖直扫描** —— 卡片由一条自上而下的亮边"刷"出来，
而不是剪贴板那支的横向扫描条；因为这里的主动作是"发出去、收回来"，
竖直方向对应请求发出到响应返回的时间轴。

素材全部是真实前端截图（docs/promo/shots/），数据来自真实 HTTP 请求。
"""
import os
import subprocess

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H, FPS, DUR = 1920, 1080, 30, 23.0
NF = int(FPS * DUR)
ROOT = os.path.dirname(os.path.abspath(__file__))
SHOTS = os.path.join(ROOT, "..", "docs", "promo", "shots")
OUT = os.path.join(ROOT, "..", "docs", "promo", "HttpTool-promo.mp4")

INK = (14, 15, 16)
PAPER = (245, 244, 240)
SILVER = (168, 173, 181)
GRAPHITE = (58, 61, 66)
STAMP = (196, 46, 38)

F_CJK, F_CJK_BD = r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\msyhbd.ttc"
F_MONO, F_MONO_BD = r"C:\Windows\Fonts\consola.ttf", r"C:\Windows\Fonts\consolab.ttf"
F_LATIN = r"C:\Windows\Fonts\bahnschrift.ttf"
_fc = {}


def _load(p, size, variation=None):
    f = ImageFont.truetype(p, size)
    if variation:
        try:
            f.set_variation_by_name(variation)
        except Exception:
            pass
    return f


def cjk(size, bold=True):
    k = ("c", size, bold)
    if k not in _fc:
        _fc[k] = _load(F_CJK_BD if bold else F_CJK, size)
    return _fc[k]


def mono(size, bold=False):
    k = ("m", size, bold)
    if k not in _fc:
        _fc[k] = _load(F_MONO_BD if bold else F_MONO, size)
    return _fc[k]


def latin(size):
    k = ("l", size)
    if k not in _fc:
        try:
            _fc[k] = _load(F_LATIN, size, "SemiBold Condensed")
        except Exception:
            _fc[k] = _load(F_MONO_BD, size)
    return _fc[k]


def ease(p):
    p = max(0.0, min(1.0, p))
    return p * p * (3 - 2 * p)


def ease_out(p):
    p = max(0.0, min(1.0, p))
    return 1 - (1 - p) ** 3


def _is_cjk(ch):
    return ord(ch) > 0x2E80


def mixed(d, xy, s, sz_cjk, sz_mono, color=PAPER, gap=0):
    x, y = xy
    runs = []
    for ch in s:
        k = "cjk" if _is_cjk(ch) else "mono"
        if runs and runs[-1][0] == k:
            runs[-1][1] += ch
        else:
            runs.append([k, ch])
    for kind, text in runs:
        f = cjk(sz_cjk, True) if kind == "cjk" else mono(sz_mono, True)
        d.text((x, y), text, font=f, fill=color, anchor="ls")
        x += d.textlength(text, font=f) + gap


def build_bg():
    img = Image.new("RGB", (W, H), INK)
    d = ImageDraw.Draw(img)
    for x in range(0, W, 160):
        d.line([(x, 0), (x, H)], fill=(25, 26, 28))
    for y in range(0, H, 160):
        d.line([(0, y), (W, y)], fill=(23, 24, 26))
    vig = Image.new("L", (W, H), 0)
    ImageDraw.Draw(vig).ellipse([-W // 3, -H // 4, W + W // 3, H + H // 4], fill=44)
    vig = vig.filter(ImageFilter.GaussianBlur(200))
    img = Image.composite(img, Image.new("RGB", (W, H), (0, 0, 0)),
                          vig.point(lambda v: 255 - v // 3))
    return img.convert("RGBA")


BG = build_bg()
_rng = np.random.default_rng(23)
GRAIN = Image.fromarray(np.clip(128 + _rng.normal(0, 7, (H, W, 1)).repeat(3, axis=2), 0, 255)
                        .astype("uint8"), "RGB")
_cards = {}


# 截图是整窗（含左栏），这里裁掉左侧边栏，只留请求/响应面板
CROPS = {
    "response.png": (760, 90, 2880, 1790),
    "headers.png": (760, 90, 2880, 1790),
    "save.png": (520, 90, 2880, 1790),
}


def card(name, scale, radius=20):
    key = (name, scale)
    if key in _cards:
        return _cards[key]
    src = Image.open(os.path.join(SHOTS, name)).convert("RGBA")
    if name in CROPS:
        src = src.crop(CROPS[name])
    w, h = int(src.width * scale), int(src.height * scale)
    body = src.resize((w, h), Image.LANCZOS)
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], radius=radius, fill=255)
    body.putalpha(mask)
    pad = 70
    out = Image.new("RGBA", (w + pad * 2, h + pad * 2), (0, 0, 0, 0))
    sh = Image.new("L", out.size, 0)
    ImageDraw.Draw(sh).rounded_rectangle([pad - 4, pad + 12, pad + w + 4, pad + h + 22],
                                         radius=radius, fill=145)
    out.paste(Image.new("RGBA", out.size, (0, 0, 0, 255)), (0, 0),
              sh.filter(ImageFilter.GaussianBlur(26)))
    g = body.copy()
    g.putalpha(g.getchannel("A").point(lambda v: v * 28 // 255))
    out.alpha_composite(g, (pad + 12, pad + 14))
    out.alpha_composite(body, (pad, pad))
    _cards[key] = out
    return out


def put_card(img, c, cx, cy, alpha=255, reveal=1.0):
    """reveal<1：由一条自上而下的亮边刷出卡片（本片的签名动作）。"""
    if alpha <= 0 or reveal <= 0:
        return
    c = c.copy()
    if reveal < 1.0:
        cut = int(c.height * reveal)
        c = c.crop((0, 0, c.width, cut))
    if alpha < 255:
        c.putalpha(c.getchannel("A").point(lambda v: v * alpha // 255))
    x = int(cx - c.width / 2)
    y = int(cy - c.height / 2)
    img.alpha_composite(c, (x, y))
    if reveal < 1.0:
        # 竖直亮边 + 上方余辉
        hh = 90
        band = Image.new("RGBA", (c.width, hh), (0, 0, 0, 0))
        bd = ImageDraw.Draw(band)
        for i in range(hh):
            t = i / (hh - 1)
            bd.line([(0, i), (c.width, i)], fill=(255, 255, 255, int(110 * (1 - t) ** 2)))
        img.alpha_composite(band, (x, y + c.height - hh))
        ImageDraw.Draw(img).line([(x, y + c.height), (x + c.width, y + c.height)],
                                 fill=(255, 255, 255, 220), width=2)


def chrome(img, idx, total, t):
    d = ImageDraw.Draw(img)
    d.line([(150, 92), (W - 150, 92)], fill=(44, 46, 50), width=2)
    d.line([(150, 92), (150 + int((W - 300) * (t / DUR)), 92)], fill=PAPER, width=2)
    d.text((W - 150, 66), f"{idx:02d}/{total:02d}", font=mono(20, True), fill=GRAPHITE, anchor="rs")
    mixed(d, (150, H - 62), "HTTPTOOL / TAURI V2 + RUST / 便携", 17, 19, (74, 78, 84))


CX, RAIL_Y = 150, 200


def eyebrow(d, y, s):
    mixed(d, (CX, y), s, 24, 22, SILVER, gap=6)


def head(d, y, s, size=86):
    d.text((CX, y), s, font=cjk(size), fill=PAPER, anchor="ls")


def sub(d, y, s, color=SILVER, size=30):
    mixed(d, (CX, y), s, size - 1, size, color)


TOTAL = 7


def s_open(t, dur):
    d = ImageDraw.Draw(layer)
    d.text((CX, 380), "HTTP 请求工具", font=cjk(84), fill=PAPER, anchor="ls")
    if t > 0.3:
        q = ease((t - 0.3) / 0.6)
        d.text((CX, 500), "HttpTool", font=latin(62),
               fill=tuple(int(SILVER[i] * q) for i in range(3)), anchor="ls")
    if t > 0.6:
        q = ease((t - 0.6) / 0.7)
        mixed(d, (CX, 600), "一个 exe，双击即用。", 38, 38,
              tuple(int(PAPER[i] * q) for i in range(3)))
        mixed(d, (CX, 664), "零安装 · 零运行时依赖 · 数据全在本地", 26, 27,
              tuple(int(SILVER[i] * q) for i in range(3)))
    pr = min(1.0, ease_out((t - 0.25) / 0.9) + 0.02)
    put_card(img, card("response.png", 0.40), 1240, 600, 255, pr)


def s_send(t, dur):
    d = ImageDraw.Draw(layer)
    eyebrow(d, RAIL_Y, "01 / 发送")
    head(d, RAIL_Y + 92, "敲 URL，回车", 64)
    sub(d, RAIL_Y + 168, "Ctrl+Enter 发送 · Ctrl+S 存集合", SILVER, 26)
    if t > 0.7:
        q = ease((t - 0.7) / 0.6)
        mixed(d, (CX, RAIL_Y + 236), "200 OK · 497 ms · 83 B", 24, 26,
              tuple(int(SILVER[i] * q) for i in range(3)))
    pr = min(1.0, ease_out((t - 0.12) / 0.85) + 0.02)
    put_card(img, card("response.png", 0.42), 1240, 640, 255, pr)


def s_read(t, dur):
    d = ImageDraw.Draw(layer)
    eyebrow(d, RAIL_Y, "02 / 读")
    head(d, RAIL_Y + 92, "响应看得清", 64)
    sub(d, RAIL_Y + 168, "JSON 折叠 · 搜索 · 存文件", SILVER, 26)
    if t > 0.8:
        q = ease((t - 0.8) / 0.6)
        mixed(d, (CX, RAIL_Y + 236), "状态码 / 耗时 / 体积 一眼可见", 24, 26,
              tuple(int(SILVER[i] * q) for i in range(3)))
    pr = min(1.0, ease_out((t - 0.15) / 0.85) + 0.02)
    put_card(img, card("headers.png", 0.42), 1240, 640, 255, pr)


def s_save(t, dur):
    d = ImageDraw.Draw(layer)
    eyebrow(d, RAIL_Y, "03 / 存")
    head(d, RAIL_Y + 92, "存成集合", 66)
    sub(d, RAIL_Y + 168, "集合 · 已保存 · 历史 · 环境", SILVER, 26)
    if t > 0.9:
        q = ease((t - 0.9) / 0.6)
        mixed(d, (CX, RAIL_Y + 236), "{{var}} 引用环境变量，发送前会检查未定义", 24, 26,
              tuple(int(SILVER[i] * q) for i in range(3)))
    pr = min(1.0, ease_out((t - 0.12) / 0.85) + 0.02)
    put_card(img, card("save.png", 0.42), 1240, 640, 255, pr)


def s_curl(t, dur):
    d = ImageDraw.Draw(layer)
    eyebrow(d, RAIL_Y, "04 / 互通")
    head(d, RAIL_Y + 92, "cURL 进出", 66)
    sub(d, RAIL_Y + 168, "DevTools 的 cURL 直接贴进来", SILVER, 26)
    if t > 0.8:
        q = ease((t - 0.8) / 0.6)
        mixed(d, (CX, RAIL_Y + 236), "当前请求也能一键导出成 cURL", 24, 26,
              tuple(int(SILVER[i] * q) for i in range(3)))
    # 用文字块模拟一条 cURL，与真实界面同色系
    if t > 0.5:
        q = ease((t - 0.5) / 0.7)
        box = Image.new("RGBA", (720, 168), (0, 0, 0, 0))
        bd = ImageDraw.Draw(box)
        bd.rounded_rectangle([0, 0, 719, 167], radius=8,
                             fill=(PAPER[0], PAPER[1], PAPER[2], int(246 * q)))
        bd.text((26, 26), "curl 'https://api.example.com/users' \\", font=mono(20), fill=INK)
        bd.text((26, 62), "  -H 'Authorization: Bearer {{token}}' \\", font=mono(20), fill=INK)
        bd.text((26, 98), "  -H 'Content-Type: application/json' \\", font=mono(20), fill=INK)
        bd.text((26, 134), "  --compressed", font=mono(20), fill=(120, 124, 130))
        if q < 1:
            box.putalpha(box.getchannel("A").point(lambda v: v * int(q) // 255))
        layer.alpha_composite(box, (1120, 700))


def s_facts(t, dur):
    d = ImageDraw.Draw(layer)
    eyebrow(d, 260, "05 / 事实")
    rows = [("可执行文件", "1 个 exe"), ("运行时依赖", "仅系统 WebView2"),
            ("数据", "http-tool.db 单文件"), ("许可", "MIT")]
    for i, (k, v) in enumerate(rows):
        q = ease((t - 0.2 - i * 0.3) / 0.55)
        if q <= 0:
            continue
        y = 420 + i * 110
        mixed(d, (CX, y), k, 32, 32, tuple(int(SILVER[c] * q) for c in range(3)))
        mixed(d, (700, y), v, 36, 38, tuple(int(PAPER[c] * q) for c in range(3)))
        d.line([(660, y + 18), (W - CX, y + 18)], fill=(255, 255, 255, int(16 * q)), width=1)
    if t > 1.6:
        q = ease((t - 1.6) / 0.6)
        mixed(d, (CX, 900), "不做协作、不做云同步 —— 本地、轻、随手可用。", 30, 30,
              tuple(int(PAPER[c] * q) for c in range(3)))


def s_end(t, dur):
    d = ImageDraw.Draw(layer)
    d.text((CX, 470), "HttpTool", font=latin(118), fill=PAPER, anchor="ls")
    d.text((CX + 8, 546), "HTTP 请求调试工具", font=cjk(40, False), fill=SILVER, anchor="ls")
    if t > 0.45:
        q = ease((t - 0.45) / 0.6)
        d.text((CX, 660), "github.com/hunterlihao/http-request-tool", font=mono(32),
               fill=tuple(int(PAPER[c] * q) for c in range(3)), anchor="ls")
        mixed(d, (CX, 720), "MIT License   ·   便携   ·   无需安装", 24, 26,
              tuple(int(SILVER[c] * q) for c in range(3)))
    if t > 0.8:
        q = ease_out((t - 0.8) / 0.45)
        ss = 1.0 + 0.14 * (1 - q)
        box = Image.new("RGBA", (250, 250), (0, 0, 0, 0))
        bd = ImageDraw.Draw(box)
        bd.rectangle([8, 8, 241, 241], outline=(STAMP[0], STAMP[1], STAMP[2], int(235 * q)), width=7)
        bd.text((125, 100), "MIT", font=latin(74), fill=(STAMP[0], STAMP[1], STAMP[2], int(235 * q)), anchor="mm")
        bd.text((125, 180), "2026", font=mono(34, True), fill=(STAMP[0], STAMP[1], STAMP[2], int(205 * q)), anchor="mm")
        box = box.rotate(-7, expand=True, resample=Image.BICUBIC)
        box = box.resize((int(box.width * ss), int(box.height * ss)), Image.LANCZOS)
        layer.alpha_composite(box, (1390, 380))


SCENES = [
    (0.0, 2.8, s_open),
    (2.8, 6.8, s_send),
    (6.8, 10.8, s_read),
    (10.8, 14.8, s_save),
    (14.8, 18.4, s_curl),
    (18.4, 20.8, s_facts),
    (20.8, 23.0, s_end),
]
FADE = 0.36
layer = None
img = None


def render():
    global layer, img
    cmd = ["ffmpeg", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}",
           "-r", str(FPS), "-i", "-", "-an", "-c:v", "libx264", "-preset", "slow",
           "-crf", "19", "-pix_fmt", "yuv420p", "-movflags", "+faststart", OUT]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    for fi in range(NF):
        t = fi / FPS
        img = BG.copy()
        for idx, (s0, s1, fn) in enumerate(SCENES):
            if not (s0 <= t < s1):
                continue
            layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            fn(t - s0, s1 - s0)
            a = 255
            lt = t - s0
            if lt < FADE:
                a = int(255 * (lt / FADE))
            if s1 - t < FADE:
                a = min(a, int(255 * ((s1 - t) / FADE)))
            if a < 255:
                layer.putalpha(layer.getchannel("A").point(lambda v: v * a // 255))
            img.alpha_composite(layer)
            if idx > 0:
                chrome(img, idx + 1, TOTAL, t)
        img = Image.blend(img.convert("RGB"), GRAIN, 0.09)
        proc.stdin.write(img.tobytes())
        if fi % 90 == 0:
            print(f"  frame {fi}/{NF}", flush=True)
    proc.stdin.close()
    proc.wait()
    print("完成:", OUT, os.path.getsize(OUT) // 1024, "KB")


if __name__ == "__main__":
    render()
