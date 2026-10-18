//! The ARG: a hidden page behind the navbar's trophy where signed-in players with a linked
//! Discord type secret messages. There are three, entered in any order, each answered by its
//! reply; the first to enter the third wins. The answers and replies live only in the
//! environment (`ArgConfig`) and the checking only here, so the frontend carries no hint of
//! them: a player is only sent a message's reply in answer to the guess that entered it.
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fmt::Write};
use time::{OffsetDateTime, UtcOffset};

use crate::{auth::AuthUser, config::ArgMessage, db, error::AppError, AppState};

// Guesses a player may make in `db::arg::recent_attempts`' window (10 minutes), against
// brute forcing.
const GUESS_LIMIT: i64 = 10;
const MAX_GUESS_LENGTH: usize = 500;
// Wrong guesses listed on the results page.
const WRONG_GUESSES_SHOWN: i64 = 200;
// The message whose first finder wins (`db::arg::players` sorts by it too).
const WINNING_MESSAGE: i16 = 3;

// Routes under the `/api` nest, named after the page (`/kupa`) rather than the ARG, so the
// network tab gives nothing away.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/kupa", post(guess))
        .route("/kupa/results/:token", get(results).post(reset))
}

// The puzzle's messages, for a signed-in player with a linked Discord; 404 while it's off.
fn messages_for<'a>(state: &'a AppState, user: &AuthUser) -> Result<[ArgMessage<'a>; 3], AppError> {
    user.require_discord_verified()?;
    state.config.arg.messages().ok_or(AppError::NotFound)
}

// Answers match ignoring case and how the words are spaced. Turkish and English disagree
// on casing i (İ/i, I/ı) and keyboards differ, so the four of them count as one.
fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace("i\u{307}", "i")
        .replace('ı', "i")
}

// Which message (1-based) a guess enters, if any.
fn matching(messages: &[ArgMessage], guess: &str) -> Option<i16> {
    let guess = normalize(guess);
    let index = messages.iter().position(|m| normalize(m.answer) == guess)?;
    i16::try_from(index + 1).ok()
}

#[derive(Debug, Deserialize)]
pub struct GuessBody {
    guess: String,
}

#[derive(Debug, Serialize)]
pub struct GuessResult {
    correct: bool,
    // The reply of the message this guess entered; none for a wrong one.
    reply: Option<String>,
}

// POST /api/kupa — checks a guess against every message, so they can be entered in
// any order. Entering one again answers with its reply again, at the place first earned.
async fn guess(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<GuessBody>,
) -> Result<impl IntoResponse, AppError> {
    let messages = messages_for(&state, &user)?;
    let guess = body.guess.trim();
    if guess.is_empty() || guess.chars().count() > MAX_GUESS_LENGTH {
        return Err(AppError::BadRequest("invalid guess"));
    }
    if db::arg::recent_attempts(&state.db, user.id).await? >= GUESS_LIMIT {
        return Err(AppError::TooManyRequests(
            "too many guesses; wait 10 minutes and try again",
        ));
    }

    let message = matching(&messages, guess);
    let mut tx = state.db.begin().await?;
    db::arg::record_attempt(&mut tx, user.id, message, guess).await?;
    if let Some(message) = message {
        db::arg::solve(&mut tx, user.id, message).await?;
    }
    tx.commit().await?;

    let Some(message) = message else {
        return Ok(Json(GuessResult {
            correct: false,
            reply: None,
        }));
    };
    tracing::info!(user = %user.id, message, "arg message entered");
    // Read back for the place, which counts everyone who entered it before this commit.
    let place = db::arg::place_of(&state.db, user.id, message).await?;
    let reply = messages[usize::try_from(message - 1).unwrap_or_default()].reply;
    Ok(Json(GuessResult {
        correct: true,
        reply: place.map(|place| reply.replace("{place}", &place.to_string())),
    }))
}

// The results page and its reset are for whoever has ARG_RESULTS_TOKEN: 404 for any other
// token, or when it isn't set.
fn check_token(state: &AppState, token: &str) -> Result<(), AppError> {
    let expected = state.config.arg.results_token().ok_or(AppError::NotFound)?;
    // Comparing digests rather than the strings keeps the time taken from saying how
    // much of a guessed token was right.
    if Sha256::digest(token.as_bytes()) != Sha256::digest(expected.as_bytes()) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ResultsQuery {
    // Set by the reset's redirect, to say it was done.
    reset: Option<String>,
}

// GET /api/kupa/results/:token — who solved what, when and in what order, as a page for
// the host who set ARG_RESULTS_TOKEN.
async fn results(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<ResultsQuery>,
) -> Result<impl IntoResponse, AppError> {
    check_token(&state, &token)?;

    let solves = db::arg::all_solves(&state.db).await?;
    let players = db::arg::players(&state.db).await?;
    let wrong = db::arg::wrong_guesses(&state.db, WRONG_GUESSES_SHOWN).await?;
    let page = results_page(
        state.config.arg.messages().is_some(),
        query.reset.is_some(),
        &solves,
        &players,
        &wrong,
    );

    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
            (header::HeaderName::from_static("x-robots-tag"), "noindex"),
        ],
        Html(page),
    ))
}

