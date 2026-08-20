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
    ja: "Euterpe Press",
    ko: "Euterpe Press",
    de: "Euterpe Press",
    es: "Euterpe Press",
    fr: "Euterpe Press",
    hr: "Euterpe Press",
    hu: "Euterpe Press",
    id: "Euterpe Press",
    nl: "Euterpe Press",
    pt: "Euterpe Press",
    ru: "Euterpe Press",
};

pub static TAGLINE: Text = Text {
    en: "Background video, ready to upload",
    zh_hans: "把背景视频弄成能直接上传的样子",
    zh_hant: "把背景影片弄成能直接上傳的樣子",
    ja: "背景動画を、そのままアップロードできる形に",
    ko: "배경 영상을 바로 업로드할 수 있는 형태로",
    de: "Hintergrundvideo, direkt hochladbar",
    es: "Video de fondo, listo para subir",
    fr: "Vidéo de fond, prête à être envoyée",
    hr: "Pozadinski video, spreman za prijenos",
    hu: "Háttérvideó, feltöltésre készen",
    id: "Video latar, siap diunggah",
    nl: "Achtergrondvideo, klaar om te uploaden",
    pt: "Vídeo de fundo, pronto para enviar",
    ru: "Фоновое видео, готовое к загрузке",
};

pub static DROP_PROMPT: Text = Text {
    en: "Drop a video here",
    zh_hans: "把视频拖进来",
    zh_hant: "把影片拖進來",
    ja: "動画をここにドロップ",
    ko: "영상을 여기에 놓으세요",
    de: "Video hierher ziehen",
    es: "Arrastra un video aquí",
    fr: "Déposez une vidéo ici",
    hr: "Ovdje ispustite video",
    hu: "Húzz ide egy videót",
    id: "Letakkan video di sini",
    nl: "Sleep hier een video naartoe",
    pt: "Solte um vídeo aqui",
    ru: "Перетащите видео сюда",
};

pub static DROP_HINT: Text = Text {
    en: "or click to choose one",
    zh_hans: "或者点一下选文件",
    zh_hant: "或者點一下選檔案",
    ja: "またはクリックして選択",
    ko: "또는 클릭해서 선택",
    de: "oder klicken, um eines auszuwählen",
    es: "o haz clic para elegir uno",
    fr: "ou cliquez pour en choisir une",
    hr: "ili kliknite za odabir",
    hu: "vagy kattints a kiválasztáshoz",
    id: "atau klik untuk memilih",
    nl: "of klik om er een te kiezen",
    pt: "ou clique para escolher um",
    ru: "или нажмите, чтобы выбрать",
};

pub static DROP_RELEASE: Text = Text {
    en: "Let go",
    zh_hans: "松手",
    zh_hant: "放開",
    ja: "離してください",
    ko: "놓으세요",
    de: "Loslassen",
    es: "Suelta",
    fr: "Lâchez",
    hr: "Pustite",
    hu: "Engedd el",
    id: "Lepaskan",
    nl: "Laat los",
    pt: "Solte",
    ru: "Отпустите",
};

/// The drop zone's headline when dragging cannot work. It stops offering the
/// thing that is broken and offers the thing that is not.
pub static DROP_PICK_INSTEAD: Text = Text {
    en: "Choose a video",
    zh_hans: "选一个视频",
    zh_hant: "選一個影片",
    ja: "動画を選択",
    ko: "영상 선택",
    de: "Video auswählen",
    es: "Elige un vídeo",
    fr: "Choisir une vidéo",
    hr: "Odaberite videozapis",
    hu: "Válassz videót",
    id: "Pilih video",
    nl: "Kies een video",
    pt: "Escolha um vídeo",
    ru: "Выберите видео",
};

/// Shown in the drop zone when Press is running elevated, where Windows blocks
/// the drop and says nothing about why.
pub static DROP_BLOCKED_BY_ADMIN: Text = Text {
    en: "Running as administrator, so Windows blocks dragging files in. \
         Click here to choose one, or reopen Press normally.",
    zh_hans: "正在以管理员身份运行，Windows 会挡住拖放。点这里选文件，或者用普通权限重新打开。",
    zh_hant: "正在以系統管理員身分執行，Windows 會擋住拖放。點這裡選檔案，或者用一般權限重新開啟。",
    ja: "管理者として実行中のため、Windows がドラッグ＆ドロップを遮断しています。ここをクリックして選ぶか、通常の権限で開き直してください。",
    ko: "관리자 권한으로 실행 중이라 Windows가 드래그 앤 드롭을 막습니다. 여기를 클릭해 선택하거나, 일반 권한으로 다시 여세요.",
    de: "Läuft als Administrator, daher blockiert Windows das Hineinziehen von Dateien. \
         Hier klicken, um eine auszuwählen, oder Press normal öffnen.",
    es: "Se está ejecutando como administrador, así que Windows bloquea arrastrar archivos. \
         Haz clic aquí para elegir uno o vuelve a abrir Press con permisos normales.",
    fr: "Exécuté en tant qu'administrateur : Windows bloque le glisser-déposer. \
         Cliquez ici pour en choisir une, ou rouvrez Press normalement.",
    hr: "Pokrenuto kao administrator, pa Windows blokira povlačenje datoteka. \
         Kliknite ovdje da odaberete datoteku ili ponovno otvorite Press s običnim ovlastima.",
    hu: "Rendszergazdaként fut, ezért a Windows letiltja a fájlok behúzását. \
         Kattints ide a kiválasztáshoz, vagy nyisd meg a Presst normál jogosultsággal.",
    id: "Berjalan sebagai administrator, jadi Windows memblokir seret-dan-lepas. \
         Klik di sini untuk memilih berkas, atau buka ulang Press dengan hak biasa.",
    nl: "Draait als administrator, dus Windows blokkeert het slepen van bestanden. \
         Klik hier om er een te kiezen, of open Press gewoon opnieuw.",
    pt: "Executando como administrador, então o Windows bloqueia arrastar arquivos. \
         Clique aqui para escolher um ou reabra o Press normalmente.",
    ru: "Запущено от имени администратора, поэтому Windows блокирует перетаскивание. \
         Нажмите здесь, чтобы выбрать файл, или откройте Press с обычными правами.",
};

// ── the verdict ──────────────────────────────────────────────────────────

pub static READING: Text = Text {
    en: "Reading it…",
    zh_hans: "正在读…",
    zh_hant: "正在讀…",
    ja: "読み込み中…",
    ko: "읽는 중…",
    de: "Wird gelesen…",
    es: "Leyendo…",
    fr: "Lecture…",
    hr: "Čitanje…",
    hu: "Beolvasás…",
    id: "Membaca…",
    nl: "Bezig met lezen…",
    pt: "Lendo…",
    ru: "Чтение…",
};

/// Heads the list of changes on the inspect screen.
///
/// A SENTENCE, not a label. It read "要改的地方" -- a noun phrase -- while the
/// screen after it read "还不能上传", a statement. One slot, two grammars, on
/// consecutive screens.
///
/// Before that it read "有问题，Press 会自动修好": two clauses for one fact, a
/// promise rather than a statement, and the product naming itself to the person
/// already looking at it.
pub static VERDICT_WILL_FIX: Text = Text {
    en: "Fix these before uploading",
    zh_hans: "上传前要改这几处",
    zh_hant: "上傳前要改這幾處",
    ja: "アップロードの前にここを直します",
    ko: "업로드 전에 이것들을 고칩니다",
    de: "Das wird vor dem Hochladen geändert",
    es: "Esto se corrige antes de subirlo",
    fr: "À corriger avant l'envoi",
    hr: "Ovo treba popraviti prije slanja",
    hu: "Ezeket javítjuk feltöltés előtt",
    id: "Ini diperbaiki sebelum diunggah",
    nl: "Dit wordt voor het uploaden hersteld",
    pt: "Isto será corrigido antes do envio",
    ru: "Это исправляется перед загрузкой",
};

/// The one case Press cannot help with: nothing readable to work from.
pub static VERDICT_UNREADABLE: Text = Text {
    en: "This file's header could not be read, so there is nothing to work from. \
         Export it again as a standard MP4.",
    zh_hans: "读不出这个文件的信息，没有可处理的东西。请重新导出成标准 MP4。",
    zh_hant: "讀不出這個檔案的資訊，沒有可處理的東西。請重新匯出成標準 MP4。",
    ja: "このファイルの情報が読み取れないため、処理のしようがありません。標準的な MP4 として書き出し直してください。",
    ko: "이 파일의 정보를 읽을 수 없어 처리할 수 없습니다. 표준 MP4로 다시 내보내 주세요.",
    de: "Der Header dieser Datei ist nicht lesbar, also gibt es nichts zu bearbeiten. \
         Exportiere sie erneut als Standard-MP4.",
    es: "No se pudo leer la cabecera de este archivo, así que no hay nada con qué trabajar. \
         Vuelve a exportarlo como un MP4 estándar.",
    fr: "L'en-tête de ce fichier est illisible, il n'y a donc rien à traiter. \
         Réexportez-le en MP4 standard.",
    hr: "Zaglavlje ove datoteke nije čitljivo, pa nema s čime raditi. \
         Ponovno je izvezite kao standardni MP4.",
    hu: "A fájl fejléce nem olvasható, így nincs mivel dolgozni. \
         Exportáld újra szabványos MP4-ként.",
    id: "Header berkas ini tidak terbaca, jadi tidak ada yang bisa diolah. \
         Ekspor ulang sebagai MP4 standar.",
    nl: "De header van dit bestand is onleesbaar, dus er valt niets te bewerken. \
         Exporteer het opnieuw als standaard-MP4.",
    pt: "Não foi possível ler o cabeçalho deste arquivo, então não há com o que trabalhar. \
         Exporte-o novamente como um MP4 padrão.",
    ru: "Заголовок этого файла не читается, работать не с чем. \
         Экспортируйте его заново как стандартный MP4.",
};

