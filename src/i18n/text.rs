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

pub static NOT_A_VIDEO: Text = Text {
    en: "That is not a video file",
    zh_hans: "这不是视频文件",
    zh_hant: "這不是影片檔案",
    ja: "これは動画ファイルではありません",
    ko: "영상 파일이 아닙니다",
    de: "Das ist keine Videodatei",
    es: "Eso no es un archivo de video",
    fr: "Ce n'est pas un fichier vidéo",
    hr: "To nije video datoteka",
    hu: "Ez nem videofájl",
    id: "Itu bukan berkas video",
    nl: "Dat is geen videobestand",
    pt: "Isso não é um arquivo de vídeo",
    ru: "Это не видеофайл",
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

pub static VERDICT_OK: Text = Text {
    en: "Ready to upload as it is",
    zh_hans: "已经符合要求，可以直接上传",
    zh_hant: "已經符合要求，可以直接上傳",
    ja: "このままアップロードできます",
    ko: "그대로 업로드할 수 있습니다",
    de: "Kann so hochgeladen werden",
    es: "Listo para subir tal cual",
    fr: "Prête à être envoyée telle quelle",
    hr: "Spreman za prijenos ovakav kakav jest",
    hu: "Így feltölthető",
    id: "Siap diunggah apa adanya",
    nl: "Kan zo geüpload worden",
    pt: "Pronto para enviar como está",
    ru: "Можно загружать как есть",
};

pub static VERDICT_WILL_FIX: Text = Text {
    en: "Press will fix this",
    zh_hans: "有问题，Press 会自动修好",
    zh_hant: "有問題，Press 會自動修好",
    ja: "問題があります。Press が直します",
    ko: "문제가 있습니다. Press가 고쳐 드립니다",
    de: "Press bringt das in Ordnung",
    es: "Press lo va a arreglar",
    fr: "Press va s'en occuper",
    hr: "Press će to popraviti",
    hu: "A Press megjavítja",
    id: "Press akan memperbaikinya",
    nl: "Press lost dit op",
    pt: "O Press vai corrigir isso",
    ru: "Press это исправит",
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

pub static FIX_AUDIO: Text = Text {
    en: "removed — the game plays music.ogg, so nobody ever hears it",
    zh_hans: "会去掉 —— 游戏播的是 music.ogg，这条音轨没有人听得到",
    zh_hant: "會去掉 —— 遊戲播的是 music.ogg，這條音軌沒有人聽得到",
    ja: "削除します。ゲームが再生するのは music.ogg なので、誰にも聞こえません",
    ko: "제거합니다. 게임은 music.ogg를 재생하므로 아무도 듣지 못합니다",
    de: "wird entfernt — das Spiel gibt music.ogg wieder, gehört wird sie nie",
    es: "se quitará — el juego reproduce music.ogg, así que nadie la oye",
    fr: "sera retirée — le jeu lit music.ogg, personne ne l'entend",
    hr: "bit će uklonjen — igra svira music.ogg, pa ga nitko ne čuje",
    hu: "eltávolítjuk — a játék a music.ogg-ot játssza, ezt senki sem hallja",
    id: "akan dihapus — gim memutar music.ogg, jadi tak ada yang mendengarnya",
    nl: "wordt verwijderd — de game speelt music.ogg, dus niemand hoort het",
    pt: "será removida — o jogo toca music.ogg, então ninguém a ouve",
    ru: "будет удалена — игра воспроизводит music.ogg, её никто не слышит",
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

pub static TRIM_NONE: Text = Text {
    en: "Whole video",
    zh_hans: "整段",
    zh_hant: "整段",
    ja: "動画全体",
    ko: "전체 영상",
    de: "Ganzes Video",
    es: "Video completo",
    fr: "Vidéo entière",
    hr: "Cijeli video",
    hu: "Teljes videó",
    id: "Seluruh video",
    nl: "Hele video",
    pt: "Vídeo inteiro",
    ru: "Всё видео",
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

pub static STEP_ENCODING: Text = Text {
    en: "Trying AV1 — it is kept only if it comes out smaller ({percent})",
    zh_hans: "正在试 AV1 —— 只有更小才会采用（{percent}）",
    zh_hant: "正在試 AV1 —— 只有更小才會採用（{percent}）",
    ja: "AV1 を試しています。小さくなった場合だけ採用します（{percent}）",
    ko: "AV1을 시도 중 — 더 작을 때만 사용합니다 ({percent})",
    de: "AV1 wird versucht — behalten wird es nur, wenn es kleiner ausfällt ({percent})",
    es: "Probando AV1: solo se conserva si sale más pequeño ({percent})",
    fr: "Essai en AV1 — conservé seulement s'il est plus petit ({percent})",
    hr: "Isprobava se AV1 — zadržava se samo ako ispadne manji ({percent})",
    hu: "AV1 próbája — csak akkor marad, ha kisebb lesz ({percent})",
    id: "Mencoba AV1 — hanya dipakai bila hasilnya lebih kecil ({percent})",
    nl: "AV1 wordt geprobeerd — alleen behouden als het kleiner uitvalt ({percent})",
    pt: "Testando AV1 — só fica se sair menor ({percent})",
    ru: "Пробуем AV1 — оставим, только если он меньше ({percent})",
};

pub static STEP_ENCODING_FORCED: Text = Text {
    en: "Re-encoding to AV1 ({percent})",
    zh_hans: "正在转成 AV1（{percent}）",
    zh_hant: "正在轉成 AV1（{percent}）",
    ja: "AV1 に変換中（{percent}）",
    ko: "AV1로 변환 중 ({percent})",
    de: "Wird zu AV1 neu kodiert ({percent})",
    es: "Recodificando a AV1 ({percent})",
    fr: "Réencodage en AV1 ({percent})",
    hr: "Prekodiranje u AV1 ({percent})",
    hu: "Átkódolás AV1-re ({percent})",
    id: "Mengodekan ulang ke AV1 ({percent})",
    nl: "Omzetten naar AV1 ({percent})",
    pt: "Recodificando para AV1 ({percent})",
    ru: "Перекодирование в AV1 ({percent})",
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

/// Cancelling when a compliant stream copy is already in hand.
pub static CANCEL_KEEP: Text = Text {
    en: "Stop and use what I have",
    zh_hans: "不等了，就用现在这个",
    zh_hant: "不等了，就用現在這個",
    ja: "ここまでで確定する",
    ko: "여기까지로 확정",
    de: "Anhalten und das Bisherige nehmen",
    es: "Parar y usar lo que ya hay",
    fr: "Arrêter et garder ce qu'on a",
    hr: "Stani i uzmi ovo što ima",
    hu: "Állj le, és ez legyen",
    id: "Berhenti dan pakai yang sudah ada",
    nl: "Stoppen en nemen wat er is",
    pt: "Parar e usar o que já tenho",
    ru: "Остановить и взять текущее",
};

/// Cancelling when the encode IS the fix and stopping leaves nothing.
pub static CANCEL_ABANDON: Text = Text {
    en: "Give up on this file",
    zh_hans: "放弃这个文件",
    zh_hant: "放棄這個檔案",
    ja: "このファイルをあきらめる",
    ko: "이 파일을 포기",
    de: "Diese Datei aufgeben",
    es: "Renunciar a este archivo",
    fr: "Abandonner ce fichier",
    hr: "Odustani od ove datoteke",
    hu: "Feladom ezt a fájlt",
    id: "Menyerah pada berkas ini",
    nl: "Dit bestand opgeven",
    pt: "Desistir deste arquivo",
    ru: "Отказаться от этого файла",
};

pub static CANCEL_ABANDON_HINT: Text = Text {
    en: "This one can only be fixed by re-encoding, so stopping leaves nothing.",
    zh_hans: "这个文件只能靠重编码修好，停下来就什么都不会有。",
    zh_hant: "這個檔案只能靠重編碼修好，停下來就什麼都不會有。",
    ja: "このファイルは再エンコードでしか直せないため、中断すると何も残りません。",
    ko: "이 파일은 재인코딩으로만 고칠 수 있어서, 중단하면 아무것도 남지 않습니다.",
    de: "Diese Datei lässt sich nur durch Neukodieren korrigieren — ein Abbruch \
         hinterlässt nichts.",
    es: "Este archivo solo se puede arreglar recodificando, así que parar no deja nada.",
    fr: "Ce fichier ne peut être corrigé qu'en réencodant : s'arrêter ne laisse rien.",
    hr: "Ova se datoteka može popraviti samo prekodiranjem, pa prekid ne ostavlja ništa.",
    hu: "Ezt a fájlt csak újrakódolással lehet megjavítani, így a leállítás után semmi sem marad.",
    id: "Berkas ini hanya bisa diperbaiki dengan pengodean ulang, jadi berhenti tidak \
         menyisakan apa pun.",
    nl: "Dit bestand is alleen met opnieuw coderen te herstellen; stoppen levert niets op.",
    pt: "Este arquivo só pode ser corrigido recodificando, então parar não deixa nada.",
    ru: "Этот файл можно исправить только перекодированием, поэтому после остановки \
         ничего не останется.",
};

// ── the result ───────────────────────────────────────────────────────────

pub static DONE: Text = Text {
    en: "Done — this file will be accepted",
    zh_hans: "完成 —— 这个文件会被接受",
    zh_hant: "完成 —— 這個檔案會被接受",
    ja: "完了しました。このファイルは受け付けられます",
    ko: "완료 — 이 파일은 통과합니다",
    de: "Fertig — diese Datei wird angenommen",
    es: "Listo: este archivo será aceptado",
    fr: "Terminé — ce fichier sera accepté",
    hr: "Gotovo — ova će datoteka biti prihvaćena",
    hu: "Kész — ezt a fájlt elfogadják",
    id: "Selesai — berkas ini akan diterima",
    nl: "Klaar — dit bestand wordt geaccepteerd",
    pt: "Pronto — este arquivo será aceito",
    ru: "Готово — этот файл примут",
};

pub static DONE_SIZE: Text = Text {
    en: "{before} → {after}",
    zh_hans: "{before} → {after}",
    zh_hant: "{before} → {after}",
    ja: "{before} → {after}",
    ko: "{before} → {after}",
    de: "{before} → {after}",
    es: "{before} → {after}",
    fr: "{before} → {after}",
    hr: "{before} → {after}",
    hu: "{before} → {after}",
    id: "{before} → {after}",
    nl: "{before} → {after}",
    pt: "{before} → {after}",
    ru: "{before} → {after}",
};

pub static DONE_UNTOUCHED: Text = Text {
    en: "Picture untouched — nothing was re-encoded",
    zh_hans: "画面没有被动过 —— 全程没有重编码",
    zh_hant: "畫面沒有被動過 —— 全程沒有重編碼",
    ja: "映像はそのままです。再エンコードは行っていません",
    ko: "화면은 그대로입니다 — 재인코딩하지 않았습니다",
    de: "Bild unangetastet — es wurde nichts neu kodiert",
    es: "Imagen intacta: no se recodificó nada",
    fr: "Image intacte — rien n'a été réencodé",
    hr: "Slika je netaknuta — ništa nije prekodirano",
    hu: "A kép érintetlen — semmit sem kódoltunk át",
    id: "Gambar tidak disentuh — tidak ada pengodean ulang",
    nl: "Beeld onaangeroerd — er is niets opnieuw gecodeerd",
    pt: "Imagem intocada — nada foi recodificado",
    ru: "Картинка не тронута — ничего не перекодировалось",
};

pub static DONE_AV1_LOST: Text = Text {
    en: "AV1 was tried and came back larger, so the original picture was kept",
    zh_hans: "试过 AV1，结果反而更大，所以保留了原画面",
    zh_hant: "試過 AV1，結果反而更大，所以保留了原畫面",
    ja: "AV1 も試しましたが大きくなったため、元の映像を残しました",
    ko: "AV1도 시도했지만 더 커져서 원본 화면을 유지했습니다",
    de: "AV1 wurde versucht und fiel größer aus, also blieb das Originalbild",
    es: "Se probó AV1 y salió más grande, así que se conservó la imagen original",
    fr: "L'AV1 a été essayé et s'est révélé plus lourd : l'image d'origine est conservée",
    hr: "AV1 je isproban i ispao je veći, pa je zadržana izvorna slika",
    hu: "Az AV1 nagyobb lett, ezért az eredeti kép maradt",
    id: "AV1 sudah dicoba tapi hasilnya lebih besar, jadi gambar aslinya dipertahankan",
    nl: "AV1 is geprobeerd en werd groter, dus het oorspronkelijke beeld is behouden",
    pt: "O AV1 foi testado e ficou maior, então a imagem original foi mantida",
    ru: "AV1 попробовали, он оказался больше — оставили исходную картинку",
};

pub static DONE_AV1_SKIPPED: Text = Text {
    en: "The AV1 comparison was stopped, so this may not be the smallest possible",
    zh_hans: "AV1 比对被中断了，所以这不一定是最小的版本",
    zh_hant: "AV1 比對被中斷了，所以這不一定是最小的版本",
    ja: "AV1 との比較を中断したため、これが最小とは限りません",
    ko: "AV1 비교를 중단했으므로 이것이 가장 작은 결과가 아닐 수 있습니다",
    de: "Der AV1-Vergleich wurde abgebrochen, dies ist also womöglich nicht das Kleinste",
    es: "Se detuvo la comparación con AV1, así que puede que no sea lo más pequeño posible",
    fr: "La comparaison AV1 a été interrompue : ce n'est peut-être pas le plus petit résultat",
    hr: "Usporedba s AV1 je prekinuta, pa ovo možda nije najmanja moguća datoteka",
    hu: "Az AV1-összehasonlítás megszakadt, így lehet, hogy nem ez a legkisebb",
    id: "Perbandingan AV1 dihentikan, jadi ini mungkin bukan yang terkecil",
    nl: "De AV1-vergelijking is gestopt, dus dit is mogelijk niet het kleinste resultaat",
    pt: "A comparação com AV1 foi interrompida, então talvez não seja o menor possível",
    ru: "Сравнение с AV1 прервано, поэтому это может быть не самый маленький вариант",
};

pub static ALREADY_STANDARD: Text = Text {
    en: "Nothing to do — this file is already exactly right",
    zh_hans: "不用动 —— 这个文件本来就完全合规",
    zh_hant: "不用動 —— 這個檔案本來就完全合規",
    ja: "何もする必要はありません。このファイルはすでに条件を満たしています",
    ko: "할 일이 없습니다 — 이 파일은 이미 조건을 만족합니다",
    de: "Nichts zu tun — diese Datei passt bereits genau",
    es: "No hay nada que hacer: este archivo ya está exactamente bien",
    fr: "Rien à faire — ce fichier est déjà exactement conforme",
    hr: "Nema se što raditi — ova je datoteka već točno kakva treba biti",
    hu: "Nincs teendő — ez a fájl már pontosan megfelel",
    id: "Tidak ada yang perlu dilakukan — berkas ini sudah tepat",
    nl: "Niets te doen — dit bestand is al precies goed",
    pt: "Nada a fazer — este arquivo já está exatamente certo",
    ru: "Делать нечего — файл уже полностью соответствует",
};

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

pub static ANOTHER: Text = Text {
    en: "Another one",
    zh_hans: "再来一个",
    zh_hant: "再來一個",
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
