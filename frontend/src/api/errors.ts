// The errors the API answers with. The server sends each as `{ group, code, message }`: the
// group and code are one of the grouped enums in backend/src/error.rs (`TeamError::NameTaken`
// is group "team", code "name_taken"), the message its English text. The page shows the
// Turkish text below instead.

/** What each error says to the user, by group. `errors.test.ts` checks it against the backend's enums. */
export const ERROR_TEXT = {
  general: {
    not_found: "Bulunamadı.",
    beatmap_not_found: "Beatmap bulunamadı.",
    osu_api_unreachable: "osu! API'sine ulaşılamadı.",
    osu_api_bad_response: "osu! API'sinin cevabı okunamadı.",
    internal: "Sunucuda bir hata oluştu.",
  },
  auth: {
    unauthenticated: "Oturumun sona erdi; yeniden giriş yap.",
    forbidden: "Bunun için yetkin yok.",
    discord_required: "Önce Discord hesabını bağla.",
    not_in_discord_server: "Önce turnuvanın Discord sunucusuna yeniden katıl.",
    invalid_login: "Giriş denemesi geçersiz ya da süresi doldu.",
    osu_login_failed: "osu! ile giriş tamamlanamadı.",
    discord_link_failed: "Discord bağlanamadı.",
    own_host_role: "Kendi host rolünü kaldıramazsın.",
    banned: "Turnuvadan banlandın.",
    own_ban: "Kendini banlayamazsın.",
    ban_reason_length: "Ban sebebi en fazla 300 karakter olabilir.",
    dev_live_needs_real_account: "Canlı test için gerçek bir osu! hesabıyla giriş yap.",
    dev_autoref_players: "Oto hakem testi için dört farklı hesap seç.",
    dev_autoref_stage: "Bu aşamanın map havuzu, best of'u ve pick-ban sırası olmalı.",
  },
  team: {
    not_in_team: "Bir takımda değilsin.",
    already_in_team: "Zaten bir takımdasın.",
    player_in_team: "Bu oyuncu zaten bir takımda.",
    someone_in_team: "İkinizden biri zaten bir takımda.",
    staff_cant_play: "Turnuva ekibi takımlara katılamaz.",
    player_banned: "Bu oyuncu turnuvadan banlı.",
    needs_global_rank: "İki oyuncunun da global rankı olmalı.",
    over_rank_limit: "Bu takım rank sınırını aşar.",
    rank_update_running: "Rank güncellemesi zaten sürüyor.",
    frozen: "Takımlar donduruldu.",
    name_length: "Takım adı 1 ile 32 karakter arasında olmalı.",
    name_characters: "Takım adı kontrol karakteri içeremez.",
    name_taken: "Bu takım adı alınmış.",
    invite_self: "Kendini davet edemezsin.",
    invite_already_sent: "Davet zaten gönderilmiş.",
    they_invited_you: "Seni zaten davet ettiler; onların davetini kabul et.",
    bad_banner: "Banner PNG, JPEG, GIF ya da WebP olmalı.",
    banner_too_large:
      "Banner çok büyük: bir kenarı en fazla 4096 piksel, tamamı en fazla 4096×2048 olabilir; hareketli GIF daha küçük ya da daha kısa olmalı.",
    banner_cooldown: "Yeni bir banner yüklemeden önce birkaç saniye bekle.",
  },
  pool: {
    name_required: "Bir isim gir.",
    unknown_stage: "Böyle bir aşama yok.",
    qualifier_stage_changed: "Eleme aşaması az önce değişti, tekrar dene.",
    bad_best_of: "Best of 1 ile 25 arasında tek bir sayı olmalı.",
    draft_order_empty: "Sıralamada en az bir ban ya da pick olmalı.",
    draft_order_step: "Her adım p1, p2, b1 ya da b2 olmalı; * yalnızca sonda.",
    draft_order_too_long: "Sıralama 40 adımdan uzun.",
    bad_pack_link: "Paket linki geçerli bir adres değil.",
    pack_link_scheme: "Paket linki http:// ya da https:// ile başlamalı.",
    pack_link_too_long: "Paket linki çok uzun.",
    abbreviation_too_long: "Kısaltma en fazla 6 karakter olabilir.",
  },
  bracket: {
    bad_match_number: "Maç numaraları 1'den başlar.",
    match_number_taken: "Bu numara başka bir maçta kullanılıyor.",
    unknown_team: "Böyle bir takım yok.",
    team_plays_itself: "Bir takım kendisiyle oynayamaz.",
    bad_slot: "Takım yeri 1 ya da 2 olmalı.",
    slot_has_source: "Bu yere zaten başka bir maçtan takım geliyor.",
    link_to_itself: "Bir maç kendisine bağlanamaz.",
    loop: "Bu bağlantı bracket'i bu maça geri döndürür.",
    not_empty: "Bracket'te zaten maç var; önce maçları kaldır.",
    already_played: "Bracket'te oynanmış maçlar var; bracket kaldırılamaz.",
    qualifiers_not_over: "Önce bütün eleme lobileri bitirilmeli.",
    bad_seeding: "Sıralamada her takım bir kez yer almalı.",
    wrong_team_count: "Bracket, formatının aldığı sayıda takımla kurulur.",
    bad_stages: "Bracket'in her haftasına bir stage seçilmeli.",
  },
  match: {
    changed: "Maç değişti; yenileyip tekrar dene.",
    concluded: "Bu maç bitti.",
    concluded_results: "Bu maç bitti; sonuçlarını değiştirmek için yeniden aç.",
    concluded_score: "Bu maç bitti; skoru değiştirmek için yeniden aç.",
    concluded_teams:
      "Bu maç bitti; takımlarını ya da bağlantılarını değiştirmeden önce yeniden aç.",
    next_concluded: "Bu maçın bağlandığı maç bitmiş; önce onu yeniden aç.",
    needs_room: "Önce maçın odasını ekle.",
    concluded_needs_room: "Bitmiş bir maçın en az bir odası olmalı; önce maçı yeniden aç.",
    needs_both_teams: "Önce maçın iki takımı da belli olmalı.",
    needs_winner: "Önce kazananı belli olan bir skor gir.",
    team_not_in_match: "Bu takım bu maçta oynamıyor.",
    player_not_in_match: "Bu oyuncu bu maçta oynamıyor.",
    bad_score: "Skorlar 0 ile 99 arasında olmalı.",
    bad_score_mode: "Bilinmeyen skor modu.",
    too_many_results: "Çok fazla harita sonucu.",
    result_needs_map: "Her harita sonucunun bir haritası olmalı.",
    negative_map_score: "Harita skorları negatif olamaz.",
    bad_map_scores: "Bir haritanın oyuncu skorları geçersiz.",
    bad_map_mods: "Bir haritanın modları geçersiz.",
    map_not_in_pool: "Bu harita maçın havuzunda değil.",
    not_a_match_map: "Bu harita bu maça ait değil.",
    map_banned: "Bu harita banlandı.",
    map_picked: "Bu harita seçildi.",
    note_too_long: "Not en fazla 200 karakter olabilir.",
    notes_too_long: "Notlar en fazla 2000 karakter olabilir.",
  },
  schedule: {
    match_started: "Maç başladı; artık yalnızca ekip taşıyabilir.",
    no_opponent: "Bu maçın rakibi henüz belli değil.",
    time_in_past: "İleri bir zaman seç.",
    time_passed: "Bu zaman geçti.",
    same_time: "Maç zaten bu saatte.",
    already_proposed: "Takımın bu saati zaten önerdi.",
    they_proposed_time: "Bu saati zaten onlar önerdi; onlarınkini kabul et.",
    own_proposal: "Bu saati senin takımın önerdi; diğer takım kabul etmeli.",
    too_many_proposals: "Takımının bu maç için çok fazla önerisi var; önce bazılarını iptal et.",
    time_changed: "Maçın saati o zamandan beri değişti.",
    moved_again: "Maç o zamandan beri yine taşındı; yeni bildirimi yanıtla.",
    already_answered: "Bunu zaten yanıtladın.",
    no_longer_on_match: "Artık bu maçta görevli değilsin.",
  },
  staff: {
    not_a_referee: "Yalnızca hakemler maç yönetebilir.",
    has_referee: "Bu maçın zaten bir hakemi var.",
    not_a_streamer: "Yalnızca yayıncılar yayın alabilir.",
    stream_own_match: "Oynadığın maçın yayınını alamazsın.",
    has_streamer: "Bu maçın zaten bir yayıncısı var.",
  },
  lobby: {
    closed: "Bu lobi kapandı.",
    started: "Lobin çoktan başladı.",
    full: "Bu lobi dolu.",
  },
  bancho: {
    bad_username: "Bu geçerli bir osu! kullanıcı adı değil.",
    bad_irc_password: "Bu geçerli bir IRC şifresi değil.",
    already_a_bot: "Bu hesap zaten bir bot.",
    bot_secret_missing: "Sunucuda BOT_SECRET_KEY ayarlı değil, bot şifresi kaydedilemiyor.",
    no_bot_connected: "Bancho'ya bağlı bot yok.",
    bots_full: "Bütün botların açık oda sınırı (her biri 4) dolu.",
    bot_disconnected: "Bot Bancho bağlantısını kaybetti.",
    bot_turned_off: "Bot kapatıldı.",
    banchobot_no_answer: "BanchoBot cevap vermedi; botun sohbetine bak.",
    restarting: "Oda yöneticisi yeniden başlıyor, tekrar dene.",
  },
  room: {
    bad_mp_link: "Bu bir osu! multiplayer linki değil.",
    taken: "Bu oda zaten bir maça bağlı.",
    closed: "Bu oda kapandı.",
    match_not_run: "Bu maç botlar tarafından yönetilmiyor.",
    without_bot: "Bu odayı yöneten bot yok; yeni oda aç.",
    bot_offline: "Odanın botu Bancho'ya bağlı değil; yeni oda aç.",
    bot_not_in_room: "Odanın botu henüz odada değil.",
    bad_slot: "Slotlar 1'den 16'ya kadar.",
    slot_empty: "O slotta kimse yok.",
    staff_not_loaded: "Ekip listesi henüz yüklenmedi.",
    cant_be_referee: "Odaya yalnızca hakem, yayıncı ve yorumcular hakem olarak eklenebilir.",
    player_cant_referee: "Maçın oyuncusu odanın hakemi olamaz.",
    bot_in_bot_room: "Bir bot başka bir botun odasına eklenemez.",
    not_referee: "Bu kişi odanın hakemi değil.",
    refs_from_settings: "Odanın hakemlerini oda ayarlarından ekleyip çıkar.",
    empty_message: "Önce bir şey yaz.",
  },
  discord: {
    bot_missing: "Discord botu kurulu değil.",
    bot_off: "Discord botu kapalı.",
    no_channel: "Bu maçın henüz Discord kanalı yok.",
    player_not_in_server: "Bu oyuncu Discord sunucusunda değil.",
    ping_cooldown: "Bu oyuncu az önce etiketlendi, biraz bekle.",
  },
  pickem: {
    closed: "Bu aşamanın pick'emi açık değil.",
    cant_predict: "Bu maç için tahmin yapılamaz.",
    impossible_score: "Bu skor bu maçta mümkün değil.",
  },
  arg: {
    invalid_guess: "Geçersiz tahmin.",
    too_many_guesses: "Çok fazla deneme; 10 dakika bekleyip tekrar dene.",
  },
  rules: {
    too_long: "Kurallar çok uzun.",
    changed:
      "Sen açtıktan sonra kurallar başkası tarafından kaydedildi; onların sürümünü görmek için yenile.",
  },
} satisfies Record<string, Record<string, string>>;