/// Shown when the source itself is over the size limit.
pub static FINDING_TOO_LARGE: Text = Text {
    en: "{size} MB, over the {limit} MB limit",
    zh_hans: "{size} MB，超过 {limit} MB 上限",
    zh_hant: "{size} MB，超過 {limit} MB 上限",
    ja: "{size} MB。上限は {limit} MB です",
    ko: "{size} MB, 제한인 {limit} MB 초과",
    de: "{size} MB, über dem Limit von {limit} MB",
    es: "{size} MB, por encima del límite de {limit} MB",
    fr: "{size} Mo, au-delà de la limite de {limit} Mo",
    hr: "{size} MB, iznad ograničenja od {limit} MB",
    hu: "{size} MB, a(z) {limit} MB-os korlát felett",
    id: "{size} MB, melebihi batas {limit} MB",
    nl: "{size} MB, boven de limiet van {limit} MB",
    pt: "{size} MB, acima do limite de {limit} MB",
    ru: "{size} МБ, больше предела в {limit} МБ",
};

pub static FINDING_AUDIO: Text = Text {
    en: "Has an audio track",
    zh_hans: "带音轨",
    zh_hant: "帶音軌",
    ja: "音声トラックがあります",
    ko: "오디오 트랙이 있습니다",
    de: "Hat eine Tonspur",
    es: "Tiene pista de audio",
    fr: "Contient une piste audio",
    hr: "Ima zvučni zapis",
    hu: "Van benne hangsáv",
    id: "Punya trek audio",
    nl: "Heeft een audiospoor",
    pt: "Tem faixa de áudio",
    ru: "Есть аудиодорожка",
};

/// The reason stays -- "why is my audio being deleted" is a fair question -- but
/// it is one clause, not two. "so nobody ever hears it" restates "the game plays
/// music.ogg" for anyone who read the first half.
pub static FIX_AUDIO: Text = Text {
    en: "removed — the game plays music.ogg",
    zh_hans: "会去掉，游戏播的是 music.ogg",
    zh_hant: "會去掉，遊戲播的是 music.ogg",
    ja: "削除します（ゲームが再生するのは music.ogg）",
    ko: "제거합니다 — 게임은 music.ogg를 재생합니다",
    de: "wird entfernt — das Spiel gibt music.ogg wieder",
    es: "se quitará — el juego reproduce music.ogg",
    fr: "sera retirée — le jeu lit music.ogg",
    hr: "bit će uklonjen — igra svira music.ogg",
    hu: "eltávolítjuk — a játék a music.ogg-ot játssza",
    id: "akan dihapus — gim memutar music.ogg",
    nl: "wordt verwijderd — de game speelt music.ogg",
    pt: "será removida — o jogo toca music.ogg",
    ru: "будет удалена — игра воспроизводит music.ogg",
};

pub static FINDING_RESOLUTION: Text = Text {
    en: "{width}×{height}, over the pixel budget",
    zh_hans: "{width}×{height}，像素总数超标",
    zh_hant: "{width}×{height}，像素總數超標",
    ja: "{width}×{height}。ピクセル総数が上限を超えています",
    ko: "{width}×{height}, 픽셀 수 초과",
    de: "{width}×{height}, über dem Pixelbudget",
    es: "{width}×{height}, por encima del presupuesto de píxeles",
    fr: "{width}×{height}, au-delà du budget de pixels",
    hr: "{width}×{height}, iznad dopuštenog broja piksela",
    hu: "{width}×{height}, a pixelkereten felül",
    id: "{width}×{height}, melebihi jatah piksel",
    nl: "{width}×{height}, boven het pixelbudget",
    pt: "{width}×{height}, acima do limite de pixels",
    ru: "{width}×{height}, больше допустимого числа пикселей",
};

pub static FIX_RESOLUTION: Text = Text {
    en: "scaled to {width}×{height}",
    zh_hans: "会缩到 {width}×{height}",
    zh_hant: "會縮到 {width}×{height}",
    ja: "{width}×{height} に縮小します",
    ko: "{width}×{height}로 축소합니다",
    de: "wird auf {width}×{height} skaliert",
    es: "se escalará a {width}×{height}",
    fr: "sera redimensionnée en {width}×{height}",
    hr: "bit će smanjeno na {width}×{height}",
    hu: "{width}×{height} méretre kicsinyítjük",
    id: "akan diperkecil ke {width}×{height}",
    nl: "wordt geschaald naar {width}×{height}",
    pt: "será redimensionado para {width}×{height}",
    ru: "будет уменьшено до {width}×{height}",
};

pub static FINDING_FPS: Text = Text {
    en: "{fps} fps, over the {limit} fps limit",
    zh_hans: "{fps} fps，超过 {limit} fps 上限",
    zh_hant: "{fps} fps，超過 {limit} fps 上限",
    ja: "{fps} fps。上限は {limit} fps です",
    ko: "{fps} fps, 제한인 {limit} fps 초과",
    de: "{fps} fps, über dem Limit von {limit} fps",
    es: "{fps} fps, por encima del límite de {limit} fps",
    fr: "{fps} fps, au-delà de la limite de {limit} fps",
    hr: "{fps} fps, iznad ograničenja od {limit} fps",
    hu: "{fps} fps, a(z) {limit} fps korlát felett",
    id: "{fps} fps, melebihi batas {limit} fps",
    nl: "{fps} fps, boven de limiet van {limit} fps",
    pt: "{fps} fps, acima do limite de {limit} fps",
    ru: "{fps} fps, больше предела в {limit} fps",
};

pub static FIX_FPS: Text = Text {
    en: "brought down to {limit} fps",
    zh_hans: "会降到 {limit} fps",
    zh_hant: "會降到 {limit} fps",
    ja: "{limit} fps に下げます",
    ko: "{limit} fps로 낮춥니다",
    de: "wird auf {limit} fps reduziert",
    es: "se reducirá a {limit} fps",
    fr: "sera ramenée à {limit} fps",
    hr: "bit će smanjeno na {limit} fps",
    hu: "{limit} fps-re csökkentjük",
    id: "akan diturunkan ke {limit} fps",
    nl: "wordt teruggebracht naar {limit} fps",
    pt: "será reduzido para {limit} fps",
    ru: "будет снижено до {limit} fps",
};

pub static FINDING_CODEC: Text = Text {
    en: "{codec}, which some players cannot decode",
    zh_hans: "{codec}，一部分玩家的机器解不了",
    zh_hant: "{codec}，一部分玩家的機器解不了",
    ja: "{codec}。一部のプレイヤーの環境ではデコードできません",
    ko: "{codec}, 일부 플레이어의 환경에서는 디코딩할 수 없습니다",
    de: "{codec} — manche Spieler können das nicht dekodieren",
    es: "{codec}, que algunos jugadores no pueden decodificar",
    fr: "{codec}, que certains joueurs ne peuvent pas décoder",
    hr: "{codec}, koji neki igrači ne mogu dekodirati",
    hu: "{codec}, amit egyes játékosok gépe nem tud dekódolni",
    id: "{codec}, yang tidak bisa didekode sebagian pemain",
    nl: "{codec}, dat sommige spelers niet kunnen decoderen",
    pt: "{codec}, que alguns jogadores não conseguem decodificar",
    ru: "{codec} — часть игроков не сможет его декодировать",
};

pub static FIX_CODEC: Text = Text {
    en: "re-encoded to AV1, which plays on every machine",
    zh_hans: "会转成 AV1，每台机器都能播",
    zh_hant: "會轉成 AV1，每台機器都能播",
    ja: "AV1 に変換します。どの環境でも再生できます",
    ko: "AV1로 변환합니다. 모든 환경에서 재생됩니다",
    de: "wird zu AV1 neu kodiert, das überall läuft",
    es: "se recodificará a AV1, que funciona en todas las máquinas",
    fr: "sera réencodée en AV1, qui se lit partout",
    hr: "bit će prekodirano u AV1, koji radi na svakom računalu",
    hu: "AV1-re kódoljuk át, ami minden gépen lejátszható",
    id: "akan dikodekan ulang ke AV1, yang bisa diputar di semua mesin",
    nl: "wordt omgezet naar AV1, dat op elke machine werkt",
    pt: "será recodificado para AV1, que toca em qualquer máquina",
    ru: "будет перекодировано в AV1, который играет на любой машине",
};

// ── the trim, the one thing the charter decides ──────────────────────────

