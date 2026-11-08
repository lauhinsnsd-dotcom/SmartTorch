SmartTorch - 火炬之光：無限 輔助工具箱 / A helper toolbox for Torchlight: Infinite
================================================================================

完全免費。官方下載只有 https://github.com/lauhinsnsd-dotcom/SmartTorch/releases
如果你是付費取得的，你被騙了。
Free. The only official download is https://github.com/lauhinsnsd-dotcom/SmartTorch/releases
If you paid for it, you were scammed.

非官方工具：只讀取遊戲畫面與日誌，不會操作遊戲、不讀取記憶體。
Unofficial: it only reads the game's picture and log; it never plays for you or reads the game's memory.


【開始使用 / Getting started】

1. 點兩下 SmartTorch.exe。第一次開啟會出現教學。
   Run SmartTorch.exe. The first start walks you through the rest.
2. 遊戲中按 Esc →「帳號相關」→「開啟日誌」，然後登出再登入。
   In the game: Esc → Account Related → Enable Log, then log out and in again.
3. 遊戲使用無邊框或視窗模式（不要用獨佔全螢幕）。
   Play borderless or windowed (not exclusive full screen).
4. 在主視窗選擇實習生雪莉等級（決定手術用具可刷新幾次）。
   Set your Intern Shelley level in the main window (it sets the instrument rerolls).

第一次執行若出現「Windows 已保護您的電腦」，點「其他資訊」→「仍要執行」。
If Windows shows "Windows protected your PC", click More info → Run anyway.


【主視窗與右下角圖示 / The main window and the tray icon】

- 按視窗的 ✕ 只會縮到右下角，工具繼續執行。點右下角圖示可再開啟。
  The window's ✕ only puts it in the tray; click the tray icon to open it again.
- 完全關閉：右下角圖示按右鍵 →「結束」。
  To quit: right-click the tray icon → Quit.
- 左下角可切換 繁體／简体／EN。 Switch 繁體 / 简体 / EN at the bottom left.
- 設定 → 一般：開機自動啟動、開遊戲時自動啟動工具。
  Settings → General: start with Windows, start the tools with the game.


【渴癮症助手 / Vorax helper】

在「手術準備」畫面標示建議的卡牌與培養槽，以及達到 840k（彩箱）的機率。
On the Surgical Preparation screen it marks the card and tanks to pick and the odds of reaching 840k.

核對讀取結果 / Check what was read:
- 每格培養槽下方的小框，例如「骨 15×12」（活性×數量，顏色＝稀有度）。
  Under each tank, e.g. "Bone 15×12" (Activity × Quantity, coloured by rarity).
- 提示框下方一行顯示讀到的卡；刷新按鈕右邊顯示剩餘刷新次數。
  Under the hint, the cards read; beside the reroll button, the rerolls left.
- 讀錯的話，那一步的建議就不可靠。 If something is misread, that move's advice can't be trusted.

模式 / Modes:
- 原作者 / Upstream's：只追求達到 840k 的機率。 Only the odds of 840k count.
- 衝分 / Push score：機率差不多時選平均分數較高的。 When the odds are close, the higher average score.