type ErrorTexts = typeof ERROR_TEXT;
export type ErrorGroup = keyof ErrorTexts;
/** An error as `group.code`, e.g. `"lobby.full"`. */
export type ErrorCode = { [G in ErrorGroup]: `${G}.${keyof ErrorTexts[G] & string}` }[ErrorGroup];

const has = (table: object, key: string) => Object.prototype.hasOwnProperty.call(table, key);

/** The known code for a `{ group, code }` answer, else null. */
function codeOf(group: unknown, code: unknown): ErrorCode | null {
  if (typeof group !== "string" || typeof code !== "string" || !has(ERROR_TEXT, group)) return null;
  return has(ERROR_TEXT[group as ErrorGroup], code) ? (`${group}.${code}` as ErrorCode) : null;
}

/** The Turkish text for a code. */
export function errorText(code: ErrorCode): string {
  const [group, name] = code.split(".") as [ErrorGroup, string];
  return (ERROR_TEXT[group] as Record<string, string>)[name];
}

/** An error answer from the API: its code, and the Turkish text to show for it. */
export class ApiError extends Error {
  /** null for an answer without a known code (a proxy's error page, a newer server). */
  readonly code: ErrorCode | null;
  readonly status: number;

  constructor(status: number, code: ErrorCode | null, serverMessage: string | null = null) {
    super(code ? errorText(code) : serverMessage || `İstek başarısız oldu (${status})`);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

/** Reads an error response's `{ group, code, message }`. */
export async function apiErrorOf(res: Response): Promise<ApiError> {
  const text = await res.text().catch(() => "");
  try {
    const body = JSON.parse(text) as { group?: unknown; code?: unknown; message?: unknown };
    const message = typeof body.message === "string" ? body.message : null;
    return new ApiError(res.status, codeOf(body.group, body.code), message);
  } catch {
    return new ApiError(res.status, null, null);
  }
}

/**
 * Why the osu! login (or a Discord link without a session) came back without signing in:
 * `/?login_error=`, from `CallbackError` in backend/src/handlers/auth.rs.
 */
export type LoginError = "cancelled" | "expired" | "failed" | "signed_out";

export const LOGIN_ERROR_TEXT: Record<LoginError, string> = {
  cancelled: "osu! ile girişi iptal ettin.",
  expired: "Giriş denemesinin süresi doldu ya da başka bir tarayıcıda başlatıldı; tekrar dene.",
  failed: "osu! ile giriş tamamlanamadı; biraz sonra tekrar dene.",
  signed_out: "Oturumun kapanmış; giriş yapıp Discord'u yeniden bağla.",
};

/** Why linking Discord didn't go through: `/discord?error=`, from the same `CallbackError`. */
export type DiscordLinkError = "cancelled" | "expired" | "failed" | "taken" | "wrong_user";

export const DISCORD_LINK_ERROR_TEXT: Record<DiscordLinkError, string> = {
  cancelled: "Discord bağlamayı iptal ettin.",
  expired: "Bağlama denemesinin süresi doldu ya da başka bir tarayıcıda başlatıldı; tekrar dene.",
  failed: "Discord bağlanamadı; biraz sonra tekrar dene.",
  taken: "Bu Discord hesabı başka bir osu! hesabına bağlı.",
  wrong_user: "Bağlamayı başka bir hesapla başlatmıştın; bu hesapla tekrar dene.",
};

/** The text for a code read from the address bar, which may be anything. */
export function textFor<K extends string>(
  table: Record<K, string>,
  code: string,
  fallback: K,
): string {
  return code in table ? table[code as K] : table[fallback];
}