pub static TRIM_TITLE: Text = Text {
    en: "Trim",
    zh_hans: "裁剪",
    zh_hant: "裁剪",
    ja: "トリミング",
    ko: "자르기",
    de: "Zuschneiden",
    es: "Recortar",
    fr: "Rogner",
    hr: "Obreži",
    hu: "Vágás",
    id: "Pangkas",
    nl: "Bijsnijden",
    pt: "Cortar",
    ru: "Обрезка",
};

/// The timeline's own status line, and the one place a hint about dragging
/// belongs -- "整段" said nothing at all, and the control it sits under was
/// being read as a read-only preview.
pub static TRIM_NONE: Text = Text {
    en: "Whole video — drag either end to trim",
    zh_hans: "整段保留，拖两端可裁剪",
    zh_hant: "整段保留，拖兩端可裁剪",
    ja: "動画全体 — 端をドラッグすると切り取れます",
    ko: "전체 영상 — 양 끝을 끌어 자를 수 있습니다",
    de: "Ganzes Video — zum Kürzen an den Enden ziehen",
    es: "Video completo: arrastra los extremos para recortar",
    fr: "Vidéo entière — glissez les extrémités pour découper",
    hr: "Cijeli video — povucite krajeve za rezanje",
    hu: "Teljes videó — húzd a végeit a vágáshoz",
    id: "Seluruh video — seret ujungnya untuk memotong",
    nl: "Hele video — sleep de uiteinden om bij te snijden",
    pt: "Vídeo inteiro — arraste as pontas para cortar",
    ru: "Всё видео — потяните за края, чтобы обрезать",
};

pub static TRIM_TAIL_ONLY: Text = Text {
    en: "Keeping {kept}, dropping {cut}",
    zh_hans: "保留 {kept}，去掉 {cut}",
    zh_hant: "保留 {kept}，去掉 {cut}",
    ja: "{kept} を残し、{cut} を削ります",
    ko: "{kept}를 남기고 {cut}를 버립니다",
    de: "{kept} bleiben, {cut} fallen weg",
    es: "Se conserva {kept}, se descarta {cut}",
    fr: "On garde {kept}, on retire {cut}",
    hr: "Zadržava se {kept}, odbacuje {cut}",
    hu: "{kept} marad, {cut} kimarad",
    id: "Menyimpan {kept}, membuang {cut}",
    nl: "{kept} blijft, {cut} vervalt",
    pt: "Mantendo {kept}, descartando {cut}",
    ru: "Останется {kept}, отброшено {cut}",
};

/// Moving the START is the one action here with a consequence the charter cannot
/// see from the picture, so it is spelled out rather than implied.
pub static TRIM_HEAD_WARNING: Text = Text {
    en: "The video will start {shift} earlier against the music. Only do this if \
         its opening is not meant to line up with the song.",
    zh_hans: "画面会相对音乐整体提前 {shift}。只有当开头本来就不该和歌曲对齐时才这么做。",
    zh_hant: "畫面會相對音樂整體提前 {shift}。只有當開頭本來就不該和歌曲對齊時才這麼做。",
    ja: "映像が音楽に対して {shift} 早く始まるようになります。冒頭を曲に合わせる必要がない場合だけにしてください。",
    ko: "영상이 음악보다 {shift} 앞당겨집니다. 도입부를 곡에 맞출 필요가 없을 때만 사용하세요.",
    de: "Das Video beginnt {shift} früher als die Musik. Nur tun, wenn der Anfang \
         nicht zum Song passen soll.",
    es: "El video empezará {shift} antes que la música. Hazlo solo si su comienzo \
         no debe coincidir con la canción.",
    fr: "La vidéo commencera {shift} plus tôt que la musique. À ne faire que si son \
         début n'est pas censé coïncider avec le morceau.",
    hr: "Video će početi {shift} ranije od glazbe. Učinite to samo ako se početak \
         ne treba poklapati s pjesmom.",
    hu: "A videó {shift} idővel korábban indul a zenéhez képest. Csak akkor tedd, \
         ha az eleje nincs a dalhoz igazítva.",
    id: "Video akan mulai {shift} lebih awal dari musiknya. Lakukan hanya jika \
         pembukaannya memang tidak perlu selaras dengan lagu.",
    nl: "De video begint {shift} eerder dan de muziek. Doe dit alleen als het begin \
         niet op het nummer hoeft aan te sluiten.",
    pt: "O vídeo começará {shift} antes da música. Só faça isso se a abertura não \
         precisar coincidir com a canção.",
    ru: "Видео начнётся на {shift} раньше музыки. Делайте это, только если начало \
         и не должно совпадать с песней.",
};

pub static TRIM_HEAD_COSTS_REENCODE: Text = Text {
    en: "Cutting the opening also means re-encoding, which takes longer than a \
         plain trim.",
    zh_hans: "裁开头会连带触发重编码，比单纯裁尾巴慢。",
    zh_hant: "裁開頭會連帶觸發重編碼，比單純裁尾巴慢。",
    ja: "冒頭を切ると再エンコードも必要になり、末尾だけを切るより時間がかかります。",
    ko: "도입부를 자르면 재인코딩도 필요해서 끝만 자를 때보다 오래 걸립니다.",
    de: "Den Anfang zu schneiden erfordert auch ein Neukodieren und dauert länger \
         als ein einfaches Kürzen.",
    es: "Cortar el comienzo obliga a recodificar, y eso tarda más que un recorte simple.",
    fr: "Couper le début impose un réencodage, plus long qu'un simple rognage.",
    hr: "Rezanje početka zahtijeva i prekodiranje, što traje dulje od običnog rezanja.",
    hu: "Az eleje levágása újrakódolással jár, ami tovább tart, mint egy egyszerű vágás.",
    id: "Memotong pembukaan berarti mengodekan ulang, dan itu lebih lama daripada \
         pemangkasan biasa.",
    nl: "Het begin wegsnijden vereist opnieuw coderen, wat langer duurt dan gewoon inkorten.",
    pt: "Cortar a abertura exige recodificar, o que demora mais que um corte simples.",
    ru: "Обрезка начала требует перекодирования и занимает больше времени, чем обычная обрезка.",
};

pub static TRIM_RESET: Text = Text {
    en: "Reset",
    zh_hans: "还原",
    zh_hant: "還原",
    ja: "元に戻す",
    ko: "되돌리기",
    de: "Zurücksetzen",
    es: "Restablecer",
    fr: "Réinitialiser",
    hr: "Poništi",
    hu: "Visszaállítás",
    id: "Atur ulang",
    nl: "Herstellen",
    pt: "Redefinir",
    ru: "Сбросить",
};

// ── doing it ─────────────────────────────────────────────────────────────

pub static START: Text = Text {
    en: "Make it uploadable",
    zh_hans: "开始处理",
    zh_hant: "開始處理",
    ja: "処理する",
    ko: "처리 시작",
    de: "Hochladbar machen",
    es: "Prepararlo para subir",
    fr: "Rendre la vidéo envoyable",
    hr: "Pripremi za prijenos",
    hu: "Tedd feltölthetővé",
    id: "Siapkan untuk diunggah",
    nl: "Maak uploadbaar",
    pt: "Preparar para envio",
    ru: "Подготовить к загрузке",
};

pub static STEP_COPYING: Text = Text {
    en: "Fixing it losslessly…",
    zh_hans: "无损修复中…",
    zh_hant: "無損修復中…",
    ja: "劣化なしで修正中…",
    ko: "무손실로 수정하는 중…",
    de: "Verlustfrei wird korrigiert…",
    es: "Arreglándolo sin pérdida…",
    fr: "Correction sans perte…",
    hr: "Popravlja se bez gubitka…",
    hu: "Javítás veszteség nélkül…",
    id: "Memperbaiki tanpa kehilangan kualitas…",
    nl: "Verliesvrij aan het herstellen…",
    pt: "Corrigindo sem perdas…",
    ru: "Исправление без потерь…",
};

/// Just what is happening.
///
/// It read "Trying AV1 — it is kept only if it comes out smaller (26%)". That
/// clause is the candidate-selection rule, which is Press's business and nobody
/// else's; somebody watching a progress bar wants to know what is running, how
/// far along it is and how big it is getting. The percentage went too -- the bar
/// underneath is the percentage.
pub static STEP_ENCODING: Text = Text {
    en: "Converting to AV1…",
    zh_hans: "正在转成 AV1…",
    zh_hant: "正在轉成 AV1…",
    ja: "AV1 に変換中…",
    ko: "AV1으로 변환하는 중…",
    de: "Wird nach AV1 konvertiert…",
    es: "Convirtiendo a AV1…",
    fr: "Conversion en AV1…",
    hr: "Pretvaranje u AV1…",
    hu: "Átalakítás AV1-re…",
    id: "Mengubah ke AV1…",
    nl: "Wordt omgezet naar AV1…",
    pt: "Convertendo para AV1…",
    ru: "Преобразование в AV1…",
};

/// Appended from the second attempt onward.
///
/// Without it a search that needs three encodes fills the bar, resets it, and
/// shows the identical sentence again -- which from outside is indistinguishable
/// from a loop. Suppressed on the first attempt, where it would be noise: its
/// APPEARANCE is the signal.
pub static STEP_ATTEMPT: Text = Text {
    en: "attempt {n}",
    zh_hans: "第 {n} 次",
    zh_hant: "第 {n} 次",
    ja: "{n} 回目",
    ko: "{n}번째",
    de: "Versuch {n}",
    es: "intento {n}",
    fr: "essai {n}",
    hr: "pokušaj {n}",
    hu: "{n}. próba",
    id: "percobaan ke-{n}",
    nl: "poging {n}",
    pt: "tentativa {n}",
    ru: "попытка {n}",
};

