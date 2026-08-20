//! The strings.
//!
//! Worded for a tool that FIXES things. Where the web can only refuse -- "re-
//! export it without audio (ffmpeg -an)" -- Press says what it found and what it
//! is about to do about it. The facts are the same because they come from the
//! same gate; the sentence is not, because the reader's situation is not.
//!
//! `{braces}` are interpolated by the caller. Every language must carry the same
//! set of them, which `super::tests` asserts -- a placeholder that exists in one
//! language and not another renders as literal braces for whoever reads that
//! language, and that is a bug only found by someone who reads it.

use super::Text;

// ── the window ───────────────────────────────────────────────────────────

pub static WINDOW_TITLE: Text = Text {
    en: "Euterpe Press",
    zh_hans: "Euterpe Press",
    zh_hant: "Euterpe Press",
};

pub static TAGLINE: Text = Text {
    en: "Background video, ready to upload",
    zh_hans: "把背景视频弄成能直接上传的样子",
    zh_hant: "把背景影片弄成能直接上傳的樣子",
};

pub static DROP_PROMPT: Text = Text {
    en: "Drop a video here",
    zh_hans: "把视频拖进来",
    zh_hant: "把影片拖進來",
};

pub static DROP_HINT: Text = Text {
    en: "or click to choose one",
    zh_hans: "或者点一下选文件",
    zh_hant: "或者點一下選檔案",
};

pub static DROP_RELEASE: Text = Text {
    en: "Let go",
    zh_hans: "松手",
    zh_hant: "放開",
};

pub static NOT_A_VIDEO: Text = Text {
    en: "That is not a video file",
    zh_hans: "这不是视频文件",
    zh_hant: "這不是影片檔案",
};

// ── the verdict ──────────────────────────────────────────────────────────

pub static READING: Text = Text {
    en: "Reading it…",
    zh_hans: "正在读…",
    zh_hant: "正在讀…",
};

pub static VERDICT_OK: Text = Text {
    en: "Ready to upload as it is",
    zh_hans: "已经符合要求，可以直接上传",
    zh_hant: "已經符合要求，可以直接上傳",
};

pub static VERDICT_WILL_FIX: Text = Text {
    en: "Press will fix this",
    zh_hans: "有问题，Press 会自动修好",
    zh_hant: "有問題，Press 會自動修好",
};

/// The one case Press cannot help with: nothing readable to work from.
pub static VERDICT_UNREADABLE: Text = Text {
    en: "This file's header could not be read, so there is nothing to work from. \
         Export it again as a standard MP4.",
    zh_hans: "读不出这个文件的信息，没有可处理的东西。请重新导出成标准 MP4。",
    zh_hant: "讀不出這個檔案的資訊，沒有可處理的東西。請重新匯出成標準 MP4。",
};

/// Shown when the source itself is over the size limit. Not a finding on its own
/// -- re-encoding is what brings it down -- but worth naming, because a file that
/// only fits at the bottom of the quality ladder will look softer and the
/// charter should know why.
pub static FINDING_TOO_LARGE: Text = Text {
    en: "{size} MB, over the {limit} MB limit",
    zh_hans: "{size} MB，超过 {limit} MB 上限",
    zh_hant: "{size} MB，超過 {limit} MB 上限",
};

pub static FINDING_AUDIO: Text = Text {
    en: "Has an audio track",
    zh_hans: "带音轨",
    zh_hant: "帶音軌",
};

pub static FIX_AUDIO: Text = Text {
    en: "removed — the game plays music.ogg, so nobody ever hears it",
    zh_hans: "会去掉 —— 游戏播的是 music.ogg，这条音轨没有人听得到",
    zh_hant: "會去掉 —— 遊戲播的是 music.ogg，這條音軌沒有人聽得到",
};

pub static FINDING_RESOLUTION: Text = Text {
    en: "{width}×{height}, over the pixel budget",
    zh_hans: "{width}×{height}，像素总数超标",
    zh_hant: "{width}×{height}，像素總數超標",
};

pub static FIX_RESOLUTION: Text = Text {
    en: "scaled to {width}×{height}",
    zh_hans: "会缩到 {width}×{height}",
    zh_hant: "會縮到 {width}×{height}",
};

pub static FINDING_FPS: Text = Text {
    en: "{fps} fps, over the {limit} fps limit",
    zh_hans: "{fps} fps，超过 {limit} fps 上限",
    zh_hant: "{fps} fps，超過 {limit} fps 上限",
};

pub static FIX_FPS: Text = Text {
    en: "brought down to {limit} fps",
    zh_hans: "会降到 {limit} fps",
    zh_hant: "會降到 {limit} fps",
};

pub static FINDING_CODEC: Text = Text {
    en: "{codec}, which some players cannot decode",
    zh_hans: "{codec}，一部分玩家的机器解不了",
    zh_hant: "{codec}，一部分玩家的機器解不了",
};

