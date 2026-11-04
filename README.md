# Vorax 輔助器 中文版（火炬之光：無限「渴癮症」）

> **非官方玩家自製工具。** 本專案以 DinoRaos 的 [vorax-solver-rs](https://github.com/DinoRaos/vorax-solver-rs) v0.2.2（MIT 授權）為基礎，加上中文客戶端支援與「衝分」模式。與原作者、遊戲開發商及營運商無關，問題請不要找原作者。是否使用第三方工具、是否違反遊戲條款，請自行判斷，風險自負。

在「手術準備」（渴癮症）畫面，工具會標示建議選擇的卡牌、培養槽，以及這一步達到 840k（彩箱）的機率。**它只讀取遊戲畫面和遊戲日誌，不會替你點擊或操作遊戲，每一步都由你自己決定。**

## 最新版本：v0.2.2-zh.3

- **不必再設定遊戲路徑**：工具會從開啟中的遊戲找到安裝位置和遊戲日誌（Steam、官方啟動器、任何磁碟皆可）。`start-zh.bat` 不用再改。
- **衝分模式**正式推出，不再標示「測試中」。
- 新增「[讀不到、讀錯時怎麼辦](#讀不到讀錯時怎麼辦)」的一步一步說明。

上一版 v0.2.2-zh.2：正式支援簡體中文客戶端、修正 3840×2160 的多項讀取問題、改為自動更新本專案的中文版。各版本完整內容見 [CHANGELOG-zh.md](CHANGELOG-zh.md)。

## 下載與使用

1. 到本專案的 [Releases](https://github.com/lauhinsnsd-dotcom/vorax-solver-rs-zh/releases/latest) 下載壓縮檔並解壓縮（內含 `vorax-overlay.exe`、`start-zh.bat`、`start-zh-debug.bat` 與說明）。
2. 遊戲中按 Esc →「帳號相關」→「開啟日誌」，然後登出再登入。
3. 點兩下 `start-zh.bat`，工具會在右下角系統匣執行（Windows 11 可能收在「^」裡）。遊戲日誌會自動尋找，不必設定路徑。詳細說明見 `使用說明.txt`。

若系統匣顯示「找不到遊戲日誌」：先開啟遊戲，工具會從遊戲的安裝位置找到日誌；仍找不到時，用記事本打開 `start-zh.bat`，在第 5 行 `GAMELOG=` 後面填入日誌路徑（`遊戲資料夾\Game\UE_game\TorchLight\Saved\Logs\UE_game.log`）。

**更新**：從 v0.2.2-zh.2 起，有新版本時系統匣會提示，點「立即更新」即可（下載本專案的新版本並檢查 SHA-256，不會變回原作者的英文版）。v0.2.2-zh.1 的使用者請手動下載 v0.2.2-zh.2 一次；在 zh.1 看到「更新到 v…」的提示請不要點，那是原作者的英文版。

### 建議解析度

| 解析度 | 狀態 |
|---|---|
| 2560×1440 視窗（16:9） | 主要測試環境（繁體、簡體客戶端） |
| 3840×2160 視窗化全螢幕（16:9） | 已測試（繁體、簡體客戶端） |
| 其他解析度、21:9 | 未充分測試，讀取可能出錯 |

請使用視窗或「視窗化全螢幕」，**不要使用獨佔全螢幕**，否則工具無法擷取畫面。

## 兩種模式（系統匣「模式」切換）

- **保守（原作者）**：預設。只追求戰前達到 840k（彩箱）的機率，840k 以上的分數不計入，所以接近 840k 時常常不再加分。
- **衝分**：以「彩箱優先、再衝高分」為目標。只在幾個選項達到 840k 的機率相差 3% 以內時，才選平均分數較高的那個；模擬未來時使用另外訓練、不封頂的估分公式。提示框會多顯示「平均 N 萬」。

電腦模擬比較（同一批 2,000 局，兩種模式玩完全相同的發牌；工具每個選項模擬 64 次，遊戲中實際為 1,024 次，實戰應更準確）：

| | 保守 | 衝分 |
|---|---|---|
| **達 840k（彩箱）** | 69.8% | **74.2%** |
| 達 1.12M | 32.5% | 52.4% |
| 平均分數 | 103.6 萬 | 133.2 萬 |
| 中位數 | 97.3 萬 | 115.2 萬 |
| 最好的 10% 局 | 158.8 萬以上 | 236.3 萬以上 |
| 最差的 10% 局 | 53.9 萬以下 | 46.8 萬以下 |
| 低於 50 萬 | 8.3% | 11.4% |
| 低於 30 萬 | 1.7% | 3.8% |

衝分模式拿到彩箱的機率較高（逐局比較，差異由運氣造成的機率約千分之一），平均分數也高很多；**代價是翻車時分數可能較低**：低於 50 萬的局由約 8% 增加到約 11%。這些是模擬結果，使用的是原作者國際服的卡牌資料；實戰表現請以「對局紀錄」的實戰統計為準。

## 版本紀錄

每個版本更新了什麼，請看 [CHANGELOG-zh.md](CHANGELOG-zh.md)（系統匣的更新提示也會顯示）。

## 中文版更新內容

**讀取中文客戶端**
- **中文卡名**：用 Windows 內建的中文文字辨識讀卡名，再比對中文卡名表（85 張卡），簡繁視為同字，漏讀或錯一兩個字仍能辨認；兩張卡分不清時標為「未知」，不亂猜。原作者的版本只能讀英文卡名。
- **培養槽**：數字、種族圖示位置偏移時會在附近尋找；修正「x」被讀成數字、白色短名稱（例如「士兵」）被當成空格、中文名稱較高時金色或藍色名稱被誤認成圖示等問題。曾經因為漏看怪物而建議錯誤的手術用具，所以特別處理。
- **回合數**：加入中文介面的數字樣本。
- **原作者資料沒有圖片的卡**：抱卵蝶、異形螯針加入從中文客戶端取得的圖片，卡名讀不到時仍可辨認。
- **自動重讀**：有卡或培養槽沒讀到時，1 秒後自動重讀，最多 2 次（例如滑鼠擋住卡名）。

**讓玩家核對讀取結果**
- **讀取結果**：每格培養槽下方顯示讀到的種族與數字（顏色代表稀有度），提示框下方顯示讀到的卡。工具讀錯時建議就不可靠，所以要讓玩家一眼看出。Ctrl + Alt + D 切換顯示。
- **刷新次數**：刷新按鈕右邊顯示讀到的剩餘次數；沒讀到時顯示橘色「刷新 ？」。原作者的版本讀不到次數時會當作 0 次，而且不會提示，結果會在應該刷新時不建議刷新。
- **「未讀到」提示**：提示框列出沒讀到的卡和培養槽；原作者資料沒有的卡（例如「解剖浸液：病軀」）會提示「不在資料內，請自行判斷」。

**計算**
- **衝分模式**：見上一節。這是唯一修改計算部分的地方；使用保守模式時，結果與原作者完全相同。

**介面**
- 提示框、系統匣選單、說明卡、對局紀錄全部中文化，內建 Noto Sans TC／SC 字型。
- **對局紀錄**：可拖曳右邊、下邊或右下角改變大小，Ctrl + 滑鼠滾輪縮放，大小會記住。最上方的**實戰統計**按模式分開，列出打完的局數、彩箱局數、開局時工具預計的平均機率與平均分數；同一模式滿 20 局後，再依開局預計機率分組比較實際結果，可以檢查工具的預計和實戰是否相符。

## 讀不到、讀錯時怎麼辦

先一步一步試，仍然不行再回報：

1. **完全沒有提示**：看系統匣（Windows 11 可能收在「^」裡）有沒有工具的圖示。沒有就重新點 `start-zh.bat`；有的話按 Ctrl + Alt + V，確認提示沒有被隱藏。遊戲要在最前面，而且不能是「獨佔全螢幕」。
2. **系統匣顯示「找不到遊戲日誌」**：照系統匣的說明做（開啟遊戲、在遊戲中開啟日誌後重新登入）。在此之前也可以按 Ctrl + Alt + X 手動讀取。
3. **提示框寫「未讀到：第 N 張卡」或卡名顯示「？」**：
   - 把滑鼠移開卡牌（移到畫面空白處）。滑鼠指著卡時，卡會升起，標題可能被擋住。
   - 等卡牌完全飛到定位（動畫中工具會暫停提示）。
   - 按 Ctrl + Alt + X 重新讀取。
4. **培養槽的小框和遊戲不同（數字、種族、顏色），或刷新次數不對**：按 Ctrl + Alt + X 重新讀取；仍然錯的話，那一步的建議不可靠，請自行判斷，並回報。
5. **一局中途才開啟或重新開啟工具**：右上角已擁有的手術用具可能讀錯，建議每局開始前開啟。
6. **建議解析度**：2560×1440 視窗或 3840×2160 視窗化全螢幕。其他解析度讀取可能較不準確。

以上都試過仍然有問題：改用 `start-zh-debug.bat` 開啟，重現一次，再依下一節回報。

## 回報問題

到本專案的 [Issues](https://github.com/lauhinsnsd-dotcom/vorax-solver-rs-zh/issues/new/choose) 選「問題回報」，填寫解析度、繁簡客戶端、模式，並附上記錄檔：

- 記錄檔：在檔案總管的網址列輸入 `%APPDATA%\VoraxRs`，裡面的 `overlay.log`。
- 畫面：用 `start-zh-debug.bat` 開啟時，`frames` 資料夾裡檔名有 `reread` 或 `manual` 的圖片最有用，壓縮後一起附上即可，不必全部附上。

## 已知問題

- 工具在一局中途開啟（或重新開啟）時，沒有遊戲日誌可參考，右上角已擁有的手術用具要靠圖示辨認，可能把開局物品認成手術用具。建議在每局開始前開啟工具。
- 「解剖浸液：病軀」不在原作者的資料中，工具無法計算它。
- 最後 3 回合（手術方案）不給建議。
- 21:9 等其他比例尚未支援。

## 致謝與授權

- 原作者：[DinoRaos/vorax-solver-rs](https://github.com/DinoRaos/vorax-solver-rs)（MIT），遊戲引擎、計算與英文版讀取皆來自原作者。
- 中文卡名：Moebing 的國服模擬器（MIT）、OpenCC 簡轉繁，以及從繁中客戶端畫面讀取的名稱。
- 字型：Noto Sans TC、Noto Sans SC（SIL Open Font License 1.1）。
- 本專案同樣以 MIT 授權發布，見 [LICENSE](LICENSE)。

---

*以下為原作者的英文說明（未修改）。*

# Vorax Solver (Rust)

> **Unofficial fan project.** Not affiliated with or endorsed by XD Inc., the developer and publisher of
> Torchlight: Infinite. Game names and images belong to their owners. Use at your own risk.

A small Windows overlay for **Surgical Preparation**, the Vorax minigame of Torchlight: Infinite (SS11). It
follows your game, reads the board when it changes and marks the move the Solver advises: the card (or the
reroll) to take, the tanks to target, and how often that line reaches 840k.

![The overlay during a Vorax round: a cyan outline on the Anesthetic Tincture - Agito card, tank 2's glass glowing cyan, and the pill "Anesthetic Tincture - Agito → tank 2 · 33% reach 840k" at the top](docs/overlay.jpg)

Inspired by CoffeeBns's Tartarus Simulator and tidewarden's Vorax Solver (see [Credits](#credits)). Under the
overlay is a game engine, Solver and screen reader in Rust, and a Solver that plays 1,024 imagined games to the
end for every option (see [How far the Solver looks ahead](#how-far-the-solver-looks-ahead)).

**Read-only.** It reads the game window and the game's log file and draws on top. It never clicks or presses
keys, and it doesn't inject into or change the game: you make every move (see [AGENTS.md](AGENTS.md)). Changes
that add input automation won't be accepted.

## What it shows

- an outline on the card to take, or on the reroll button (cyan by default; five colours in the tray menu);
- a glowing outline on the glass of each tank to target;
- one small pill at the top with the move and how often it reaches 840k.

Nothing else is drawn, and nothing is drawn outside the Surgical Preparation screen. The marks only show while the
game is in front.

## Requirements

- **Windows 10 or 11.** The game borderless or windowed, not exclusive fullscreen.
- **The game's log turned on.** In the game: Esc → **Account Related** → **Enable Log**, then log out and back
  in. The overlay finds `UE_game.log` through Steam (or start it with `--log <path>`). The log only says *when*
  the board changes; everything else is read from the picture.

Card names are read with Windows' built-in text recognition: English when that language is installed, otherwise
your Windows language (tested with German). Without it, the pictures decide.

## Download

Get `vorax-overlay.exe` from the [latest release](https://github.com/DinoRaos/vorax-solver-rs/releases/latest) and run
it; there is nothing to install. Windows may say "Windows protected your PC" because the program isn't signed:
click **More info** → **Run anyway**. The release notes list each file's SHA-256 if you want to check your
download.

**Updates.** At start the overlay asks this project's GitHub releases once whether a newer version is out; nothing
about you or your games is sent. When there is one, the tray menu offers it with what's new, and **Update now**
downloads it, checks it against the SHA-256 GitHub gives for it, and restarts into it. It advises with the data
it was built with (see **Data** below); a new release brings any change.

## Build from source

You need [Rust](https://rustup.rs) (stable) and the Visual Studio Build Tools with "Desktop development with C++".

```powershell
git clone https://github.com/DinoRaos/vorax-solver-rs.git
cd vorax-solver-rs
cargo build --release                          # a few minutes the first time
.\target\release\vorax-overlay.exe
```

## Use

Start a Vorax game and the marks appear when the cards settle. Other ways to start it:

```powershell
.\target\release\vorax-overlay.exe --shelley 4   # your Intern Shelley level (default 6; remembered)
.\target\release\vorax-overlay.exe --selftest    # checks the log, the game window, a capture and the text reader
.\target\release\vorax-overlay.exe --demo        # a sample board's marks over the game for 15 s (switch to the game)
.\target\release\vorax-overlay.exe --preview shot.png out.png   # the marks drawn onto a saved screenshot
.\target\release\vorax-overlay.exe --frames D:\frames   # also saves every picture it reads (for reporting a misread)
```

| Key | |
|---|---|
| Ctrl+Alt+V | show or hide the overlay |
| Ctrl+Alt+X | read the board now |
| Ctrl+Alt+Q | quit |

<img src="docs/menu.png" alt="The tray menu: version and odds in use, game log status, Show overlay, Read the board now, Intern Shelley level 1-6, marker colour, Your runs, Quit" width="252" align="right">

The tray icon's menu has the same, plus your **Intern Shelley level** (1–6, remembered), the marks' colour, the
version and odds in use, and **Your runs**: every run the overlay read, newest first, with its instruments and how
the odds of reaching 840k went, opened board by board with a click (the advised move, the instruments, the
tanks). **Raw log** there opens the debug log itself (`%APPDATA%\VoraxRs\overlay.log`): attach it when reporting a
misread board.

When a card is still flying as the board is read, the overlay looks again a moment later (up to three times),
and hides its marks rather than leave ones for a board that's gone.

**Intern Shelley level.** It sets the instrument rerolls the advice counts on while they aren't on screen: a
potion board doesn't show them, so the overlay carries the count last read on an instrument offer, and before any
offer this game, the level's.

**Data.** The card odds (as of Oct 22, 2026), the board value and the screen reader's references are built in
(`data/`).

## How it works

1. **Log** (`crates/vorax-scan/src/live.rs`): follows `UE_game.log` every 250 ms; its lines say when a new board
   has settled.
2. **Picture**: Windows Graphics Capture of the game window only, started for that board and stopped after,
   frames grabbed until two in a row look alike; cut to the game's client area; no mouse, and on Windows 11 no
   yellow border.
3. **Reading** (`crates/vorax-scan`): tanks, cards, round, rerolls, instruments and the instrument tracker, with
   card titles read by Windows' OCR and the pictures deciding where a title doesn't settle. About 0.5 s per board.
4. **Advice** (`crates/vorax`): the Solver's look-ahead on every core, up to about a quarter second on the hardest
   boards (12 threads), most in well under 0.1 s.
5. **Overlay** (`crates/vorax-overlay`): a transparent, click-through, topmost window over the game's picture,
   shown only while the game is in front, redrawn only when the advice changes.

## How far the Solver looks ahead

For every option (each card on its best tanks, the next best plays, a reroll) the Solver plays imagined games to
the end: random next hands from the card odds, random outcomes, later choices judged by the board value (see
below). It counts how many reach 840k (the "% reach 840k" in the pill) and advises the option that did best. Each
imagined game is one "future"; the more of them, the less a close call between two options comes down to luck in
the Solver's own futures. The same 3,000 games, played with the Solver's full advice:

| Board value | Futures per option | Reached 840k | Hardest recorded board, 12 threads |
|---|---|---|---|
| Linear | 64 | 69.3% | 0.01 s |
| Trees | 64 | 71.8% | 0.06 s |
| Linear | 256 | 78.9% | 0.05 s |
| Trees | 256 (v0.2.0) | 79.2% | 0.28 s |
| Trees | 512 | 81.0% | 0.55 s |
| **Linear (built in)** | **1,024** | **81.5%** | **0.20 s** |

From 64 to 1,024 futures, 12 more games in a hundred reach 840k (a split that uneven by luck is less than a 0.1%
chance); against v0.2.0, 2.3 more (1.4%), with the advice ready sooner. On fewer cores the time grows in
proportion (8 threads: about 0.3 s on the hardest board), most boards being several times faster.

## Why Rust

An overlay that sits next to the game all evening asks for:

- **Speed.** The Solver's look-ahead plays out many futures for every option, and every future rates thousands of
  boards; on all cores even the hardest board's advice is ready in about a fifth of a second. The game state is a
  fixed-size value that copies in a single memcpy, and [rayon](https://github.com/rayon-rs/rayon) spreads the
  futures over every core.
- **Reading in about 0.5 s.**
- **One small program.** A ~5 MB exe with no runtime, browser or Electron, idle while the board doesn't change.
  It uses what Windows already has: Windows Graphics Capture for the game window alone, Windows OCR for card
  titles, and a layered window for the marks.
- **Training at scale.** The same engine plays thousands of simulated games a second, so the board value learns
  from millions of them.

## The board value

Inside the futures, the Solver judges where a board is headed by its board value: the final score (counted up to
840k) that games from a board like it tend to reach, given its monsters, instruments and round. The one built in
is linear, 656 weights per round, fitted on simulated games.

We also trained 120 decision trees (gradient boosting, fitted with [LightGBM](https://github.com/microsoft/LightGBM))
on 30 million boards from 1.5 million simulated games. With 64 futures per option they did better: 72.1% of 6,000
games reached 840k against 68.9% (luck: less than 0.1%), and v0.1.6 to v0.2.0 shipped them. With more futures the
Solver leans on the board value less, and at 256 the two are even (79.2% and 78.9% of 3,000 games, luck: 80%),
while the trees take five times as long to evaluate. So the time goes into futures: the linear board value at
1,024 futures beats the trees at 256 and matches them at 512 (table above).

```powershell
.\target\release\vorax train --games 2000000 --out linear.json    # a linear board value, fitted here
.\target\release\vorax train --games 1500000 --rows boards.bin    # boards as rows, for fitting trees
.\target\release\vorax sim --policy solver --futures 1024 --games 3000 --model a.json --scores a.txt
.\target\release\vorax gate built-in.txt a.txt                    # seed by seed: more games reaching 840k?
.\target\release\vorax sim --policy solver --futures 1024 --model b.json --against a.txt   # stops once it's clear
```

`--against` is a sequential test (as chess engines test their changes): the challenger plays the champion's games a
batch at a time and stops as soon as it's clearly better or not (5% error each way), so a clear result takes a
fraction of the games. One idea it ruled out: spending the futures by successive halving (dropping the worse half
of the options round by round) reached 840k in 80.1% of 1,440 games against 81.6% for every option alike.

## The `vorax` command

```powershell
.\target\release\vorax advise screen.json      # the Solver's advice for a screen (ScreenState JSON)
.\target\release\vorax advise --quick < s.json # this round's expected score only
.\target\release\vorax read shot.png --advise  # read a screenshot of the game (and advise on it)
.\target\release\vorax sim --games 1000 --policy value   # value | steer | solver (--futures N, as the overlay: 1024)
.\target\release\vorax bench fixtures          # advice timings on the 8 recorded screens
.\target\release\vorax info                    # the data in use
```

`--model <model.json|none>` on any command plays with another board value.

## Tests

`cargo test --release --workspace` replays recorded traces, so a change can't make the engine, Solver or reader
play differently by accident:

- 300 random games (normal and sandbox starts, every instrument), 11,627 actions: the full state after every
  action, including the RNG state, board value and build progress;
- 60 boards: options, expected values (with rollouts, steering and the board value), choices, shortlists,
  potion and instrument futures, and the instrument picker;
- 8 screens: the Solver's full advice (64 futures per option) and its quick advice;
- 8 whole games played by the bots;
- the screen reader on 6 test frames, each as it is, shrunk to 3/4 and recoloured (every field, score, problem
  and ranking, the card titles with three stand-in text readers, and the instrument tracker through a game), and
  the log settler on a recorded game log.

Everything matches exactly, floats included. The tests play with the linear board value the traces were recorded
with (`fixtures/boardValue-linear.json`, the one built in), and also check that a board's features give exactly
what it computes.

## Speed

Solver advice on the 8 recorded screens with 1,024 futures: up to 0.2 s a screen on 12 threads, up to about 1.4 s
on one core. Whole games: about 20,000 a second with the board-value bot; a game with the Solver's full advice at
every move (1,024 futures) takes about 3 s per game when 12 cores each play their own.

## Layout

| Path | What |
|---|---|
| `crates/vorax` | engine (`game.rs`, `context.rs`, `effects.rs`), board value (`value.rs`), bots (`bot.rs`), Solver (`advisor.rs`) |
| `crates/vorax-scan` | screen reader, instrument tracker and log settler (`scan.rs`, `titles.rs`, `track.rs`, `live.rs`) |
| `crates/vorax-overlay` | the overlay: `draw.rs` (the marks), `pipeline.rs`, and `app/` (capture, OCR, window, tray menu, your runs, updates) |
| `crates/vorax-cli` | the `vorax` command, and training (`train.rs`) |
| `data/` | cards, odds, the board value; `data/scan/` the reader's references |
| `fixtures/` | what the tests replay: recorded traces, the linear board value, test screenshots and a game log |

## Credits

Inspired by:

- **CoffeeBns's [Tartarus Simulator](https://github.com/blahdot3h/tartarus-simulator)**
  ([twitch.tv/coffeebns](https://twitch.tv/coffeebns)): following the game's log and reading the game window,
  the approach this overlay follows. No code from it is used here.
- **tidewarden's [Vorax Solver](https://github.com/tidewarden/tli-vorax-simulator)**: the game engine, Solver,
  screen reader and data here began as a Rust port of its last open version (7b4c9ba, Oct 22, 2026, MIT), itself
  based on [Moebing's Vorax simulator](https://github.com/Moebing/Torchlight-Infinite-SS11-Vorax-simulator)
  (MIT). Nothing from it after that version is used.

## License

[MIT](LICENSE): use, copy, change and share it freely, keeping the copyright and license notice. The notice keeps
tidewarden's and Moebing's copyright lines, as their MIT license requires for work built on theirs.