/// The quality being used right now. It is why a second attempt exists at all,
/// and there is no knob for it, so the least Press can do is say it.
pub static STEP_QUALITY: Text = Text {
    en: "quality CRF {crf}",
    zh_hans: "画质 CRF {crf}",
    zh_hant: "畫質 CRF {crf}",
    ja: "画質 CRF {crf}",
    ko: "화질 CRF {crf}",
    de: "Qualität CRF {crf}",
    es: "calidad CRF {crf}",
    fr: "qualité CRF {crf}",
    hr: "kvaliteta CRF {crf}",
    hu: "minőség CRF {crf}",
    id: "kualitas CRF {crf}",
    nl: "kwaliteit CRF {crf}",
    pt: "qualidade CRF {crf}",
    ru: "качество CRF {crf}",
};

pub static STEP_VERIFYING: Text = Text {
    en: "Checking the result…",
    zh_hans: "正在校验结果…",
    zh_hant: "正在校驗結果…",
    ja: "結果を確認中…",
    ko: "결과를 확인하는 중…",
    de: "Ergebnis wird geprüft…",
    es: "Comprobando el resultado…",
    fr: "Vérification du résultat…",
    hr: "Provjera rezultata…",
    hu: "Az eredmény ellenőrzése…",
    id: "Memeriksa hasil…",
    nl: "Resultaat wordt gecontroleerd…",
    pt: "Verificando o resultado…",
    ru: "Проверка результата…",
};

// ── the result ───────────────────────────────────────────────────────────

pub static CANCELLED: Text = Text {
    en: "Stopped. Nothing was written.",
    zh_hans: "已停止，没有写出任何文件。",
    zh_hant: "已停止，沒有寫出任何檔案。",
    ja: "中断しました。ファイルは作成されていません。",
    ko: "중단했습니다. 파일은 만들어지지 않았습니다.",
    de: "Angehalten. Es wurde nichts geschrieben.",
    es: "Detenido. No se escribió nada.",
    fr: "Arrêté. Rien n'a été écrit.",
    hr: "Zaustavljeno. Ništa nije zapisano.",
    hu: "Leállítva. Semmi sem íródott ki.",
    id: "Dihentikan. Tidak ada yang ditulis.",
    nl: "Gestopt. Er is niets weggeschreven.",
    pt: "Interrompido. Nada foi gravado.",
    ru: "Остановлено. Ничего не записано.",
};

pub static FAILED: Text = Text {
    en: "This one did not work out",
    zh_hans: "这个文件没能处理成功",
    zh_hant: "這個檔案沒能處理成功",
    ja: "このファイルは処理できませんでした",
    ko: "이 파일은 처리하지 못했습니다",
    de: "Bei dieser Datei hat es nicht geklappt",
    es: "Con este archivo no funcionó",
    fr: "Ça n'a pas fonctionné pour ce fichier",
    hr: "S ovom datotekom nije uspjelo",
    hu: "Ezzel a fájllal nem sikerült",
    id: "Yang ini tidak berhasil",
    nl: "Bij dit bestand is het niet gelukt",
    pt: "Com este arquivo não deu certo",
    ru: "С этим файлом не получилось",
};

// ── Phase A: the deterministic screen ──────────────────────────────────

/// The other half of [`VERDICT_WILL_FIX`], worded to match it.
///
/// Size is deliberately not mentioned -- it is not settled by looking at the
/// header, and claiming anything about it here would promise an answer Press
/// does not have yet. "第一层检查没发现问题" said so by naming the phase, which
/// is a word from the source code rather than from the charter's situation.
/// THE HANDOFF, and it is the whole reason this line is worded the way it is.
///
/// This screen deliberately does not judge size -- that is not settled by
/// reading a header. So a file can pass here and still be refused on the next
/// screen, and when this said merely "nothing needs changing" that sequence read
/// as the window contradicting itself: 「你刚说没什么要改，怎么又说不能传」.
///
/// Naming what has NOT been looked at yet costs four words and removes the
/// contradiction entirely, because the two screens are then plainly about two
/// different things.
pub static PHASE_A_CLEAR: Text = Text {
    en: "Nothing to change — size comes next",
    zh_hans: "没有要改的，接着看体积",
    zh_hant: "沒有要改的，接著看體積",
    ja: "変更は不要です。次はサイズを見ます",
    ko: "고칠 것은 없습니다 — 다음은 용량입니다",
    de: "Nichts zu ändern — als Nächstes die Größe",
    es: "Nada que cambiar: ahora toca el tamaño",
    fr: "Rien à changer — reste la taille",
    hr: "Ništa za promijeniti — slijedi veličina",
    hu: "Nincs mit módosítani — jön a méret",
    id: "Tidak ada yang perlu diubah — berikutnya ukuran",
    nl: "Niets te veranderen — nu de grootte",
    pt: "Nada a mudar — falta o tamanho",
    ru: "Менять нечего — дальше размер",
};

pub static PHASE_A_CLEAR_HINT: Text = Text {
    en: "Trim it if you want to, then carry on.",
    zh_hans: "想裁就裁，然后继续下一步。",
    zh_hant: "想裁就裁，然後繼續下一步。",
    ja: "必要ならトリミングしてから次へ進んでください。",
    ko: "원하면 자른 뒤 다음 단계로 넘어가세요.",
    de: "Schneide es zu, wenn du magst, und mach dann weiter.",
    es: "Recórtalo si quieres y sigue adelante.",
    fr: "Découpez-la si vous voulez, puis continuez.",
    hr: "Skratite ga ako želite, pa nastavite.",
    hu: "Vágd meg, ha szeretnéd, aztán folytasd.",
    id: "Potong kalau mau, lalu lanjutkan.",
    nl: "Snijd het bij als je wilt en ga dan verder.",
    pt: "Corte se quiser e siga em frente.",
    ru: "Обрежьте, если хотите, и продолжайте.",
};

// ── the bench, between the phases ──────────────────────────────────────

pub static BENCH_READY: Text = Text {
    en: "Ready to upload",
    zh_hans: "可以直接上传了",
    zh_hant: "可以直接上傳了",
    ja: "そのままアップロードできます",
    ko: "바로 업로드할 수 있습니다",
    de: "Bereit zum Hochladen",
    es: "Listo para subir",
    fr: "Prêt à être envoyé",
    hr: "Spremno za slanje",
    hu: "Feltöltésre kész",
    id: "Siap diunggah",
    nl: "Klaar om te uploaden",
    pt: "Pronto para enviar",
    ru: "Готово к загрузке",
};

pub static BENCH_REMAINING: Text = Text {
    en: "One more pass needed",
    zh_hans: "还需要再处理一次",
    zh_hant: "還需要再處理一次",
    ja: "もう一度処理が必要です",
    ko: "한 번 더 처리해야 합니다",
    de: "Ein weiterer Durchgang nötig",
    es: "Hace falta una pasada más",
    fr: "Une passe de plus est nécessaire",
    hr: "Potreban je još jedan prolaz",
    hu: "Még egy menet kell",
    id: "Perlu satu proses lagi",
    nl: "Nog één ronde nodig",
    pt: "Falta mais uma passagem",
    ru: "Нужен ещё один проход",
};

// ── the bench, in four lines ─────────────────────────────────────────────
//
// A headline that says WHAT rather than merely THAT, one short line under it,
// one line of what pressing the button will do, and the file's specification.
//
// The version this replaces labelled three bands -- "just done", the verdict,
// "next" -- and it read as a ledger. "刚才做了 / 什么都不用做" is a label
// contradicting its own content; "还需要再处理一次" is a headline that leaves
// the reader asking what needs processing. Labels cannot rescue a headline that
// does not say anything.

/// Joins items in a sentence, per language.
///
/// The interpunct that separates FIELDS -- `av1 · 1080×816 · 24 fps` -- is not
/// what joins CLAUSES: "Processing will scale it to 2956×1246 · get it under
/// 80 MB" is not a sentence anyone writes. Latin scripts take a comma, CJK
/// takes its own enumeration mark.
pub static LIST_SEP: Text = Text {
    en: ", ",
    zh_hans: "，",
    zh_hant: "，",
    ja: "、",
    ko: ", ",
    de: ", ",
    es: ", ",
    fr: ", ",
    hr: ", ",
    hu: ", ",
    id: ", ",
    nl: ", ",
    pt: ", ",
    ru: ", ",
};

/// The inspect screen's button.
///
/// NOT "Start". What it starts is a stream copy that takes a second, and giving
/// it the same word as the bench's button -- which starts a transcode that can
/// take minutes -- put one label on two very different acts. From the charter's
/// side this press is simply carrying on.
pub static NEXT_STEP: Text = Text {
    en: "Next",
    zh_hans: "下一步",
    zh_hant: "下一步",
    ja: "次へ",
    ko: "다음",
    de: "Weiter",
    es: "Siguiente",
    fr: "Suivant",
    hr: "Dalje",
    hu: "Tovább",
    id: "Lanjut",
    nl: "Volgende",
    pt: "Avançar",
    ru: "Далее",
};