pub static FIX_CODEC: Text = Text {
    en: "re-encoded to AV1, which plays on every machine",
    zh_hans: "会转成 AV1，每台机器都能播",
    zh_hant: "會轉成 AV1，每台機器都能播",
};

// ── the trim, the one thing the charter decides ──────────────────────────

pub static TRIM_TITLE: Text = Text {
    en: "Trim",
    zh_hans: "裁剪",
    zh_hant: "裁剪",
};

pub static TRIM_NONE: Text = Text {
    en: "Whole video",
    zh_hans: "整段",
    zh_hant: "整段",
};

pub static TRIM_TAIL_ONLY: Text = Text {
    en: "Keeping the first {kept}, dropping {cut} from the end",
    zh_hans: "保留前 {kept}，从结尾去掉 {cut}",
    zh_hant: "保留前 {kept}，從結尾去掉 {cut}",
};

/// Moving the START is the one action here with a consequence the charter cannot
/// see from the picture, so it is spelled out rather than implied.
pub static TRIM_HEAD_WARNING: Text = Text {
    en: "The video will start {shift} earlier against the music. Only do this if \
         its opening is not meant to line up with the song.",
    zh_hans: "画面会相对音乐整体提前 {shift}。只有当开头本来就不该和歌曲对齐时才这么做。",
    zh_hant: "畫面會相對音樂整體提前 {shift}。只有當開頭本來就不該和歌曲對齊時才這麼做。",
};

pub static TRIM_HEAD_COSTS_REENCODE: Text = Text {
    en: "Cutting the opening also means re-encoding, which takes longer than a \
         plain trim.",
    zh_hans: "裁开头会连带触发重编码，比单纯裁尾巴慢。",
    zh_hant: "裁開頭會連帶觸發重編碼，比單純裁尾巴慢。",
};

pub static TRIM_RESET: Text = Text {
    en: "Reset",
    zh_hans: "还原",
    zh_hant: "還原",
};

// ── doing it ─────────────────────────────────────────────────────────────

pub static START: Text = Text {
    en: "Make it uploadable",
    zh_hans: "开始处理",
    zh_hant: "開始處理",
};

pub static STEP_COPYING: Text = Text {
    en: "Fixing it losslessly…",
    zh_hans: "无损修复中…",
    zh_hant: "無損修復中…",
};

pub static STEP_ENCODING: Text = Text {
    en: "Trying AV1 — it is kept only if it comes out smaller ({percent})",
    zh_hans: "正在试 AV1 —— 只有更小才会采用（{percent}）",
    zh_hant: "正在試 AV1 —— 只有更小才會採用（{percent}）",
};

pub static STEP_ENCODING_FORCED: Text = Text {
    en: "Re-encoding to AV1 ({percent})",
    zh_hans: "正在转成 AV1（{percent}）",
    zh_hant: "正在轉成 AV1（{percent}）",
};

pub static STEP_VERIFYING: Text = Text {
    en: "Checking the result…",
    zh_hans: "正在校验结果…",
    zh_hant: "正在校驗結果…",
};

/// Cancelling when a compliant stream copy is already in hand.
pub static CANCEL_KEEP: Text = Text {
    en: "Stop and use what I have",
    zh_hans: "不等了，就用现在这个",
    zh_hant: "不等了，就用現在這個",
};

/// Cancelling when the encode IS the fix and stopping leaves nothing.
pub static CANCEL_ABANDON: Text = Text {
    en: "Give up on this file",
    zh_hans: "放弃这个文件",
    zh_hant: "放棄這個檔案",
};

pub static CANCEL_ABANDON_HINT: Text = Text {
    en: "This one can only be fixed by re-encoding, so stopping leaves nothing.",
    zh_hans: "这个文件只能靠重编码修好，停下来就什么都不会有。",
    zh_hant: "這個檔案只能靠重編碼修好，停下來就什麼都不會有。",
};

// ── the result ───────────────────────────────────────────────────────────

pub static DONE: Text = Text {
    en: "Done — this file will be accepted",
    zh_hans: "完成 —— 这个文件会被接受",
    zh_hant: "完成 —— 這個檔案會被接受",
};

pub static DONE_SIZE: Text = Text {
    en: "{before} → {after}",
    zh_hans: "{before} → {after}",
    zh_hant: "{before} → {after}",
};

pub static DONE_UNTOUCHED: Text = Text {
    en: "Picture untouched — nothing was re-encoded",
    zh_hans: "画面没有被动过 —— 全程没有重编码",
    zh_hant: "畫面沒有被動過 —— 全程沒有重編碼",
};

pub static DONE_AV1_LOST: Text = Text {
    en: "AV1 was tried and came back larger, so the original picture was kept",
    zh_hans: "试过 AV1，结果反而更大，所以保留了原画面",
    zh_hant: "試過 AV1，結果反而更大，所以保留了原畫面",
};

