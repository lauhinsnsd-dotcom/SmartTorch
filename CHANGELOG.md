# 版本紀錄 · Changelog

每個版本更新了什麼。有新版本時，SmartTorch 的更新提示也會顯示該版本的內容。
What each version changed. When a new version is out, SmartTorch's update note shows its part.

## v2.0.1

**解析度 · Resolutions**
- 支援 16:10 螢幕（2560×1600 等）：以前培養槽全部讀不到、卡片常讀錯，現在介面縮放用推薦值或最小值都讀得準。 16:10 screens (2560×1600 and the like) are supported: the tanks weren't read at all and cards were often misread; at the recommended or the smallest interface scale they now read well.
- 修正：名字較短的怪物（例如弓兵、輕弩兵、赤鏜螂）的種類圖示，常被誤認為覺醒者（2560×1440 等解析度都有）。 Fixed: monsters with short names (弓兵, 輕弩兵, 赤鏜螂…) often had their family icon taken for Agito's (at 2560×1440 too).
- 修正：介面縮放用推薦值時，手上有 5 張藥劑卡（藥劑箱之後）中間那張常讀不到，旁邊的卡也可能讀錯（例如靶向異種激素被當成異種激素，建議放兩格）。 Fixed: at the recommended interface scale, a hand of five potions (after a Potion Kit) often missed its middle card and could misread the one beside it (靶向異種激素 taken for 異種激素, advised on two tanks).
- 修正：「靶向異種激素」有時被讀成「異種激素」（卡名前面兩個字沒讀到），建議因此放錯格。現在卡名剛好是另一張卡名的後半段時，會再用卡面圖案確認（所有解析度）。 Fixed: 靶向異種激素 was at times read as 異種激素 (its first two characters missed), and advised on the wrong tanks. A title that is the end of a longer card's name is now checked against the card's picture (at every resolution).
- 修正：畫面剛打開、顏色偏暗時，第一回合有時讀不到回合數和卡片。 Fixed: with the screen just opened and its colours a little dark, the first round's number and cards were sometimes not read.
- 主視窗的解析度說明更新：支援 1920×1080、2560×1080、2560×1440、2560×1600 和 3840×2160（3840×2160 建議用最小介面縮放），這些解析度不再顯示「此解析度未測試」。 The main window's resolutions are brought up to date: 1920×1080, 2560×1080, 2560×1440, 2560×1600 and 3840×2160 (the smallest interface scale advised at 3840×2160) no longer show as untested.

**渴癮症助手 · Vorax helper**
- 讀不到卡時的選卡框新增「沒有這張卡」：選項裡都沒有時按它，工具不會記住錯誤的卡名，並會自動記下這個畫面，之後按「回報問題」就會附上。 The picker's card for a card not read has "None of these": when none of the choices is the card, press it. Nothing wrong is learned, and the picture is set aside for Report a problem.