/// The headline when the file cannot be uploaded yet. Paired with a short list
/// of what is wrong, because "one more pass needed" is not an answer.
pub static NOT_READY: Text = Text {
    en: "Not ready yet",
    zh_hans: "还不能上传",
    zh_hant: "還不能上傳",
    ja: "まだアップロードできません",
    ko: "아직 업로드할 수 없습니다",
    de: "Noch nicht bereit",
    es: "Todavía no está listo",
    fr: "Pas encore prêt",
    hr: "Još nije spremno",
    hu: "Még nem kész",
    id: "Belum siap",
    nl: "Nog niet klaar",
    pt: "Ainda não está pronto",
    ru: "Пока не готово",
};

/// The whole sentence, with `{steps}` where the list goes.
///
/// A template rather than a prefix, because `format!("{lead} {steps}")` puts a
/// space between them and Chinese does not take one: "处理后会 压到 80 MB 以内".
/// Each language decides its own join.
pub static WILL_LEAD: Text = Text {
    en: "Processing will {steps}",
    zh_hans: "处理后会{steps}",
    zh_hant: "處理後會{steps}",
    ja: "処理すると{steps}",
    ko: "처리하면 {steps}",
    de: "Die Verarbeitung wird {steps}",
    es: "El procesado va a {steps}",
    fr: "Le traitement va {steps}",
    hr: "Obrada će {steps}",
    hu: "A feldolgozás: {steps}",
    id: "Pemrosesan akan {steps}",
    nl: "De bewerking gaat {steps}",
    pt: "O processamento vai {steps}",
    ru: "Обработка: {steps}",
};

// What is wrong, in as few words as carry the meaning. The numbers are on the
// specification line directly below, so repeating them here -- "5120×2160,
// over the pixel budget" above a line that says "5120×2160" -- is one fact
// printed twice on one card.

pub static SHORT_RESOLUTION: Text = Text {
    en: "Resolution too high",
    zh_hans: "分辨率太高",
    zh_hant: "解析度太高",
    ja: "解像度が高すぎます",
    ko: "해상도가 너무 높음",
    de: "Auflösung zu hoch",
    es: "Resolución demasiado alta",
    fr: "Résolution trop élevée",
    hr: "Prevelika razlučivost",
    hu: "Túl nagy felbontás",
    id: "Resolusi terlalu tinggi",
    nl: "Resolutie te hoog",
    pt: "Resolução alta demais",
    ru: "Слишком высокое разрешение",
};

pub static SHORT_FPS: Text = Text {
    en: "Frame rate too high",
    zh_hans: "帧率太高",
    zh_hant: "影格率太高",
    ja: "フレームレートが高すぎます",
    ko: "프레임 레이트가 너무 높음",
    de: "Bildrate zu hoch",
    es: "Tasa de fotogramas demasiado alta",
    fr: "Fréquence d'images trop élevée",
    hr: "Previsoka brzina sličica",
    hu: "Túl nagy képfrissítés",
    id: "Laju bingkai terlalu tinggi",
    nl: "Beeldsnelheid te hoog",
    pt: "Taxa de quadros alta demais",
    ru: "Слишком высокая частота кадров",
};

pub static SHORT_CODEC: Text = Text {
    en: "Codec not accepted",
    zh_hans: "编码不支持",
    zh_hant: "編碼不支援",
    ja: "対応していないコーデック",
    ko: "지원하지 않는 코덱",
    de: "Codec nicht zugelassen",
    es: "Códec no admitido",
    fr: "Codec non accepté",
    hr: "Kodek nije prihvaćen",
    hu: "Nem elfogadott kodek",
    id: "Kodek tidak diterima",
    nl: "Codec niet toegestaan",
    pt: "Codec não aceito",
    ru: "Кодек не принимается",
};

pub static SHORT_SIZE: Text = Text {
    en: "File too large",
    zh_hans: "文件太大",
    zh_hant: "檔案太大",
    ja: "ファイルが大きすぎます",
    ko: "파일이 너무 큼",
    de: "Datei zu groß",
    es: "Archivo demasiado grande",
    fr: "Fichier trop volumineux",
    hr: "Datoteka prevelika",
    hu: "Túl nagy fájl",
    id: "Berkas terlalu besar",
    nl: "Bestand te groot",
    pt: "Arquivo grande demais",
    ru: "Файл слишком большой",
};

// ── the crop, which is an offer and never a finding ────────────────────

/// The nudge, shown only when the source is neither shape players are usually
/// on. Worded as an observation, not a fault: the gate checks a pixel COUNT and
/// has no opinion about shape, so a 4:3 background video is accepted and will
/// play -- it simply will not fill the screen.
pub static CROP_OFFER: Text = Text {
    en: "Players are usually on 16:9 or 16:10 — this fills neither",
    zh_hans: "玩家屏幕通常是 16:9 或 16:10，这个都填不满",
    zh_hant: "玩家螢幕通常是 16:9 或 16:10，這個都填不滿",
    ja: "プレイヤーの画面はたいてい 16:9 か 16:10 で、これはどちらも埋めません",
    ko: "플레이어 화면은 보통 16:9나 16:10이고, 이 영상은 어느 쪽도 채우지 못합니다",
    de: "Spieler sind meist auf 16:9 oder 16:10 — das füllt keines von beiden",
    es: "Los jugadores suelen usar 16:9 o 16:10; esto no llena ninguno",
    fr: "Les joueurs sont le plus souvent en 16:9 ou 16:10 — ceci ne remplit ni l'un ni l'autre",
    hr: "Igrači su obično na 16:9 ili 16:10 — ovo ne ispunjava nijedan",
    hu: "A játékosok általában 16:9-en vagy 16:10-en vannak — ez egyiket sem tölti ki",
    id: "Layar pemain biasanya 16:9 atau 16:10 — ini tidak memenuhi keduanya",
    nl: "Spelers zitten meestal op 16:9 of 16:10 — dit vult geen van beide",
    pt: "Os jogadores costumam usar 16:9 ou 16:10 — isto não preenche nenhum",
    ru: "У игроков обычно 16:9 или 16:10 — это не заполняет ни то, ни другое",
};

/// A crop option, with what it costs. The cost is the whole decision: from a
/// 4:3 source, 16:9 gives up a quarter of the picture and 16:10 gives up
/// noticeably less.
pub static CROP_OPTION: Text = Text {
    en: "{ratio} · loses {percent}%",
    zh_hans: "{ratio} · 切掉 {percent}%",
    zh_hant: "{ratio} · 切掉 {percent}%",
    ja: "{ratio} · {percent}% を切る",
    ko: "{ratio} · {percent}% 잘림",
    de: "{ratio} · verliert {percent}%",
    es: "{ratio} · pierde {percent}%",
    fr: "{ratio} · perd {percent}%",
    hr: "{ratio} · gubi {percent}%",
    hu: "{ratio} · {percent}%-ot veszít",
    id: "{ratio} · kehilangan {percent}%",
    nl: "{ratio} · verliest {percent}%",
    pt: "{ratio} · perde {percent}%",
    ru: "{ratio} · теряет {percent}%",
};

/// Turning the crop off again.
pub static CROP_NONE: Text = Text {
    en: "Keep it all",
    zh_hans: "都不裁",
    zh_hant: "都不裁",
    ja: "切らない",
    ko: "자르지 않음",
    de: "Alles behalten",
    es: "No recortar",
    fr: "Tout garder",
    hr: "Zadrži sve",
    hu: "Maradjon mind",
    id: "Simpan semuanya",
    nl: "Alles houden",
    pt: "Manter tudo",
    ru: "Оставить всё",
};

/// Only shown once a crop is chosen, because only then is there something to
/// drag.
pub static CROP_DRAG: Text = Text {
    en: "Drag the picture to choose what to keep",
    zh_hans: "拖动画面选择保留哪一块",
    zh_hant: "拖動畫面選擇保留哪一塊",
    ja: "映像をドラッグして残す範囲を選べます",
    ko: "화면을 끌어 남길 부분을 고르세요",
    de: "Zieh am Bild, um den Ausschnitt zu wählen",
    es: "Arrastra la imagen para elegir qué conservar",
    fr: "Faites glisser l'image pour choisir ce qui reste",
    hr: "Povucite sliku da odaberete što ostaje",
    hu: "Húzd a képet, hogy kiválaszd a megtartott részt",
    id: "Seret gambar untuk memilih bagian yang disimpan",
    nl: "Sleep het beeld om te kiezen wat blijft",
    pt: "Arraste a imagem para escolher o que fica",
    ru: "Потяните изображение, чтобы выбрать оставляемую часть",
};

pub static SHORT_CROP: Text = Text {
    en: "Crop requested",
    zh_hans: "要裁画面",
    zh_hant: "要裁畫面",
    ja: "切り抜きの指定あり",
    ko: "자르기 요청됨",
    de: "Zuschnitt gewünscht",
    es: "Recorte solicitado",
    fr: "Recadrage demandé",
    hr: "Zatraženo izrezivanje",
    hu: "Kért vágás",
    id: "Pemotongan diminta",
    nl: "Bijsnijden gevraagd",
    pt: "Recorte solicitado",
    ru: "Запрошена обрезка",
};