中文客戶端已測試（遊戲「介面縮放」用推薦值或最小值）：1920×1080、2560×1440、2560×1600；3840×2160 建議用最小值。其他解析度可能讀不準。
A Chinese client is tested (the game's interface scale at its recommended value or the smallest): 1920×1080, 2560×1440, 2560×1600; at 3840×2160 the smallest is advised. Other resolutions may be misread.


【囚獸助手 / Tartarus helper】

glcoder 的 Tartarus Solver（https://github.com/glcoder/TartarusSolver），請自行下載解壓，
開啟一次 TartarusSolver.exe，SmartTorch 會自動找到它（或在主視窗「選擇位置」）。
之後 SmartTorch 會啟動它、收起它的視窗；按「設定」打開它的視窗。
glcoder's Tartarus Solver: download and unpack it, run TartarusSolver.exe once and SmartTorch finds it
(or Choose its place in the main window). SmartTorch then starts it and keeps its window out of the way;
Settings opens its window.


【黑帆備忘 / Blacksail Notes】

虛海封印秘語：在主視窗「黑帆備忘」按「啟動」，遊戲上會出現小視窗。每次選擇秘語時讀取卡片，
列出本局已選的秘語（最多 6 條），離開虛海終港後自動清空。拖曳可移動；拖曳右下角可調整大小；
「透明」讓滑鼠穿透清單；「×」關閉。1920×1080 請用推薦值介面縮放；2560×1440、2560×1600、
3840×2160 推薦值或最小值皆可。
Blacksail seals: press Start on the Blacksail Notes page and a small window shows over the game. Each
pick is read off the cards and listed (up to 6), emptied once you leave the port. Drag it to move it,
its lower right corner to resize it; 透明 lets clicks pass through the list; × closes it. At 1920×1080 use
the recommended interface scale; 2560×1440, 2560×1600, 3840×2160: recommended or smallest.


【易火 / ETor】

其他作者的收益統計工具「易火 ETor」的啟動器。請自行安裝，開啟一次後 SmartTorch 會自動記住位置
（或在主視窗「易火」按「選擇…」），之後可在 SmartTorch 開啟它；在「設定」可設定開遊戲時自動開啟。
易火需要系統管理員權限，每次開啟 Windows 都會詢問，請按「是」。SmartTorch 不會修改或讀取它的資料。
A launcher for ETor, another author's income tracker. Install it and open it once, and SmartTorch
remembers it (or Choose… on the ETor page); then open it from SmartTorch, or with the game (Settings).
It needs administrator rights, so Windows asks each time: press Yes. SmartTorch never touches its data.


【自訂啟動 / Custom launchers】

自己選兩個程式（.exe），可改名，在 SmartTorch 一鍵開啟；在「設定」可設定開遊戲時自動開啟（只開啟、不會關閉）。
Two programs of your choice (renamed as you like): open them from SmartTorch, or with the game (Settings;
opened only, never closed).


【流派碼 / Build codes】

儲存自己常用的流派碼，按「複製」後到遊戲裡貼上。 Keep the build codes you use; Copy, then paste in the game.


【快捷鍵 / Hotkeys】

Ctrl + Alt + X   立即讀取 / Read the board now
Ctrl + Alt + V   顯示／隱藏提示 / Show or hide the hints
Ctrl + Alt + D   顯示／隱藏讀取結果 / Show or hide what was read
Ctrl + Alt + B   記下讀錯的畫面 / Set a misread picture aside


【讀錯時 / When something is misread】

1. 按 Ctrl + Alt + X 重新讀取。把滑鼠移開卡牌，等卡牌飛到定位。
   Press Ctrl + Alt + X to read again. Move the mouse off the cards; wait for them to land.
2. 仍然不對：按 Ctrl + Alt + B 記下這個畫面。
   Still wrong: Ctrl + Alt + B sets the picture aside.
3. 之後在主視窗按「回報問題」：桌面出現 SmartTorch-report-日期.zip，並開啟回報頁，
   把 zip 拖進頁面即可（需要 GitHub 帳號）。
   Later press Report a problem in the main window: SmartTorch-report-<time>.zip appears on the
   desktop and the report page opens; drag the zip into it (a GitHub account is needed).

主視窗「設定 → 疑難排解」有更多說明。 More in the main window: Settings → Troubleshooting.


【授權 / Licence】

MIT（見 LICENSE.txt）。渴癮症助手基於 DinoRaos 的 vorax-solver-rs（MIT）、tidewarden 與 Moebing 的成果；
字型 Noto Sans TC／SC（SIL OFL 1.1，見 FONT-LICENSE.txt）。由肥可製作。
MIT (LICENSE.txt). The Vorax helper builds on DinoRaos's vorax-solver-rs (MIT) and the work of tidewarden
and Moebing; fonts Noto Sans TC/SC (SIL OFL 1.1, FONT-LICENSE.txt). Made by 肥可.
