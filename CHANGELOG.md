# 版本紀錄 · Changelog

每個版本更新了什麼。有新版本時，SmartTorch 的更新提示也會顯示該版本的內容。
What each version changed. When a new version is out, SmartTorch's update note shows its part.

## v1.0.3

**渴癮症助手 · Vorax helper**
- 修正：Windows 裝有英文時，中文遊戲的卡名會用英文文字辨識讀取而讀不到（只能靠卡面圖案，常出現「未讀到」）。現在卡名跟隨 SmartTorch 的語言讀取：繁體用繁體中文辨識、简体用簡體中文辨識、English 用英文辨識，不再受 Windows 語言順序影響。 Fixed: with English installed in Windows, a Chinese client's card names were read with English text recognition and missed (cards told by their pictures only, often "not read"). Card names are now read in SmartTorch's language: Traditional or Simplified Chinese, or English, whatever the order of Windows' languages.
- 請讓 SmartTorch 的語言和遊戲相同（英文遊戲請選 English）。Windows 沒有中文文字辨識時，「文字辨識」一行會顯示黃點。 Keep SmartTorch's language the same as the game's (English for an English client). Text reading shows a yellow dot when Windows has no Chinese text recognition.

## v1.0.2

**更新提示 · Updates**
- 有新版本時，主視窗左下角（語言切換下方）會顯示「立即更新」與「更新內容」，在任何一頁都看得到。 A new version shows at the bottom left of the main window (under the language switch), with Update now and What's new, on every page.
- SmartTorch 開著時每 6 小時自動檢查一次新版本（以前只在開啟時檢查）。 While it runs, SmartTorch looks for a new version every 6 hours (before, only at its start).

**版本紀錄 · Changelog**
- 沒有新版本時，左下角顯示目前版本，點一下即可在「設定 › 版本紀錄」查看每個版本更新了什麼。 With no new version, the bottom left shows this version: click it to see what each version changed, in Settings › Changelog.

## v1.0.1

**流派碼 · Build codes**
- 新分頁「流派碼」：儲存自己常用的流派碼（名字＋流派碼），按「複製」後到遊戲裡貼上；可修改、刪除、拖動排序。「清除紀錄」不會刪除流派碼。 A new Build codes page: keep the codes you use (a name and the code), Copy one and paste it in the game; edit, delete, drag to reorder. Clear records leaves them.

**易火 · ETor**
- 新分頁「易火」：其他作者的收益統計工具「易火 ETor」的啟動器。自行安裝後開啟一次，SmartTorch 會自動記住位置（或按「選擇…」），之後可在 SmartTorch 開啟它。易火需要系統管理員權限，每次開啟 Windows 都會詢問，請按「是」。SmartTorch 不會修改或讀取它的資料。 A new ETor page: a launcher for ETor, another author's income tracker. Install it and open it once, and SmartTorch remembers it (or Choose…); then open it from SmartTorch. It needs administrator rights, so Windows asks each time: press Yes. SmartTorch neither changes nor reads its data.

**其他 · Other**
- 「設定 › 一般」的「開遊戲時自動啟動」改為每個工具各自一個開關：渴癮症助手、囚獸助手、易火。原本開啟的人兩個助手都保持開啟；易火預設關閉，而且只會自動開啟，不會在關遊戲時關閉。 Settings › General: starting with the game is now one switch per tool: Vorax helper, Tartarus helper and ETor. If it was on, both helpers stay on; ETor is off by default, and it's only opened, never closed with the game.

## v1.0.0