pub static WILL_CROP: Text = Text {
    en: "crop it to {width}×{height}",
    zh_hans: "裁成 {width}×{height}",
    zh_hant: "裁成 {width}×{height}",
    ja: "{width}×{height} に切り抜く",
    ko: "{width}×{height}로 자름",
    de: "auf {width}×{height} zuschneiden",
    es: "recortarlo a {width}×{height}",
    fr: "la recadrer en {width}×{height}",
    hr: "izrezati na {width}×{height}",
    hu: "{width}×{height} méretre vágni",
    id: "memotongnya ke {width}×{height}",
    nl: "bijsnijden naar {width}×{height}",
    pt: "recortá-lo para {width}×{height}",
    ru: "обрезать до {width}×{height}",
};

pub static SHORT_OPENING: Text = Text {
    en: "Opening to cut",
    zh_hans: "开头要裁掉",
    zh_hant: "開頭要裁掉",
    ja: "冒頭のカットが必要",
    ko: "앞부분을 잘라야 함",
    de: "Anfang abzuschneiden",
    es: "Hay que recortar el inicio",
    fr: "Début à couper",
    hr: "Početak treba odrezati",
    hu: "Az elejét le kell vágni",
    id: "Awalan perlu dipotong",
    nl: "Begin moet eraf",
    pt: "Início a cortar",
    ru: "Начало нужно обрезать",
};

// What the next press will do. Bare verbs, because they are joined after
// `WILL_LEAD` -- the `FIX_*` set carries its own "will" and reads as a
// stammer once it has one in front of it.

pub static WILL_SCALE: Text = Text {
    en: "scale it to {width}×{height}",
    zh_hans: "缩到 {width}×{height}",
    zh_hant: "縮到 {width}×{height}",
    ja: "{width}×{height} に縮小",
    ko: "{width}×{height}로 축소",
    de: "auf {width}×{height} skalieren",
    es: "escalarlo a {width}×{height}",
    fr: "le redimensionner en {width}×{height}",
    hr: "smanjiti na {width}×{height}",
    hu: "{width}×{height} méretre kicsinyíteni",
    id: "memperkecil ke {width}×{height}",
    nl: "schalen naar {width}×{height}",
    pt: "redimensionar para {width}×{height}",
    ru: "уменьшить до {width}×{height}",
};

pub static WILL_FPS: Text = Text {
    en: "bring it down to {limit} fps",
    zh_hans: "降到 {limit} fps",
    zh_hant: "降到 {limit} fps",
    ja: "{limit} fps に下げる",
    ko: "{limit} fps로 낮춤",
    de: "auf {limit} fps reduzieren",
    es: "reducirlo a {limit} fps",
    fr: "la ramener à {limit} fps",
    hr: "smanjiti na {limit} fps",
    hu: "{limit} fps-re csökkenteni",
    id: "menurunkan ke {limit} fps",
    nl: "terugbrengen naar {limit} fps",
    pt: "reduzir para {limit} fps",
    ru: "снизить до {limit} fps",
};

pub static WILL_CODEC: Text = Text {
    en: "convert it to AV1",
    zh_hans: "转成 AV1",
    zh_hant: "轉成 AV1",
    ja: "AV1 に変換",
    ko: "AV1으로 변환",
    de: "nach AV1 konvertieren",
    es: "convertirlo a AV1",
    fr: "la convertir en AV1",
    hr: "pretvoriti u AV1",
    hu: "AV1-re alakítani",
    id: "mengubahnya ke AV1",
    nl: "omzetten naar AV1",
    pt: "convertê-lo para AV1",
    ru: "преобразовать в AV1",
};

pub static WILL_SIZE: Text = Text {
    en: "get it under {limit} MB",
    zh_hans: "压到 {limit} MB 以内",
    zh_hant: "壓到 {limit} MB 以內",
    ja: "{limit} MB 以内に収める",
    ko: "{limit} MB 이내로 줄임",
    de: "unter {limit} MB bringen",
    es: "dejarlo por debajo de {limit} MB",
    fr: "la faire passer sous {limit} Mo",
    hr: "spustiti ispod {limit} MB",
    hu: "{limit} MB alá vinni",
    id: "membuatnya di bawah {limit} MB",
    nl: "onder {limit} MB brengen",
    pt: "deixá-lo abaixo de {limit} MB",
    ru: "уместить в {limit} МБ",
};

pub static WILL_OPENING: Text = Text {
    en: "cut {shift} off the start",
    zh_hans: "裁掉开头 {shift}",
    zh_hant: "裁掉開頭 {shift}",
    ja: "冒頭 {shift} をカット",
    ko: "앞부분 {shift}을 자름",
    de: "{shift} vom Anfang abschneiden",
    es: "cortar {shift} del inicio",
    fr: "couper {shift} au début",
    hr: "odrezati {shift} s početka",
    hu: "levágni az elejéről {shift}-t",
    id: "memotong {shift} dari awal",
    nl: "{shift} van het begin afhalen",
    pt: "cortar {shift} do início",
    ru: "срезать {shift} с начала",
};

/// The headline after an encode has just produced something.
///
/// Separate from [`BENCH_READY`] on purpose. The state after a successful encode
/// reads the same as the state before it -- "ready to upload" was true both
/// times -- so returning to a screen whose headline had not changed was
/// indistinguishable from nothing having happened.
pub static BENCH_COMPRESSED: Text = Text {
    en: "Compressed",
    zh_hans: "压好了",
    zh_hant: "壓好了",
    ja: "圧縮できました",
    ko: "압축했습니다",
    de: "Komprimiert",
    es: "Comprimido",
    fr: "Compressée",
    hr: "Sažeto",
    hu: "Tömörítve",
    id: "Terkompresi",
    nl: "Gecomprimeerd",
    pt: "Comprimido",
    ru: "Сжато",
};

/// A number that was not on the previous screen, which is the other half of
/// making this one read as a different screen.
pub static BENCH_SMALLER: Text = Text {
    en: "{percent}% smaller",
    zh_hans: "小了 {percent}%",
    zh_hant: "小了 {percent}%",
    ja: "{percent}% 小さく",
    ko: "{percent}% 작아짐",
    de: "{percent}% kleiner",
    es: "{percent}% más pequeño",
    fr: "{percent}% plus petit",
    hr: "{percent}% manje",
    hu: "{percent}%-kal kisebb",
    id: "{percent}% lebih kecil",
    nl: "{percent}% kleiner",
    pt: "{percent}% menor",
    ru: "меньше на {percent}%",
};

pub static BENCH_ALLOWANCE: Text = Text {
    en: "{percent}% of the {limit} allowed",
    zh_hans: "占上限 {limit} 的 {percent}%",
    zh_hant: "佔上限 {limit} 的 {percent}%",
    ja: "上限 {limit} の {percent}%",
    ko: "제한 {limit}의 {percent}%",
    de: "{percent}% der erlaubten {limit}",
    es: "{percent}% de los {limit} permitidos",
    fr: "{percent}% des {limit} autorisés",
    hr: "{percent}% od dopuštenih {limit}",
    hu: "a megengedett {limit} {percent}%-a",
    id: "{percent}% dari {limit} yang diizinkan",
    nl: "{percent}% van de toegestane {limit}",
    pt: "{percent}% dos {limit} permitidos",
    ru: "{percent}% от разрешённых {limit}",
};

pub static DIFF_AUDIO: Text = Text {
    en: "Audio track removed",
    zh_hans: "去掉了音轨",
    zh_hant: "去掉了音軌",
    ja: "音声トラックを削除しました",
    ko: "오디오 트랙을 제거했습니다",
    de: "Tonspur entfernt",
    es: "Pista de audio eliminada",
    fr: "Piste audio supprimée",
    hr: "Zvučni zapis uklonjen",
    hu: "Hangsáv eltávolítva",
    id: "Trek audio dihapus",
    nl: "Audiospoor verwijderd",
    pt: "Faixa de áudio removida",
    ru: "Звуковая дорожка удалена",
};

pub static DIFF_TAIL: Text = Text {
    en: "Tail cut: {was} → {now}",
    zh_hans: "裁掉了尾巴：{was} → {now}",
    zh_hant: "裁掉了尾巴：{was} → {now}",
    ja: "末尾をカット：{was} → {now}",
    ko: "뒷부분을 잘랐습니다: {was} → {now}",
    de: "Ende gekürzt: {was} → {now}",
    es: "Cola recortada: {was} → {now}",
    fr: "Fin coupée : {was} → {now}",
    hr: "Rep odrezan: {was} → {now}",
    hu: "Vége levágva: {was} → {now}",
    id: "Ekor dipotong: {was} → {now}",
    nl: "Staart afgeknipt: {was} → {now}",
    pt: "Cauda cortada: {was} → {now}",
    ru: "Хвост обрезан: {was} → {now}",
};