// POST /api/kupa/results/:token — the results page's reset: deletes every guess and every
// message found, so the ARG starts over (places, the winner and the guess limits with it).
// The page's own URL is the form's target, so the token never has to be written into it.
async fn reset(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    check_token(&state, &token)?;
    db::arg::reset(&state.db).await?;
    tracing::warn!("arg guesses reset from the results page");
    // Relative, so it lands back on this same page; a 303, so reloading it doesn't post again.
    Ok(Redirect::to("?reset=1"))
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

// Turkey's time, which has been UTC+3 all year since 2016, to the second.
fn when(at: OffsetDateTime) -> String {
    let at = at.to_offset(UtcOffset::from_hms(3, 0, 0).expect("valid offset"));
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

fn player_cell(osu_id: i64, username: &str, staff: bool) -> String {
    let staff = if staff {
        r#" <span class="tag">ekip</span>"#
    } else {
        ""
    };
    format!(
        r#"<a href="https://osu.ppy.sh/users/{osu_id}">{}</a>{staff}"#,
        escape(username)
    )
}

fn results_page(
    enabled: bool,
    reset_done: bool,
    solves: &[db::arg::SolveRow],
    players: &[db::arg::PlayerRow],
    wrong: &[db::arg::WrongGuess],
) -> String {
    let mut body = String::new();
    if reset_done {
        body.push_str(r#"<p class="notice">Denemeler sıfırlandı.</p>"#);
    }
    if !enabled {
        body.push_str(r#"<p class="warn">ARG kapalı: üç ARG_ANSWER_* ve ARG_REPLY_* değişkeninin hepsi ayarlı değil.</p>"#);
    }

    let winner = solves.iter().find(|s| s.message == WINNING_MESSAGE);
    match winner {
        Some(w) => {
            let _ = write!(
                body,
                r#"<p class="winner">Kazanan: {} · {}</p>"#,
                player_cell(w.osu_id, &w.username, w.staff),
                when(w.solved_at)
            );
        }
        None => body.push_str(r#"<p class="muted">Henüz kazanan yok.</p>"#),
    }

    let message_name = |message: i16| {
        if message == WINNING_MESSAGE {
            format!("Mesaj {message} <span class=\"muted\">(kazandıran)</span>")
        } else {
            format!("Mesaj {message}")
        }
    };

    // How many found each message, and who first.
    body.push_str("<h2>Mesajlar</h2><div class=\"scroll\"><table><tr><th>Mesaj</th><th>Bulan</th><th>İlk bulan</th><th>Zaman (TSİ)</th></tr>");
    for message in 1..=WINNING_MESSAGE {
        let rows: Vec<_> = solves.iter().filter(|s| s.message == message).collect();
        let (first, at) = match rows.first() {
            Some(s) => (
                player_cell(s.osu_id, &s.username, s.staff),
                when(s.solved_at),
            ),
            None => ("<span class=\"muted\">–</span>".to_owned(), String::new()),
        };
        let _ = write!(
            body,
            "<tr><td>{}</td><td>{}</td><td>{first}</td><td>{at}</td></tr>",
            message_name(message),
            rows.len()
        );
    }
    body.push_str("</table></div>");

    for message in 1..=WINNING_MESSAGE {
        let rows: Vec<_> = solves.iter().filter(|s| s.message == message).collect();
        let _ = write!(
            body,
            "<h2>{} <span class=\"muted\">· {} kişi</span></h2>",
            message_name(message),
            rows.len()
        );
        if rows.is_empty() {
            body.push_str(r#"<p class="muted">Henüz bulan yok.</p>"#);
            continue;
        }
        body.push_str(
            "<div class=\"scroll\"><table><tr><th>#</th><th>osu!</th><th>Discord</th><th>Zaman (TSİ)</th></tr>",
        );
        for s in rows {
            let _ = write!(
                body,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                s.place,
                player_cell(s.osu_id, &s.username, s.staff),
                escape(s.discord_username.as_deref().unwrap_or("")),
                when(s.solved_at)
            );
        }
        body.push_str("</table></div>");
    }

    let _ = write!(
        body,
        "<h2>Deneyenler <span class=\"muted\">· {} kişi</span></h2>",
        players.len()
    );
    if players.is_empty() {
        body.push_str(r#"<p class="muted">Henüz deneyen yok.</p>"#);
    } else {
        let found: HashMap<(i64, i16), &db::arg::SolveRow> =
            solves.iter().map(|s| ((s.osu_id, s.message), s)).collect();
        body.push_str("<div class=\"scroll\"><table><tr><th>osu!</th><th>Discord</th>");
        for message in 1..=WINNING_MESSAGE {
            let _ = write!(body, "<th>Mesaj {message}</th>");
        }
        body.push_str("<th>Deneme</th><th>İlk deneme</th><th>Son deneme</th></tr>");
        for p in players {
            let _ = write!(
                body,
                "<tr><td>{}</td><td>{}</td>",
                player_cell(p.osu_id, &p.username, p.staff),
                escape(p.discord_username.as_deref().unwrap_or(""))
            );
            for message in 1..=WINNING_MESSAGE {
                match found.get(&(p.osu_id, message)) {
                    Some(s) => {
                        let _ = write!(
                            body,
                            "<td><span class=\"found\">✓ #{}</span><br><span class=\"muted small\">{}</span></td>",
                            s.place,
                            when(s.solved_at)
                        );
                    }
                    None => body.push_str("<td class=\"muted\">–</td>"),
                }
            }
            let _ = write!(
                body,
                "<td>{}</td><td>{}</td><td>{}</td></tr>",
                p.attempts,
                when(p.first_at),
                when(p.last_at)
            );
        }
        body.push_str("</table></div>");
    }

    let _ = write!(body, "<h2>Son yanlış denemeler <span class=\"muted\">· en fazla {WRONG_GUESSES_SHOWN}</span></h2>");
    if wrong.is_empty() {
        body.push_str(r#"<p class="muted">Yok.</p>"#);
    } else {
        body.push_str("<div class=\"scroll\"><table><tr><th>Zaman (TSİ)</th><th>osu!</th><th>Deneme</th></tr>");
        for w in wrong {
            let _ = write!(
                body,
                "<tr><td>{}</td><td>{}</td><td class=\"guess\">{}</td></tr>",
                when(w.created_at),
                escape(&w.username),
                escape(&w.guess)
            );
        }
        body.push_str("</table></div>");
    }

    body.push_str(
        r#"<h2>Sıfırla</h2>
<form method="post" class="reset" onsubmit="return confirm('Tüm denemeler ve bulunan mesajlar silinecek, kazanan da. Geri alınamaz. Emin misin?')">
<button type="submit">Denemeleri sıfırla</button>
<span class="muted">Bütün denemeleri ve bulunan mesajları siler; sıralamalar ve kazanan da sıfırlanır.</span>
</form>"#,
    );

    format!(
        r#"<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>ARG sonuçları</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{ margin: 0 auto; max-width: 960px; padding: 24px 16px; background: #14161a; color: #e7e9ee; font: 14px/1.5 system-ui, sans-serif; }}
  h1 {{ font-size: 22px; margin: 0 0 12px; }}
  h2 {{ font-size: 16px; margin: 28px 0 8px; }}
  a {{ color: #8ab4ff; }}
  table {{ width: 100%; border-collapse: collapse; }}
  th, td {{ padding: 6px 8px; border-bottom: 1px solid #2a2e36; text-align: left; vertical-align: top; }}
  th {{ color: #9aa0ab; font-weight: 500; }}
  td {{ font-variant-numeric: tabular-nums; }}
  .guess {{ white-space: pre-wrap; word-break: break-word; }}
  .muted {{ color: #9aa0ab; font-weight: 400; }}
  .winner {{ padding: 10px 12px; border-radius: 8px; background: rgba(255, 196, 0, 0.12); color: #ffd666; font-size: 16px; }}
  .warn {{ padding: 10px 12px; border-radius: 8px; background: rgba(243, 135, 126, 0.12); color: #f3877e; }}
  .notice {{ padding: 10px 12px; border-radius: 8px; background: rgba(126, 224, 161, 0.12); color: #7ee0a1; }}
  .reset {{ display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }}
  .reset button {{ padding: 8px 14px; border: 1px solid #f3877e; border-radius: 8px; background: rgba(243, 135, 126, 0.12); color: #f3877e; font: inherit; cursor: pointer; }}
  .reset button:hover {{ background: rgba(243, 135, 126, 0.22); }}
  .tag {{ padding: 0 6px; border-radius: 4px; background: #2a2e36; color: #9aa0ab; font-size: 12px; }}
  .scroll {{ overflow-x: auto; }}
  .found {{ color: #7ee0a1; white-space: nowrap; }}
  .small {{ font-size: 12px; }}
</style>
</head>
<body>
<h1>ARG sonuçları</h1>
{body}
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ignores_case_and_spacing() {
        assert_eq!(
            normalize("  Kupa   SENİN\n olsun "),
            normalize("kupa senin olsun")
        );
        assert_eq!(normalize("SENIN"), normalize("senın"));
        assert_ne!(normalize("kupa"), normalize("kupalar"));
    }

    #[test]
    fn any_message_matches_in_any_order() {
        let messages = [
            ArgMessage {
                answer: "bir",
                reply: "1",
            },
            ArgMessage {
                answer: "iki kelime",
                reply: "2",
            },
            ArgMessage {
                answer: "üç",
                reply: "3",
            },
        ];
        assert_eq!(matching(&messages, "ÜÇ"), Some(3));
        assert_eq!(matching(&messages, " iki   KELİME "), Some(2));
        assert_eq!(matching(&messages, "bir"), Some(1));
        assert_eq!(matching(&messages, "dört"), None);
    }

    #[test]
    fn escape_neutralizes_markup() {
        assert_eq!(
            escape(r#"<script>"a" & 'b'</script>"#),
            "&lt;script&gt;&quot;a&quot; &amp; &#39;b&#39;&lt;/script&gt;"
        );
    }
}