**火炬之光：無限 輔助器中文版 改名為 SmartTorch，成為工具箱 · The Chinese edition is now SmartTorch, a toolbox**
- 新的主視窗：左邊選工具，右邊看狀態、統計與設定；右下角只剩一個圖示。按 ✕ 只會縮到右下角。 A main window: the tools on the left, their state, statistics and settings on the right; one tray icon. ✕ only puts it in the tray.
- 介面有繁體、简体、English 三種（第一次開啟依 Windows 語言選擇）；英文時遊戲上的提示也用英文卡名。 Traditional Chinese, Simplified Chinese or English (from Windows' language at the first start); in English the hint over the game uses the cards' English names.
- 開機自動啟動 SmartTorch；開遊戲時自動啟動工具，關遊戲時自動停止。 Starts with Windows; the tools start with the game and stop when it closes.
- 第一次使用的教學、疑難排解、關於頁；完全免費的聲明。 Getting started, troubleshooting and about pages; a note that it's free.

**囚獸助手 · Tartarus helper**
- 整合 glcoder 的 Tartarus Solver：自行下載後，SmartTorch 會自動找到它、在背後啟動並收起它的視窗，按「設定」才顯示；顯示局數與彩色囚犯統計，可在 SmartTorch 重設；有新版本時提示。 glcoder's Tartarus Solver: download it and SmartTorch finds it, starts it with its window out of the way (Settings shows it), shows its runs and Chromatic Inmates (reset in SmartTorch) and tells you when it has a new version.

**渴癮症助手 · Vorax helper**
- 主視窗顯示連接狀態（遊戲視窗與解析度、遊戲日誌、文字辨識、最後讀取），遊戲日誌可按「選擇…」手動指定，不必再改 bat 檔。 The main window shows the connection (game window and resolution, the game's log, text reading, the last read); Choose… sets the log by hand, no more bat files.
- 模式、雪莉等級、提示與讀取結果開關都在主視窗；可「停止」與「啟動」。 Mode, Shelley level and the hint switches are in the main window, with Stop and Start.
- 統計（局數、彩箱、平均分數）與「清除紀錄」。 Statistics (runs, reached 840k, mean score) and Clear records.

**回報問題 · Reporting a problem**
- 不必再先開除錯模式：讀錯時按 Ctrl + Alt + B 記下畫面，之後按「回報問題」，桌面會出現回報用的 zip，並開啟回報頁。 No debug start needed: Ctrl + Alt + B sets the misread picture aside, then Report a problem makes a zip on the desktop and opens the report page.

**其他 · Other**
- 新圖示；程式改名為 SmartTorch.exe；不再需要 start-zh.bat。 A new icon; the program is SmartTorch.exe; start-zh.bat is gone.
- 從 v0.2.2-zh.3 會自動更新到這一版，設定會保留。 v0.2.2-zh.3 updates itself to this version, keeping its settings.

## v0.2.2-zh.3

**不必再設定遊戲路徑**
- 工具會從開啟中的遊戲找到它的安裝位置，再找到遊戲日誌，不論 Steam 或官方啟動器、裝在哪個磁碟；也會檢查各磁碟常見的安裝資料夾。
- `start-zh.bat` 第 5 行的 `GAMELOG=` 預設留空，只有系統匣顯示「找不到遊戲日誌」時才需要填。之前填好路徑的人照樣可用。
- 「找不到遊戲日誌」的說明改為一步一步的做法。

**其他**
- 衝分模式不再標示「測試中」。
- 說明文件新增「讀不到、讀錯時怎麼辦」，一步一步排除，仍不行再回報。
- 補充：系統匣圖示在 Windows 11 可能收在「^」裡；`vorax-overlay.exe` 就是中文版主程式（檔名和原作者相同）。

## v0.2.2-zh.2

**簡體中文客戶端**
- 正式支援簡體中文客戶端（2560×1440、3840×2160 皆已測試）。
- 修正簡體客戶端培養槽和刷新按鈕上的「1」被讀成「4」或讀不到、卡名中的「-」被當成國字「一」。

**3840×2160（4K）**
- 修正藥劑階段的標題被認成手術方案：之前因此讀不到刷新次數（工具以為沒得刷新，從不建議刷新）、藥劑箱的 5 張卡只讀到 3 張。
- 修正內容簡單的卡（例如「藥劑箱（大）」）被當成「卡片還在移動」，整回合都不給建議。
- 修正蠟燭火光被當成數字，讓培養槽讀錯（例如 5×24 讀成 15×8）；數字正確但字形略不同時不再一直顯示「未讀到」。

**辨識**
- 「疫區聖母像」被雕像擋住標題時也能辨認。
- 卡名讀不到時，會再用不同大小讀一次。

**更新與回報**
- 自動更新改為本專案的中文版：有新版本時系統匣會提示，點「立即更新」即可（會檢查檔案的 SHA-256）。v0.2.2-zh.1 的使用者需要手動下載本版一次。
- 新增 GitHub「問題回報」表單。

## v0.2.2-zh.1

第一個公開版本，以原作者 DinoRaos/vorax-solver-rs v0.2.2 為基礎。

**讀取中文客戶端**
- 用 Windows 內建的中文文字辨識讀卡名，比對中文卡名表（85 張卡），簡繁視為同字，漏讀一兩個字仍能辨認；分不清時標為「未知」，不亂猜。
- 培養槽、回合數的讀取針對中文介面修正；抱卵蝶、異形螯針加入圖片樣本，卡名讀不到時也能辨認。
- 有卡或培養槽沒讀到時，1 秒後自動重讀，並在提示框列出「未讀到」的位置。

**讓玩家核對讀取結果**
- 每格培養槽下方顯示讀到的種族與活性×數量，提示框下方顯示讀到的卡，刷新按鈕旁顯示讀到的刷新次數（沒讀到顯示橘色「？」）。Ctrl + Alt + D 切換顯示。

**計算**
- 新增「衝分」模式：彩箱機率差不多時選平均分數較高的選項，不再接近 840k 就收手。預設仍是原作者的「保守」模式。

**介面**
- 提示框、系統匣選單、說明、對局紀錄全部中文化；對局紀錄可拖曳改變大小、Ctrl + 滾輪縮放，並附實戰統計（預計與實際的比較）。