**更新 · Updates**
- 更新提示的「更新內容」不再把中文字顯示成方格（改用 Windows 內建的中文字型）。由於顯示更新內容的是舊版本，這個修正要到下一次更新時才看得到。 The update's What's new no longer shows Chinese characters as boxes (Windows' own Chinese font draws them). The notes are shown by the version you have, so this shows from the next update on.

## v2.0.0

**渴癮症助手：模擬資料修正 · Vorax helper: the simulation's data corrected**
- 對照兩位玩家 63 局實戰紀錄後發現，原資料的怪物機率與遊戲不符：新出的怪是首領約 3%（原資料 16%），魔法怪較多。建議裡模擬的未來改用實戰機率，評估局面的公式也以新機率重新訓練（同一批 1,500 局比較：原作者模式達 840k 由 52.7% 升至 56.2%）。顯示的「達 840k」機率會比以前低，但更接近實戰。 Two players' 63 runs showed the original data's monster odds don't match the game: a new monster is a boss about 3% of the time (16% in the data), and magic more often. The advice's futures now use the live odds, and the board value judging boards was trained again with them (the same 1,500 games: upstream's mode reached 840k 56.2% of the time, up from 52.7%). The odds of 840k shown are lower than before, and closer to what you get.
- 說明中的模擬數字已更新為實戰機率下的結果。 The simulated figures in the guide are now the live odds' ones.
- 育雛的後代卡（子一代至子三代：寄生蟲卵、爬蟲卵簇、寄生蟲蛹、爬蟲蛹簇、寄生蝶、破繭蝶群）不在原資料中，以前讀不到；現在能認出，並標示「出現就選」。 The cards a Xenopanacea: Brood leaves (子一代 to 子三代), not in the original data, weren't read; they're now told and marked as always taken.
- 解剖系列卡（解剖浸液：病軀 等五張）改為「出現就選」：有實戰紀錄全部拿齊後打到 1,800 萬以上（參考 DinoRaos/vorax-solver-rs v0.3.0）。 The Dissection cards (解剖浸液：病軀 and the four after it) are now always taken: a recorded game that took them all ended past 18M (after DinoRaos/vorax-solver-rs v0.3.0).

**渴癮症助手 · Vorax helper**
- 離開渴癮畫面一定會收起提示和選卡框：遊戲日誌沒記到時，SmartTorch 也會自己發現（約 10 秒內）。 Leaving the Vorax screen always takes the hints and the picker's card away: when the game's log doesn't say so, SmartTorch notices by itself (within about 10 s).
- 在城鎮等地方按 Ctrl+Alt+X 不再讀出錯誤內容，會提示「現在不是渴癮畫面」。 Ctrl+Alt+X outside Vorax no longer reads the town as a board: it says it isn't the Vorax screen.
- 減少卡頓：計算讓出 CPU 給遊戲（較低優先級，並留 1–2 個核心）。 Fewer hitches: the advice leaves the game the CPU first (below normal priority, a core or two left free).
- Windows 截圖失敗或全黑時，自動改用另一種方式；仍擷取不到畫面時，「遊戲視窗」顯示紅點，疑難排解有處理方法。 When Windows' capture fails or comes back black, another way is used; still no picture, the Game window line shows a red dot, with what to do in Troubleshooting.
- 修正：實戰統計少計了最後一回合的增長（最後讀到的是第 13 回合選手術方案的畫面，之後活性還會再長一次），有些過了 84 萬的局沒算進去。現在以最後讀到的局面加上之後的增長計算最終分數；「彩箱局數」改名為「84萬以上局數」。 Fixed: your results left out the last round's growth (the last board read is round 13's protocol pick, and the activity grows once more after it), so some runs past 840k weren't counted. The final score is now the last board read with the growth after it; "彩箱局數" is now "84萬以上局數".
- 對局紀錄保留多 4 倍，打幾百局統計也不會減少；回報問題會附上全部紀錄。 Four times the runs are kept, so the statistics last for hundreds of runs; reports carry them all.
- 紀錄和回報檔不再含電腦使用者名稱（路徑中的使用者資料夾顯示為 %USERPROFILE%），公開貼出也不會洩露。 Logs and reports no longer carry the PC's user name (the home folder in paths shows as %USERPROFILE%), so they can be posted in public.
- 移除 Ctrl+Alt+Q（關閉 SmartTorch）；請用右下角圖示右鍵 →「結束」。 Ctrl+Alt+Q (quit) is gone: right-click the tray icon → Quit.

**更新 · Updates**
- 網絡不穩時也能更新：下載中斷會從斷點續傳，卡住會自動重試。 Updates get through an unsteady connection: a download cut off goes on from where it stopped, a stalled one is tried again.
- 「檢查更新」連不上 GitHub 時會顯示「檢查失敗」，不再誤報「已是最新版本」；更新失敗會提示怎麼做。 Check for updates says when it can't reach GitHub, instead of "the latest"; a failed update says what to do.

**囚獸助手 · Tartarus helper**
- 修正：在 SmartTorch 裡更新囚獸助手會失敗（更新途中被重新開啟）。 Fixed: updating the Tartarus Solver from SmartTorch failed (it was started again mid-update).

## v1.0.7

**渴癮症助手 · Vorax helper**
- 修正：在城鎮等渴癮以外的畫面切換「衝分／原作者」模式時，會把畫面誤讀成培養槽（出現「?」「空」和「畫面未完全讀到」），讀取時也會令電腦忙碌很久。現在切換模式或雪莉等級只會用上一次讀到的渴癮局面重新計算，不會讀取畫面；不在渴癮畫面時不做任何事。 Fixed: switching between Push score and Upstream's outside Vorax (in town, say) read the screen as tanks (? and 空, "Board not fully read") and kept the PC busy for a long while. Changing the mode or Intern Shelley's level now works the last Vorax board out again without reading the screen, and does nothing outside Vorax.

## v1.0.6

**1920×1080 · 1920×1080**
- 1920×1080 請把遊戲的「介面縮放」設為推薦值（按「使用推薦值」）或最小值，兩種都已實測讀得準。拉到最大時卡片位置對不上，會讀不到，暫不支援。 At 1920×1080, set the game's interface scale to its recommended value (Use Recommended) or the minimum: both are tested and read well. At the maximum the cards don't line up and aren't read, so it isn't supported for now.
- 修正：用推薦值時，剛開啟 SmartTorch 的第一個畫面可能被當成「卡片還在飛」而讀不到。 Fixed: at the recommended scale, the first board after SmartTorch starts could be taken for a card still in flight and not read.
- 設定說明與疑難排解已加入上述建議。 The guide and Troubleshooting now say so.

**渴癮症助手 · Vorax helper**
- 修正：手動選卡框未選就自己消失（按 Ctrl+Alt+B 記下畫面時，或重讀多認出一張卡時）。現在選卡框出現後不會再自動重讀，直到你選好，或自己在遊戲中選卡、刷新；按 Ctrl+Alt+X 仍可重讀，認出的卡會從選卡框移除。你選的卡一定生效。 Fixed: the picker's card closing before a pick (on Ctrl+Alt+B, or when a reread made out one more card). Once it's up, nothing is read by itself until you pick, or play the round in the game; Ctrl+Alt+X still reads, and the cards made out leave the picker. A card you pick always counts.
- 修正：「藥劑箱（大）」常讀不到（括號內的字沒讀到，分不出大小），現在會用卡面圖案分辨。 Fixed: Potion Kit (Large) often not read (the size in brackets missed): its picture now tells the two apart.
- 修正：同一系列的卡只讀到幾個字時（例如「孿生 - 骨」），讀到只屬於某一張的字就能認出。 Fixed: a card of a family read only in part (such as 孿生 - 骨) is told by the characters only that version has.

## v1.0.5

**渴癮症助手 · Vorax helper**
- 新增手動選卡：卡片重讀後仍讀不到時，遊戲左下角會列出最可能的 5 張卡讓你選（一次一張，由左至右）。選好立即重新計算建議，並記住這張卡，下次讀到同樣的字會自動認出。 New: picking a card not read. When a card still isn't read after the rereads, the five likeliest cards are listed in the game's bottom left corner (one card at a time, left to right). The advice is worked out again with the card picked, and it's known by itself next time.
- 選卡期間不顯示建議卡、培養槽和刷新的框，選好才顯示，避免照著不完整的建議按錯。 While a card is being asked about, the frames on the card, tanks and reroll button stay hidden until it's picked.
- 讀到的卡名改為顯示在卡片上方（「選擇一種藥劑」的上面），對照時不用上下望。 The cards read are shown just above the cards (over the "pick" banner), so checking them needs no looking up and down.
- 「渴癮症助手」頁可關閉手動選卡，或清除已學的卡；回報檔會附上已學的卡。 The Vorax helper page turns the picker off and forgets the learned cards; reports include them.

**其他 · Other**
- SmartTorch.exe 加上名稱、版本和作者資料（檔案「內容 › 詳細資料」）。 SmartTorch.exe carries its name, version and maker (Properties › Details).

## v1.0.4

**渴癮症助手 · Vorax helper**
- 中文客戶端支援 1920×1080：卡名讀不到時，改用黑白版本再讀；「痳」當作「麻」；只讀到一半的卡名也較容易辨認。 A Chinese client at 1920×1080: a card name not read is read again in black and white; 痳 counts as 麻; a name half read is told more often.
- 英文客戶端 1920×1080：卡名只讀到一部分時（例如「Claws」），若只符合一張卡就認得出。 An English client at 1920×1080: a card name read only in part (such as "Claws") is told when it fits one card alone.
- 2560×1440 最準；3840×2160 和 1920×1080 只測試過幾十局，遇到讀錯請多回報。 2560×1440 reads best; 3840×2160 and 1920×1080 have had a few dozen games of testing, so please report misreads.

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