/// Sits under "ready to upload" when the file arrived that way. Short, because
/// it is good news and good news does not need a paragraph.
pub static DIFF_NOTHING: Text = Text {
    en: "It already met every rule",
    zh_hans: "本来就符合要求",
    zh_hant: "本來就符合要求",
    ja: "もともと条件を満たしていました",
    ko: "원래부터 조건을 만족했습니다",
    de: "Sie erfüllte bereits alle Regeln",
    es: "Ya cumplía todas las reglas",
    fr: "Elle respectait déjà toutes les règles",
    hr: "Već je zadovoljavala sva pravila",
    hu: "Eleve megfelelt minden szabálynak",
    id: "Sudah memenuhi semua aturan",
    nl: "Het voldeed al aan alle regels",
    pt: "Já cumpria todas as regras",
    ru: "Оно уже отвечало всем требованиям",
};

pub static FIX_SIZE: Text = Text {
    en: "compressed to fit under {limit} MB",
    zh_hans: "会压到 {limit} MB 以内",
    zh_hant: "會壓到 {limit} MB 以內",
    ja: "{limit} MB 以内に圧縮します",
    ko: "{limit} MB 이내로 압축합니다",
    de: "wird unter {limit} MB gebracht",
    es: "se comprimirá por debajo de {limit} MB",
    fr: "sera compressée sous {limit} Mo",
    hr: "bit će sažeto ispod {limit} MB",
    hu: "{limit} MB alá tömörítjük",
    id: "akan dikompresi di bawah {limit} MB",
    nl: "wordt gecomprimeerd tot onder {limit} MB",
    pt: "será comprimido para menos de {limit} MB",
    ru: "будет сжато до {limit} МБ",
};

/// One line, with the number that decided it. The paragraph this replaces spent
/// three clauses explaining a choice the reader can check at a glance.
pub static SAID_MATERIAL_WINS: Text = Text {
    en: "AV1 came out at {size}, so the original was kept",
    zh_hans: "转 AV1 反而有 {size}，所以保留了原编码",
    zh_hant: "轉 AV1 反而有 {size}，所以保留了原編碼",
    ja: "AV1 では {size} になったため、元のまま残しました",
    ko: "AV1은 {size}가 되어 원본을 유지했습니다",
    de: "AV1 kam auf {size}, also blieb das Original",
    es: "AV1 salió con {size}, así que se conservó el original",
    fr: "L'AV1 faisait {size}, l'original a donc été conservé",
    hr: "AV1 je ispao {size}, pa je zadržan izvornik",
    hu: "Az AV1 {size} lett, így maradt az eredeti",
    id: "AV1 menjadi {size}, jadi aslinya dipertahankan",
    nl: "AV1 werd {size}, dus is het origineel behouden",
    pt: "O AV1 ficou com {size}, então o original foi mantido",
    ru: "AV1 вышел на {size}, поэтому остался оригинал",
};

/// Shown while the search throws away one quality and starts another.
pub static STEP_RECALIBRATING: Text = Text {
    en: "Changing quality: CRF {from} → {to}",
    zh_hans: "换个画质重来：CRF {from} → {to}",
    zh_hant: "換個畫質重來：CRF {from} → {to}",
    ja: "画質を変えてやり直します：CRF {from} → {to}",
    ko: "화질을 바꿔 다시 시도합니다: CRF {from} → {to}",
    de: "Qualität wird gewechselt: CRF {from} → {to}",
    es: "Cambiando la calidad: CRF {from} → {to}",
    fr: "Changement de qualité : CRF {from} → {to}",
    hr: "Mijenjam kvalitetu: CRF {from} → {to}",
    hu: "Minőségváltás: CRF {from} → {to}",
    id: "Mengubah kualitas: CRF {from} → {to}",
    nl: "Kwaliteit wordt gewijzigd: CRF {from} → {to}",
    pt: "Mudando a qualidade: CRF {from} → {to}",
    ru: "Меняю качество: CRF {from} → {to}",
};

/// Stop a run. Always safe now: Phase A's material exists before Phase B ever
/// starts, so there is never nothing to go back to.
pub static STOP: Text = Text {
    en: "Stop",
    zh_hans: "停止",
    zh_hant: "停止",
    ja: "停止",
    ko: "중지",
    de: "Anhalten",
    es: "Detener",
    fr: "Arrêter",
    hr: "Zaustavi",
    hu: "Leállítás",
    id: "Hentikan",
    nl: "Stoppen",
    pt: "Parar",
    ru: "Остановить",
};

/// The same button as [`ABANDON`], once the file has been written.
///
/// It does the same thing -- clear the workspace, back to the drop screen -- but
/// what that MEANS has changed. Before the save it is throwing work away; after
/// it, the work is safely on disk and this only starts the next video. "Discard"
/// beside a file that was just saved reads as though it might undo the save.
pub static ANOTHER: Text = Text {
    en: "Another one",
    zh_hans: "再处理一个",
    zh_hant: "再處理一個",
    ja: "次のファイル",
    ko: "다음 파일",
    de: "Noch eines",
    es: "Otro más",
    fr: "Un autre",
    hr: "Još jedan",
    hu: "Még egyet",
    id: "Satu lagi",
    nl: "Nog eentje",
    pt: "Mais um",
    ru: "Ещё один",
};

/// Throw the whole thing away and start from a file again.
pub static ABANDON: Text = Text {
    en: "Discard",
    zh_hans: "放弃",
    zh_hant: "放棄",
    ja: "破棄",
    ko: "버리기",
    de: "Verwerfen",
    es: "Descartar",
    fr: "Abandonner",
    hr: "Odbaci",
    hu: "Elvetés",
    id: "Buang",
    nl: "Weggooien",
    pt: "Descartar",
    ru: "Отменить",
};

/// Offered only where the advice above it is "trim it shorter".
pub static BACK_TO_TRIM: Text = Text {
    en: "Back to trimming",
    zh_hans: "回去裁剪",
    zh_hant: "回去裁剪",
    ja: "トリミングに戻る",
    ko: "자르기로 돌아가기",
    de: "Zurück zum Zuschneiden",
    es: "Volver al recorte",
    fr: "Revenir au découpage",
    hr: "Natrag na rezanje",
    hu: "Vissza a vágáshoz",
    id: "Kembali ke pemotongan",
    nl: "Terug naar bijsnijden",
    pt: "Voltar ao corte",
    ru: "Вернуться к обрезке",
};

/// The file is already uploadable; this makes it lighter for every player who
/// downloads it, which is why it is the LOUD button rather than the hedged one.
///
/// It read "try to make it smaller" and sat in a grey pill beside a big blue
/// Save. Both halves of that were wrong: the hedge undersells something Press is
/// confident about, and the weight pointed at the exit instead of at the
/// recommended step.
pub static SHRINK_FURTHER: Text = Text {
    en: "Make it smaller",
    zh_hans: "压得更小",
    zh_hant: "壓得更小",
    ja: "さらに小さく",
    ko: "더 작게",
    de: "Kleiner machen",
    es: "Hacerlo más pequeño",
    fr: "Le réduire",
    hr: "Smanji ga",
    hu: "Legyen kisebb",
    id: "Perkecil lagi",
    nl: "Kleiner maken",
    pt: "Deixar menor",
    ru: "Уменьшить",
};

pub static SAVE: Text = Text {
    en: "Save the video",
    zh_hans: "保存视频",
    zh_hant: "儲存影片",
    ja: "動画を保存",
    ko: "영상 저장",
    de: "Video speichern",
    es: "Guardar el vídeo",
    fr: "Enregistrer la vidéo",
    hr: "Spremi videozapis",
    hu: "Videó mentése",
    id: "Simpan video",
    nl: "Video opslaan",
    pt: "Salvar o vídeo",
    ru: "Сохранить видео",
};

/// The output's size while it is being written. The percentage says how far the
/// encoder has got; this says whether the result is going to be any use.
pub static STEP_WRITTEN: Text = Text {
    en: "{size} written of {limit} allowed",
    zh_hans: "已写入 {size}，上限 {limit}",
    zh_hant: "已寫入 {size}，上限 {limit}",
    ja: "{size} 書き込み済み / 上限 {limit}",
    ko: "{size} 기록됨 / 최대 {limit}",
    de: "{size} geschrieben von {limit} erlaubt",
    es: "{size} escritos de {limit} permitidos",
    fr: "{size} écrits sur {limit} autorisés",
    hr: "zapisano {size} od dopuštenih {limit}",
    hu: "{size} kiírva a megengedett {limit}-ból",
    id: "{size} ditulis dari {limit} yang diizinkan",
    nl: "{size} geschreven van {limit} toegestaan",
    pt: "{size} gravados de {limit} permitidos",
    ru: "записано {size} из разрешённых {limit}",
};

/// Labels the full path of the file that was written. The charter is about to
/// go and upload it, so where it went is not a detail.
pub static SAVED_TO: Text = Text {
    en: "Saved to",
    zh_hans: "已保存到",
    zh_hant: "已儲存到",
    ja: "保存先",
    ko: "저장 위치",
    de: "Gespeichert unter",
    es: "Guardado en",
    fr: "Enregistré dans",
    hr: "Spremljeno u",
    hu: "Mentve ide",
    id: "Disimpan ke",
    nl: "Opgeslagen in",
    pt: "Salvo em",
    ru: "Сохранено в",
};