pub static DONE_AV1_SKIPPED: Text = Text {
    en: "The AV1 comparison was stopped, so this may not be the smallest possible",
    zh_hans: "AV1 比对被中断了，所以这不一定是最小的版本",
    zh_hant: "AV1 比對被中斷了，所以這不一定是最小的版本",
};

pub static ALREADY_STANDARD: Text = Text {
    en: "Nothing to do — this file is already exactly right",
    zh_hans: "不用动 —— 这个文件本来就完全合规",
    zh_hant: "不用動 —— 這個檔案本來就完全合規",
};

pub static CANCELLED: Text = Text {
    en: "Stopped. Nothing was written.",
    zh_hans: "已停止，没有写出任何文件。",
    zh_hant: "已停止，沒有寫出任何檔案。",
};

pub static FAILED: Text = Text {
    en: "This one did not work out",
    zh_hans: "这个文件没能处理成功",
    zh_hant: "這個檔案沒能處理成功",
};

pub static SHOW_IN_FOLDER: Text = Text {
    en: "Show in folder",
    zh_hans: "打开所在文件夹",
    zh_hant: "開啟所在資料夾",
};

pub static ANOTHER: Text = Text {
    en: "Another one",
    zh_hans: "再来一个",
    zh_hant: "再來一個",
};

// ── the encoder ──────────────────────────────────────────────────────────

pub static NO_ENCODER: Text = Text {
    en: "The encoder is missing from this build, so nothing can be processed.",
    zh_hans: "这个构建里没有带编码器，没法处理任何文件。",
    zh_hant: "這個建置裡沒有帶編碼器，沒法處理任何檔案。",
};

/// Every string, for the tests that hold the table to its own rules.
pub static EVERY_STRING: &[(&str, &Text)] = &[
    ("WINDOW_TITLE", &WINDOW_TITLE),
    ("TAGLINE", &TAGLINE),
    ("DROP_PROMPT", &DROP_PROMPT),
    ("DROP_HINT", &DROP_HINT),
    ("DROP_RELEASE", &DROP_RELEASE),
    ("NOT_A_VIDEO", &NOT_A_VIDEO),
    ("READING", &READING),
    ("VERDICT_OK", &VERDICT_OK),
    ("VERDICT_WILL_FIX", &VERDICT_WILL_FIX),
    ("VERDICT_UNREADABLE", &VERDICT_UNREADABLE),
    ("FINDING_TOO_LARGE", &FINDING_TOO_LARGE),
    ("FINDING_AUDIO", &FINDING_AUDIO),
    ("FIX_AUDIO", &FIX_AUDIO),
    ("FINDING_RESOLUTION", &FINDING_RESOLUTION),
    ("FIX_RESOLUTION", &FIX_RESOLUTION),
    ("FINDING_FPS", &FINDING_FPS),
    ("FIX_FPS", &FIX_FPS),
    ("FINDING_CODEC", &FINDING_CODEC),
    ("FIX_CODEC", &FIX_CODEC),
    ("TRIM_TITLE", &TRIM_TITLE),
    ("TRIM_NONE", &TRIM_NONE),
    ("TRIM_TAIL_ONLY", &TRIM_TAIL_ONLY),
    ("TRIM_HEAD_WARNING", &TRIM_HEAD_WARNING),
    ("TRIM_HEAD_COSTS_REENCODE", &TRIM_HEAD_COSTS_REENCODE),
    ("TRIM_RESET", &TRIM_RESET),
    ("START", &START),
    ("STEP_COPYING", &STEP_COPYING),
    ("STEP_ENCODING", &STEP_ENCODING),
    ("STEP_ENCODING_FORCED", &STEP_ENCODING_FORCED),
    ("STEP_VERIFYING", &STEP_VERIFYING),
    ("CANCEL_KEEP", &CANCEL_KEEP),
    ("CANCEL_ABANDON", &CANCEL_ABANDON),
    ("CANCEL_ABANDON_HINT", &CANCEL_ABANDON_HINT),
    ("DONE", &DONE),
    ("DONE_SIZE", &DONE_SIZE),
    ("DONE_UNTOUCHED", &DONE_UNTOUCHED),
    ("DONE_AV1_LOST", &DONE_AV1_LOST),
    ("DONE_AV1_SKIPPED", &DONE_AV1_SKIPPED),
    ("ALREADY_STANDARD", &ALREADY_STANDARD),
    ("CANCELLED", &CANCELLED),
    ("FAILED", &FAILED),
    ("SHOW_IN_FOLDER", &SHOW_IN_FOLDER),
    ("ANOTHER", &ANOTHER),
    ("NO_ENCODER", &NO_ENCODER),
];