/// The title of the save dialog.
pub static SAVE_TITLE: Text = Text {
    en: "Save the background video as",
    zh_hans: "背景视频保存为",
    zh_hant: "背景影片另存為",
    ja: "背景動画の保存先",
    ko: "배경 영상 저장",
    de: "Hintergrundvideo speichern unter",
    es: "Guardar el vídeo de fondo como",
    fr: "Enregistrer la vidéo de fond sous",
    hr: "Spremi pozadinski videozapis kao",
    hu: "Háttérvideó mentése másként",
    id: "Simpan video latar sebagai",
    nl: "Achtergrondvideo opslaan als",
    pt: "Salvar o vídeo de fundo como",
    ru: "Сохранить фоновое видео как",
};

/// Not a failure. Everything produced was legal except its weight, which is a
/// statement about the LENGTH of the source and nothing Press will decide on
/// the charter's behalf.
pub static STILL_TOO_LARGE: Text = Text {
    en: "Too long to fit",
    zh_hans: "太长了，压不进去",
    zh_hant: "太長了，壓不進去",
    ja: "長すぎて収まりません",
    ko: "너무 길어서 담기지 않습니다",
    de: "Zu lang, um hineinzupassen",
    es: "Demasiado largo para caber",
    fr: "Trop long pour tenir",
    hr: "Predugo da bi stalo",
    hu: "Túl hosszú, nem fér bele",
    id: "Terlalu panjang untuk muat",
    nl: "Te lang om te passen",
    pt: "Longo demais para caber",
    ru: "Слишком длинное, не помещается",
};

pub static STILL_TOO_LARGE_DETAIL: Text = Text {
    en: "Even at the lowest quality it comes to {size} MB, against a limit of \
         {limit} MB. Trim it shorter and press again.",
    zh_hans: "即使压到最低画质也有 {size} MB，上限是 {limit} MB。把它裁短一点再试一次。",
    zh_hant: "即使壓到最低畫質也有 {size} MB，上限是 {limit} MB。把它裁短一點再試一次。",
    ja: "画質を最低まで落としても {size} MB あり、上限は {limit} MB です。短く切ってからもう一度お試しください。",
    ko: "가장 낮은 화질로도 {size} MB이고 제한은 {limit} MB입니다. 더 짧게 잘라서 다시 시도해 주세요.",
    de: "Selbst bei niedrigster Qualität sind es {size} MB, das Limit liegt bei \
         {limit} MB. Kürze es und drücke noch einmal.",
    es: "Incluso con la calidad más baja ocupa {size} MB, y el límite es de \
         {limit} MB. Recórtalo y vuelve a intentarlo.",
    fr: "Même à la qualité la plus basse, il fait {size} Mo, pour une limite de \
         {limit} Mo. Raccourcissez-le et réessayez.",
    hr: "Čak i pri najnižoj kvaliteti ima {size} MB, a ograničenje je {limit} MB. \
         Skratite ga i pokušajte ponovno.",
    hu: "A legalacsonyabb minőségen is {size} MB, a korlát pedig {limit} MB. \
         Vágd rövidebbre, és próbáld újra.",
    id: "Bahkan pada kualitas terendah ukurannya {size} MB, sedangkan batasnya \
         {limit} MB. Potong lebih pendek lalu coba lagi.",
    nl: "Zelfs op de laagste kwaliteit is het {size} MB, tegen een limiet van \
         {limit} MB. Kort het in en probeer het opnieuw.",
    pt: "Mesmo na qualidade mais baixa fica com {size} MB, contra um limite de \
         {limit} MB. Corte um trecho e tente de novo.",
    ru: "Даже на самом низком качестве получается {size} МБ при пределе {limit} МБ. \
         Обрежьте его короче и попробуйте снова.",
};

pub static SHOW_IN_FOLDER: Text = Text {
    en: "Show in folder",
    zh_hans: "打开所在文件夹",
    zh_hant: "開啟所在資料夾",
    ja: "フォルダーで表示",
    ko: "폴더에서 보기",
    de: "Im Ordner anzeigen",
    es: "Mostrar en la carpeta",
    fr: "Afficher dans le dossier",
    hr: "Prikaži u mapi",
    hu: "Megjelenítés a mappában",
    id: "Tampilkan di folder",
    nl: "Toon in map",
    pt: "Mostrar na pasta",
    ru: "Показать в папке",
};

// ── the encoder ──────────────────────────────────────────────────────────

pub static NO_ENCODER: Text = Text {
    en: "The encoder is missing from this build, so nothing can be processed.",
    zh_hans: "这个构建里没有带编码器，没法处理任何文件。",
    zh_hant: "這個建置裡沒有帶編碼器，沒法處理任何檔案。",
    ja: "このビルドにはエンコーダーが含まれていないため、処理できません。",
    ko: "이 빌드에는 인코더가 없어 아무것도 처리할 수 없습니다.",
    de: "In diesem Build fehlt der Encoder, es kann nichts verarbeitet werden.",
    es: "A esta compilación le falta el codificador, así que no se puede procesar nada.",
    fr: "L'encodeur est absent de cette version : rien ne peut être traité.",
    hr: "U ovoj gradnji nedostaje koder, pa se ništa ne može obraditi.",
    hu: "Ebből a buildből hiányzik a kódoló, így semmit sem lehet feldolgozni.",
    id: "Build ini tidak menyertakan enkoder, jadi tidak ada yang bisa diproses.",
    nl: "In deze build ontbreekt de encoder, dus er kan niets verwerkt worden.",
    pt: "Falta o codificador nesta compilação, então nada pode ser processado.",
    ru: "В этой сборке нет кодировщика, обработать ничего нельзя.",
};

/// Every string, for the tests that hold the table to its own rules.
pub static EVERY_STRING: &[(&str, &Text)] = &[
    ("WINDOW_TITLE", &WINDOW_TITLE),
    ("TAGLINE", &TAGLINE),
    ("DROP_PROMPT", &DROP_PROMPT),
    ("DROP_HINT", &DROP_HINT),
    ("DROP_RELEASE", &DROP_RELEASE),
    ("DROP_PICK_INSTEAD", &DROP_PICK_INSTEAD),
    ("DROP_BLOCKED_BY_ADMIN", &DROP_BLOCKED_BY_ADMIN),
    ("READING", &READING),
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
    ("STEP_VERIFYING", &STEP_VERIFYING),
    ("CANCELLED", &CANCELLED),
    ("FAILED", &FAILED),
    ("STEP_WRITTEN", &STEP_WRITTEN),
    ("STEP_ATTEMPT", &STEP_ATTEMPT),
    ("STEP_QUALITY", &STEP_QUALITY),
    ("PHASE_A_CLEAR", &PHASE_A_CLEAR),
    ("PHASE_A_CLEAR_HINT", &PHASE_A_CLEAR_HINT),
    ("LIST_SEP", &LIST_SEP),
    ("NEXT_STEP", &NEXT_STEP),
    ("NOT_READY", &NOT_READY),
    ("WILL_LEAD", &WILL_LEAD),
    ("SHORT_RESOLUTION", &SHORT_RESOLUTION),
    ("SHORT_FPS", &SHORT_FPS),
    ("SHORT_CODEC", &SHORT_CODEC),
    ("SHORT_SIZE", &SHORT_SIZE),
    ("SHORT_OPENING", &SHORT_OPENING),
    ("CROP_OFFER", &CROP_OFFER),
    ("CROP_OPTION", &CROP_OPTION),
    ("CROP_NONE", &CROP_NONE),
    ("CROP_DRAG", &CROP_DRAG),
    ("SHORT_CROP", &SHORT_CROP),
    ("WILL_CROP", &WILL_CROP),
    ("WILL_SCALE", &WILL_SCALE),
    ("WILL_FPS", &WILL_FPS),
    ("WILL_CODEC", &WILL_CODEC),
    ("WILL_SIZE", &WILL_SIZE),
    ("WILL_OPENING", &WILL_OPENING),
    ("BENCH_READY", &BENCH_READY),
    ("BENCH_COMPRESSED", &BENCH_COMPRESSED),
    ("BENCH_SMALLER", &BENCH_SMALLER),
    ("BENCH_ALLOWANCE", &BENCH_ALLOWANCE),
    ("BENCH_REMAINING", &BENCH_REMAINING),
    ("DIFF_AUDIO", &DIFF_AUDIO),
    ("DIFF_TAIL", &DIFF_TAIL),
    ("DIFF_NOTHING", &DIFF_NOTHING),
    ("FIX_SIZE", &FIX_SIZE),
    ("SAID_MATERIAL_WINS", &SAID_MATERIAL_WINS),
    ("STEP_RECALIBRATING", &STEP_RECALIBRATING),
    ("STOP", &STOP),
    ("ABANDON", &ABANDON),
    ("ANOTHER", &ANOTHER),
    ("BACK_TO_TRIM", &BACK_TO_TRIM),
    ("SHRINK_FURTHER", &SHRINK_FURTHER),
    ("SAVE", &SAVE),
    ("SAVED_TO", &SAVED_TO),
    ("SAVE_TITLE", &SAVE_TITLE),
    ("STILL_TOO_LARGE", &STILL_TOO_LARGE),
    ("STILL_TOO_LARGE_DETAIL", &STILL_TOO_LARGE_DETAIL),
    ("SHOW_IN_FOLDER", &SHOW_IN_FOLDER),
    ("NO_ENCODER", &NO_ENCODER),
];
