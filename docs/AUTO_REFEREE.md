# The auto referee

The site's Bancho bots can referee a match on their own. A bot opens the osu! room, invites the players and waits for them. For a bracket match it then runs the warmups, the roll, the bans and the picks. It starts each map, reads the scores from osu!, and concludes the match when it's over. Referees watch from the referee page and can take over at any moment.

This guide is for the tournament committee, hosts, referees and players.

**How to read it.** The site and the bot speak Turkish. Every Turkish text appears in quotes with its English translation after it in italics, like this: "Otomatik hakem" *(Auto referee)*. [Appendix A](#appendix-a-every-message-in-turkish-and-english) lists every message the system sends, in both languages.

---

## For the review committee

### What the tool does without a person

| Action | Effect | Reversible? |
|---|---|---|
| Opens a match's osu! room and invites the players | A room exists on osu!. | The room is closed when the match ends, or a referee closes it. |
| Moves players to their slot and team colour, kicks strangers | Room only. | Yes (slot picker, `!mp` commands in manual mode). |
| Gives a team the room's host for its warmup | Room only. | Yes. |
| Records bans and picks the players confirmed | Saved on the site. | Yes: a referee takes them back in manual mode. |
| Sets maps and mods, starts maps, aborts a map on `-abort` or an early disconnect | Room only. | n/a |
| Reads the scores from osu!'s match record and saves the running score | Saved on the site. | Yes: "Sayma" *(don't count)*, a referee's award, or saving the score by hand. |
| **Forfeits** a bracket team that isn't complete 10 minutes after the match time | Match concluded as W / FF; the other team advances. | A **host** reopens the match. |
| **Removes** an incomplete team from its qualifier lobby 10 minutes after the time | The team can sign up for another lobby that hasn't started. | A host puts the team back. |
| **Concludes** the match when it's decided | Results are saved, the teams advance, Discord posts the result. | A host reopens the match. |
| Sends Discord messages and site notifications | See Appendix A. | n/a |

### What it never does

- It never bans or edits a player, a team, a stage or a map pool.
- It never deletes a match or a result.
- It never acts on a match a referee has switched to manual (except answering `-rapor`, `-durdur` and `-davet`).
- It never decides a case it isn't sure about: if neither side is complete, if the stage has no best of, if the pick order runs out, if the score is impossible, it stops and calls the staff (see [When the bot stops by itself](#when-the-bot-stops-by-itself)).

### Safeguards

- **Off by default where it matters.** Rooms only open on their own after a host turns on "Odaları otomatik aç" *(Open rooms automatically)*.
- **A person can always take over.** Every match has an "Otomatik hakem" *(Auto referee)* switch on its referee page. Turning it off stops the bot at once; turning it back on resumes from the same step.
- **Players can stop it.** Any player can type `-durdur` *(stop)* to put the match in manual mode and alert every host and referee.
- **No conflicting clicks.** While the bot runs a match, the referee page disables the controls that would fight it.
- **The score comes from osu!.** After every map, osu!'s own match record replaces what the chat said.
- **Everything is logged.** Each decision is written to the match's "Otomatik hakem" log tab on the referee page, with time and reason.
- **It survives restarts.** Its progress is saved after every step; after a server restart it rejoins the room and continues.
- **Flood limits.** `-rapor` and `-durdur` work once a minute per player, `-davet` once every 20 seconds.

### Manual check: what to test

Each line is one scenario to run on a test match (a test server with dummy accounts, or a real room with staff playing). The bot's messages are in [Appendix A](#appendix-a-every-message-in-turkish-and-english).

| # | Do this | Expect |
|---|---|---|
| 1 | Leave "Odaları otomatik aç" off; give a match a time 5 minutes from now. | No room opens, no Discord reminder. |
| 2 | Turn it on. | Within 15 s the room opens, every player is invited, Discord posts the room and the bot's name. |
| 3 | A missing player DMs the room's bot `-davet`. | They're invited again and the bot replies "Davet gönderildi" *(Invite sent)*. |
| 4 | Let the match time pass with one player missing. | The bot re-invites them and announces the 10-minute wait. |
| 5 | Keep a bracket team incomplete for 10 more minutes. | The incomplete team forfeits; both teams get a notification, the absent one a DM. |
| 6 | Same in a qualifier lobby. | The incomplete team is kicked and removed from the lobby; its players get a notification and a DM; the lobby goes on. |
| 7 | Bracket: everyone joins. | Team 1 gets the host for a warmup; a pool map or a map over 4 minutes is refused; `-warmup-gec` skips. |
| 8 | Both teams `!roll`. | Only each team's first roll counts; the higher one is asked to ban or pick first per the stage's order. |
| 9 | The team types a slot (`hd1`), then again. | Asked to confirm, then the pick is recorded and the map opens with its mods. |
| 10 | Try a banned map, an already played map, TB, or the same mod twice in a row (not NM). | Each is refused with a reason. |
| 11 | On an FM map, one player without NoFail readies. | The bot names them and asks for NF before starting. |
| 12 | `-abort` within 30 s; then again after 30 s; then a third time. | First aborts ("1 hakkınız kaldı" *(1 left)*); the late one is refused; once 2 are used, refused. |
| 13 | A player drops in the first 30 s. | The map is aborted on their team's account. |
| 14 | Finish a map. | 3 s later the score is confirmed from osu! and posted; the next turn is asked. |
| 15 | `-rapor <text>`. | Every host and referee gets a notification with the text and a ping in the Discord referee channel. |
| 16 | `-durdur`. | The match goes to manual; staff are notified and pinged. Switching back to auto resumes at the same step. |
| 17 | Switch the match to manual on the referee page, act by hand, switch back. | The bot does nothing while manual, then resumes from the current state. |
| 18 | Play to a win (or the tiebreaker, then a win). | The bot announces the winner, concludes the match, the room closes 30 s later. |
| 19 | Restart the server mid-match. | The bot rejoins and continues the same step. |

---

## At a glance

| When | What happens |
|---|---|
| **20 min before** | Discord pings the players in the match channel: the bot will invite them in 10 minutes. |
| **10 min before** | The bot opens the room, sets it up, invites everyone and posts the room and the bot's name in Discord. |
| **Match time** | If anyone is missing, the bot invites them again and says the 10-minute wait has started. If everyone is in, the match starts. If everyone is in **and ready** earlier, it starts early. |
| **Match time + 10 min** | Anyone still incomplete is out. In a bracket match the missing team forfeits. In a qualifier lobby the incomplete teams are removed from the lobby. |
| **Bracket match** | Warmups → roll → bans and picks → maps, until a team wins (tiebreaker if needed). |
| **Qualifier lobby** | The pool's maps one by one, in pool order. |
| **After every map** | 3 seconds later the bot reads osu!'s record of the room. That record is the source of truth. |
| **End** | The bot announces the result, concludes the match on the site and closes the room 30 seconds later. |

The 20- and 10-minute steps only happen while "Odaları otomatik aç" *(Open rooms automatically)* is on.

---

## The two switches

| Switch | Where | Who | Default | What it does |
|---|---|---|---|---|
| "Odaları otomatik aç" *(Open rooms automatically)* | Bancho bots page, `/admin/bancho` | Hosts | **Off** | Tournament-wide. On: the bot opens the room of every match in auto mode 10 minutes before its time, and the 20-minute Discord reminder goes out. Off: referees open the rooms, and no reminder is sent. |
| "Otomatik hakem" *(Auto referee)* | Referee page, room bar | Referees, hosts | **On** | Per match. On: the bot referees the match. Off: manual control, and the bot only answers `-rapor`, `-durdur` and `-davet`. Turning it back on makes the bot **continue where it left off**. |

With the global switch off and a match's switch on, the bot still referees the match once a referee opens its room with "Oda oluştur" *(Create room)*.

---


## The match, step by step

### 1. Opening the room (both kinds)

- 20 minutes before, the match's Discord channel gets the reminder: "Maçınız 20 dakika sonra başlayacak…" *(Your match starts in 20 minutes…)*.
- 10 minutes before, the bot creates the room. It sets the team mode (Team Vs for a bracket match, Head to Head for a lobby) and the score mode, locks the slots and invites every player. Discord gets "Maç odası kuruldu…" *(The match room is set up…)* with the room link and the bot's name.
- Players who join are moved to their slot and team colour automatically. Strangers are kicked.

### 2. Waiting for the players (both kinds)

- **Everyone is in at match time:** the match starts.
- **Everyone is in and ready before match time:** the match starts early.
- **Someone is missing at match time:** the bot re-invites them and says "Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takım maçı kaybeder." *(It's match time. Waiting 10 minutes for the missing players; a team that doesn't come loses the match.)*
- **10 minutes after match time:**
  - **Bracket:** a team missing anyone forfeits, and the other team wins (W / FF). Both teams' players get a site notification, and the absent team also gets a Discord DM.
  - **Qualifier:** every incomplete team is kicked and removed from the lobby. Its players get a notification and a DM, and can sign up for another lobby that hasn't started. The lobby goes on with the complete teams.
  - **Nobody is complete:** the bot stops and hands the match to the staff (see [When the bot stops by itself](#when-the-bot-stops-by-itself)).

If a referee opened the room late, the 10 minutes count from when the room was opened, not from the match time.

### 3a. Qualifier lobby

1. The bot opens the pool's maps **one by one, in pool order** (the tiebreaker is skipped).
2. For each map it sets the map and mods, starts a **3-minute** timer and says "Sıradaki map: NM1. Herkes hazır olduğunda ya da sayaç sıfırlandığında map başlayacak." *(Next map: NM1. The map starts when everyone is ready or when the timer reaches zero.)*
3. The map starts with a 10-second countdown when **everyone is ready**, or when the **timer runs out**.
4. After the **first** map, a player who drops doesn't stop anything: the lobby goes on and their score just isn't counted.
5. When every map has been played, the lobby is concluded.

### 3b. Bracket match

**Warmups**

1. Team 1 (blue) gets the host first: its first player is made the room's host. The bot says "<takım> takımına warmup açmaları için host yetkisi verildi…" *(<team> has been given host to open a warmup…)* and starts a **3-minute** timer to open a warmup.
2. The team picks a map and starts it themselves.
   - A map **from the pool** is refused, and aborted if started.
   - A map **longer than 4 minutes** (by osu!'s record) is refused, and aborted if started.
3. When the warmup finishes, or the 3 minutes run out without one, the host passes to team 2.
4. A team can type `-warmup-gec` *(skip warmup)* to skip its warmup, even before its turn.

**Roll**

1. "Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var." *(Warmups are over. Use !roll to decide the pick and ban order; you have 2 minutes.)*
2. Only each team's **first** `!roll` counts, and it must be a plain `!roll` (1–100). Anything over 100 is ignored.
3. Higher roll wins. A tie means both teams roll again. A team that doesn't roll in 2 minutes loses the roll. If neither team rolls, the bot picks one at random.
4. The roll winner is **team "1" of the stage's draft order**.

**Bans and picks**

1. The bot asks the team whose turn it is: "<takım> takımı pick sırası, seçtiğiniz mapin slotunu söyleyin." *(<team>'s turn to pick: say the slot of the map you choose.)*
2. Either player of that team types the slot, e.g. `hd1` (case and spaces don't matter).
3. The bot asks them to confirm: "Pick için HD1 seçtiğinizi doğrulamak için tekrar yazın." *(Type it again to confirm HD1 as your pick.)* Typing the same slot again confirms it.
4. A ban is recorded. A pick is recorded and the map opens with its mods.
5. The bot **refuses**:
   - the tiebreaker
   - a map already banned
   - a map already picked or played
   - a pick in the **same mod category as the team's previous pick** (NM is the exception)

**Each map**

- The map and mods go on, with a **2-minute** timer.
- It starts with a 10-second countdown when **all four players are in and ready**, or when the timer runs out.
- **FM and TB maps:** before starting, the bot reads everyone's mods. A player without **NoFail** gets "<oyuncu>: NoFail (NF) modunu seçmediniz, lütfen NF açın." *(<player>: you haven't selected NoFail (NF), please turn NF on.)* If the timer has already run out, they get 30 more seconds. After that the map starts anyway.
- After each map the bot posts the score, then asks for the next ban or pick.

**Tiebreaker and the end**

- When the score reaches the tiebreaker (e.g. 4–4 in a Bo9), the bot says "Tiebreaker!" and opens the TB map.
- When a team reaches the winning number of maps, the bot announces "<takım> maçı kazandı!" *(<team> won the match!)* with the score.

### 4. After every map (both kinds)

- **3 seconds** after the map ends, the bot reads osu!'s record of the room (the mp link). osu!'s scores replace what the chat said.
- If osu! doesn't have the map yet, the bot tries again every 3 seconds, up to 5 times. After that the chat's scores stand, and the log says so.

### 5. The end (both kinds)

1. "Maç bitti, oda 30 saniye sonra kapanacak." *(The match is over; the room closes in 30 seconds.)*, with a 30-second timer in the room.
2. The match is concluded on the site. A bracket match uses the bot's counted score, the teams move on in the bracket, and Discord gets the results.
3. 30 seconds later the room is closed.

---

## Player commands

Typed in the **match room**, unless the table says otherwise.

| Command | Meaning | What it does | Limits | Works in manual mode |
|---|---|---|---|---|
| `-davet` | *invite* | **In a DM to the room's bot:** sends a new room invite. | Once every 20 s | Yes |
| `-abort` | *abort* | Aborts the map being played. The bot says "Abort kullanıldı. N hakkınız kaldı" *(Abort used. You have N left)*. | **2 per team** per match, and only in the **first 30 seconds** of a map. Later: "Maç başladıktan 30 saniye sonra abort kullanılamaz" *(Abort can't be used 30 seconds after the map has started)* | No |
| `-rapor <mesaj>` | *report <message>* | Sends the message to every host and referee: a site notification, plus a ping in the Discord referee channel. | Once a minute per player, 300 characters | Yes |
| `-durdur` | *stop* | **Emergency stop.** The match switches to **manual mode**, and every host and referee is notified and pinged in Discord. A referee switches it back to auto when it's sorted, and the bot continues where it stopped. | No use limit. Once a minute per player, so it can't be spammed. | Yes (it just alerts, since the match is already manual) |
| `-warmup-gec` | *skip warmup* | Skips the team's warmup. | Bracket warmups only | No |
| `hd1`, `nm2`, `fm1`… | *a pool slot* | Names the ban or pick when it's the team's turn. Typed twice to confirm. | Only the team whose turn it is | No |
| `!roll` | *osu!'s roll* | The team's first roll decides the pick/ban order. | 1–100 only | n/a |

**Disconnects:** a player who drops in the first 30 seconds of a map gets it aborted. That uses one of their team's 2 aborts. In a qualifier this only applies to the first map.

---

## Timings and numbers

| | |
|---|---|
| Room opens | 10 min before match time (only with "Odaları otomatik aç" on) |
| Discord reminder | 20 min before (only with "Odaları otomatik aç" on) |
| Wait for no-shows | 10 min after match time (or after the room opened, if later) |
| Warmup | 3 min to open one, maps up to 4 min |
| Roll | 2 min |
| Map ready timer | 3 min (qualifier), 2 min (bracket) |
| NoFail grace | 30 s extra, once |
| Start countdown | 10 s |
| `-abort` | 2 per team, within 30 s of the map starting |
| Score read from osu! | 3 s after the map, up to 5 tries |
| Room closes | 30 s after the match is over |

---

## When the bot stops by itself

When the bot can't continue safely, it **switches the match to manual**. It says "Otomatik hakem durdu, maçı bir hakem devralacak." *(The auto referee stopped; a referee will take over the match.)* in the room and notifies every host and referee: a site notification, plus a ping in the Discord referee channel with the reason.

| Reason, as the bot states it | In English | What to do |
|---|---|---|
| "10 dakika beklendi ama hiçbir takım tam kadro gelmedi." | *Waited 10 minutes, but no team arrived with a full roster.* | Decide with the hosts: forfeit, reschedule, or wait longer. |
| "Stage'in best of değeri yok, maçın ne zaman bittiği bilinmiyor." | *The stage has no best of, so it's unknown when the match ends.* | Set the stage's best of on `/admin/stages`, then switch back to auto. |
| "Stage'in pick sırası bitti ama maç bitmedi." | *The stage's pick order ran out, but the match isn't over.* | The draft order has no closing `*`. Fix the order, or finish the match by hand. |
| "Sıradaki ban/pick'in takımı belirlenemedi." | *Couldn't determine which team has the next ban/pick.* | Choose the team under "Sıradaki pick" *(Next pick)* in manual mode, then switch back to auto. |
| "Skor (a - b) BoN'u aşıyor." | *The score (a - b) exceeds BoN.* | The score is wrong. Fix it ("Sayma" *(don't count)*, a referee's award, or "Maç linkinden eşitle" *(sync from the match link)*). |
| "Skor tiebreaker'a geldi ama havuzda tiebreaker yok." | *The score reached the tiebreaker, but the pool has no tiebreaker.* | Add a TB map, or open a map by hand. |
| "Tiebreaker oynandı ama kazanan belli değil." | *The tiebreaker was played, but there's no winner.* | Check the TB's result and fix the score. |
| "Maçın iki takımı belli değil." | *The match's two teams aren't known.* | Set both teams on the bracket. |

A player's `-durdur` also puts the match in manual (see above).

---

## Control surfaces (staff)

### Referee page (`/admin/referee/<match>`)

**Room bar.** The "Otomatik hakem" *(Auto referee)* switch flips between auto and manual. Switching it back on continues from the same step: the bot restarts that step's timer and catches up with anything a referee did meanwhile, such as bans or picks marked by hand.

**Status line** (under the room bar). One line shows:
- what the bot is waiting for, with a countdown: match time, the no-show cutoff, the warmup, the roll, the map timer
- whose ban or pick it is
- each team's remaining `abort` uses (crossed out when spent)
- before the room exists: when the bot will open it, or that automatic room opening is off
- in manual mode: "Manuel kontrol" *(Manual control)*, and what the bot will continue with

**Commands while in auto.** Controls that referee the match stay visible but are **disabled**. A line at the top says "Otomatik hakem maçı yönetiyor" *(The auto referee is running the match)*, with a "Manuel kontrole geç" *(Switch to manual control)* button.

| Locked in auto mode | Still open in auto mode |
|---|---|
| Room commands: "Başlat" *(Start)*, "İptal et" *(Abort)*, "Zamanlayıcı" *(Timer)*, "Durdur" *(Stop timer)*, "Odayı yenile" *(Refresh room)*, "Hepsini davet et" *(Invite all)*, "Yerleri düzelt" *(Fix slots)*, "Kilitle" *(Lock)*, "Kurulumu tekrarla" *(Redo setup)* | The "Otomatik hakem" *(Auto referee)* switch |
| Clicking pool maps, ban/pick buttons and taking them back | Chat (plain messages; `!mp` lines wait for manual mode) |
| "Sıradaki pick" *(Next pick)* team choice, "Skoru odaya yaz" *(Post score to room)* | Pinging a player on Discord |
| Invites, kicks, moving players between slots | Adding the streamer as a room referee |
| "Sayma" *(Don't count)*, "Hakem kararıyla map ver" *(Award a map by referee decision)*, "Maç linkinden eşitle" *(Sync from the match link)*, "Skoru kaydet" *(Save score)* | Opening a room: "Oda oluştur" *(Create room)*, "Yeni oda aç" *(Open new room)*, adding a link |
| "Maçı bitir" *(Conclude match)*, "Hükmen bitir…" *(End by forfeit…)*, "Odayı kapat" *(Close room)* | Room options (score type, NF, placing, kicking) |

The lock is on the page only. The server doesn't refuse these actions if they're called directly.

**Chat column, "Otomatik hakem" tab** *(Auto referee)*. The bot's log: every decision with its time and level. The levels are "bilgi" *(info)*, "uyarı" *(warning)* and "hata" *(error)*.
- **bilgi:** each step: room opened, roll results, bans and picks confirmed, maps opened, scores confirmed from osu!, switches to manual or auto and who flipped them.
- **uyarı:** reports, `-durdur`, no-shows, a map started without NF, osu! scores that couldn't be read.
- **hata:** handovers and failed actions.

New warnings and errors put a badge on the tab. Every log line is translated in [Appendix A](#a6-auto-referee-log-referee-page).

### Bancho bots page (`/admin/bancho`)

"Odaları otomatik aç" *(Open rooms automatically)*: the tournament-wide switch for rooms opening themselves. It asks for confirmation before turning on.

### Notifications (the bell)

| Notification | Who gets it | Means |
|---|---|---|
| "Oyuncu raporu" *(Player report)* | Hosts, referees | A player's `-rapor`, with their message. Links to the referee page. |
| "Acil durdurma" *(Emergency stop)* | Hosts, referees | A player typed `-durdur`; the match is now manual. |
| "Otomatik hakem durdu" *(Auto referee stopped)* | Hosts, referees | The bot handed the match over, with the reason. |
| "Maç hükmen bitti" *(Match ended by forfeit)* | Both teams' players | A bracket match was forfeited for a team that didn't show up. |
| "Eleme lobisinden çıkarıldınız" *(You were removed from the qualifier lobby)* | The removed team's players | Their team was taken out of its lobby for not showing up, so they can join another. |

### Discord

| Message | Where |
|---|---|
| 20-minute reminder (pings the players) | Match channel |
| "Maç odası kuruldu…" *(The match room is set up…)* with the room link and the bot's name | Match channel |
| `-rapor`, `-durdur` and handover alerts (ping the hosts and the match's referee, or every referee if the match has none) | Referee channel |
| No-show notice | DM to the absent team's players; for a lobby, also a line in the lobby's channel |
| Forfeit / results | As usual: match channel and results channel |

---

## FAQ

**A team picked the wrong map. How do I undo it?** Switch to manual, take the pick back in the pool (or fix the room), then switch back to auto. The bot reads the current bans and picks and asks for the right turn.

**The bot is waiting for a pick but nobody answers.** Picks have no timer. Ping the team on Discord, or go to manual and mark the pick yourself.

**A player's game crashed after the 30 seconds.** That's not an auto-abort. Use manual mode: abort, then switch back. The bot reopens the same map.

**The bot restarted (server update) in the middle of a match.** Its state is saved after every step. It rejoins the room and continues from the same step, with that step's timer restarted.

**Can a referee play along without switching to manual?** Yes, for chatting, pinging and fixing rooms. Anything that changes the match (maps, picks, scores, concluding) needs manual mode first.

**Will the bot open rooms during testing?** Only with "Odaları otomatik aç" on. Leave it off until the tournament is live.

---

## Appendix A: every message, in Turkish and English

`<takım>` is a team name, `<oyuncu>` a player's name, `<slot>` a pool slot such as HD1, `N` a number. Messages in the osu! room go out exactly as written (the bot also sends the usual `!mp` commands, which aren't listed).

### A.1 In the osu! room

**Before the match**

| Turkish | English | When |
|---|---|---|
| Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takım maçı kaybeder. | It's match time. Waiting 10 minutes for the missing players; a team that doesn't come loses the match. | Match time, someone missing |
| <takım> takımı 10 dakika içinde tam kadro gelmedi. Maç hükmen <takım> takımına verildi. | <team> didn't arrive with a full roster within 10 minutes. The match is awarded to <team> by forfeit. | Bracket no-show |
| <takımlar> tam kadro gelmediği için lobiden çıkarıldı. | <teams> were removed from the lobby for not arriving with a full roster. | Qualifier no-show |
| Herkes burada, maç başlıyor! Mapler sırayla açılacak, iyi şanslar. | Everyone's here, the match is starting! The maps will open in order, good luck. | Qualifier starts |

**Warmups**

| Turkish | English | When |
|---|---|---|
| <takım> takımına warmup açmaları için host yetkisi verildi. Warmup mapleri 4 dakikayı geçmemelidir. Eğer warmup oynamak istemiyorlarsa -warmup-gec yazabilirler. | <team> has been given host to open a warmup. Warmup maps must not exceed 4 minutes. If they don't want to play a warmup, they can type -warmup-gec. | A team's warmup turn |
| <takım> warmupı geçti. | <team> skipped their warmup. | `-warmup-gec` on their turn |
| <takım> sırası gelince warmupı geçecek. | <team> will skip their warmup when their turn comes. | `-warmup-gec` before their turn |
| <takım> takımının warmup süresi doldu. | <team>'s warmup time ran out. | 3 minutes without a warmup |
| Havuzdaki mapler warmup olarak oynanamaz, başka bir map seçin. | Pool maps can't be played as a warmup; choose another map. | A pool map is chosen or started |
| Bu map M:SS uzunluğunda, warmup 4 dakikayı geçmemeli. Başka bir map seçin. | This map is M:SS long; a warmup must not exceed 4 minutes. Choose another map. | A long map is chosen |
| Bu map 4 dakikadan uzun, başka bir warmup seçin. | This map is longer than 4 minutes; choose another warmup. | A long map is started (and aborted) |
| Warmup 4 dakikayı geçti, sonlandırıldı. | The warmup went past 4 minutes and was ended. | A warmup runs too long |

**Roll**

| Turkish | English | When |
|---|---|---|
| Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var. | Warmups are over. Use !roll to decide the pick and ban order; you have 2 minutes. | Roll starts |
| Pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var. | Use !roll to decide the pick and ban order; you have 2 minutes. | Roll resumes after manual mode |
| <oyuncu>: sadece !roll yazın (1-100), bu roll sayılmadı. | <player>: type just !roll (1-100); this roll didn't count. | A roll over 100 |
| Berabere (a - b)! İki takım da tekrar !roll yazsın. | A tie (a - b)! Both teams roll again. | Equal rolls |
| <takım> rollü kazandı (a - b). | <team> won the roll (a - b). | Both rolled |
| <takım> rollü kazandı (rakibi roll atmadı). | <team> won the roll (their opponent didn't roll). | One team didn't roll |
| <takım> rollü kazandı (iki takım da roll atmadı, bot rastgele seçti). | <team> won the roll (neither team rolled; the bot chose at random). | Neither team rolled |

**Bans and picks**

| Turkish | English | When |
|---|---|---|
| <takım> takımı pick sırası, seçtiğiniz mapin slotunu söyleyin. | <team>'s turn to pick: say the slot of the map you choose. | Asking for a pick |
| <takım> takımı ban sırası, seçtiğiniz mapin slotunu söyleyin. | <team>'s turn to ban: say the slot of the map you choose. | Asking for a ban |
| Pick için <slot> seçtiğinizi doğrulamak için tekrar yazın. | Type it again to confirm <slot> as your pick. | First answer |
| Ban için <slot> seçtiğinizi doğrulamak için tekrar yazın. | Type it again to confirm <slot> as your ban. | First answer |
| <takım> takımı <slot> mapini seçti. | <team> picked <slot>. | Pick confirmed |
| <takım> takımı <slot> mapini banladı. | <team> banned <slot>. | Ban confirmed |
| <slot> tiebreaker, pick edilemez. / ban edilemez. | <slot> is the tiebreaker; it can't be picked. / banned. | TB named |
| <slot> banlandı, başka bir map seçin. | <slot> is banned; choose another map. | A banned map named |
| <slot> zaten oynandı, başka bir map seçin. | <slot> has already been played; choose another map. | A picked or played map named |
| Aynı moddan (<mod>) arka arkaya pick yapamazsınız (NM hariç), başka bir mod seçin. | You can't pick the same mod (<mod>) twice in a row (except NM); choose another mod. | Same category twice |

**Maps**

| Turkish | English | When |
|---|---|---|
| Sıradaki map: <slot>. Herkes hazır olduğunda ya da sayaç sıfırlandığında map başlayacak. | Next map: <slot>. The map starts when everyone is ready or when the timer reaches zero. | A map opens |
| <oyuncular>: NoFail (NF) modunu seçmediniz, lütfen NF açın. | <players>: you haven't selected NoFail (NF), please turn NF on. | FM/TB, NF missing |
| Süre doldu, map NoFail'siz oyuncularla başlıyor. | Time's up; the map starts with players who don't have NoFail. | NF still missing after the grace |
| Abort kullanıldı. N hakkınız kaldı | Abort used. You have N left | `-abort` accepted |
| <oyuncu> oyundan düştü. Abort kullanıldı. N hakkınız kaldı | <player> disconnected. Abort used. You have N left | Early disconnect |
| Maç başladıktan 30 saniye sonra abort kullanılamaz | Abort can't be used 30 seconds after the map has started | Late `-abort` |
| <oyuncu>: takımınızın abort hakkı kalmadı. | <player>: your team has no aborts left. | `-abort` with none left |
| <oyuncu>: şu an oynanan bir map yok. | <player>: no map is being played right now. | `-abort` between maps |
| Skor: <takım> a - b <takım> | Score: <team> a - b <team> | After each bracket map |
| Tiebreaker! | Tiebreaker! | Score reaches the tiebreaker |

**The end**

| Turkish | English | When |
|---|---|---|
| <takım> maçı kazandı! Skor: <takım> a - b <takım> | <team> won the match! Score: <team> a - b <team> | Bracket match decided |
| Bütün mapler oynandı, katıldığınız için teşekkürler! | All maps have been played, thanks for taking part! | Qualifier finished |
| Maç bitti, oda 30 saniye sonra kapanacak. | The match is over; the room closes in 30 seconds. | End |

**Player commands and stops**

| Turkish | English | When |
|---|---|---|
| <oyuncu>: raporun hostlara ve hakemlere iletildi. | <player>: your report was passed on to the hosts and referees. | `-rapor` accepted |
| <oyuncu>: kullanım: -rapor <mesajınız> | <player>: usage: -rapor <your message> | `-rapor` with no text |
| <oyuncu>: bir dakikada bir rapor gönderebilirsiniz. | <player>: you can send one report per minute. | `-rapor` too often |
| Durdur komutu kullanıldı. Bu komut sadece acil durumlarda kullanılmalıdır. Hostlar bilgilendirildi, eğer bu komut kötü niyetle kullanıldıysa -rapor komutuyla bildiriniz | The stop command was used. This command should only be used in emergencies. The hosts have been informed; if it was used in bad faith, report it with the -rapor command | `-durdur` |
| Otomatik hakem durdu, maçı bir hakem devralacak. | The auto referee stopped; a referee will take over the match. | The bot hands over |

### A.2 Private messages from the bot

| Turkish | English | When |
|---|---|---|
| Davet gönderildi: #mp_N | Invite sent: #mp_N | `-davet` accepted |
| Şu an seni bekleyen, bu botun yönettiği bir maç odası yok. | There's no match room run by this bot waiting for you right now. | `-davet` with no match |

### A.3 Discord

Embeds (the referee channel alerts and the DMs) are shown as *title — text*. `<maç>` is the match's title, e.g. "L16: Team A vs Team B".

| Turkish | English | Where |
|---|---|---|
| ⏰ <oyuncular> Maçınız 20 dakika sonra başlayacak. Bot, oda davetlerini 10 dakika sonra yollayacak. Oyundaki özel mesajlarınızın bottan mesaj almaya hazır olduğundan emin olun. Eğer bottan davet almazsanız odanızın botuna özelden `-davet` yazarak tekrar davet yollamasını sağlayabilirsiniz. Bir sorun yaşarsanız hostlardan yardım alın. | ⏰ <players> Your match starts in 20 minutes. The bot will send the room invites in 10 minutes. Make sure your in-game private messages can receive messages from the bot. If you don't get an invite, DM your room's bot `-davet` to have it send one again. If you have a problem, ask the hosts for help. | Match channel |
| 🎮 Maç odası kuruldu: <link> Odanızın botu **<bot>**. Eğer davetiye almadıysanız oyundan bu bota DM atıp `-davet` yazın. | 🎮 The match room is set up: <link> Your room's bot is **<bot>**. If you didn't get an invite, DM this bot in game and type `-davet`. | Match channel |
| 📣 Oyuncu raporu: <maç> — <oyuncu> (<takım>): <mesaj> | 📣 Player report: <match> — <player> (<team>): <message> | Referee channel |
| 🛑 -durdur kullanıldı: <maç> — <oyuncu> (<takım>) acil durum için -durdur yazdı: maç manuel kontrole geçti, bir hakemin bakması gerekiyor. | 🛑 -durdur used: <match> — <player> (<team>) typed -durdur for an emergency: the match went to manual control and needs a referee to look at it. | Referee channel |
| ⚠️ Otomatik hakem durdu: <maç> — <neden> Maç manuel kontrole geçti, bir hakemin devralması gerekiyor. | ⚠️ Auto referee stopped: <match> — <reason> The match went to manual control; a referee needs to take over. | Referee channel |
| ❌ Maç hükmen kaybedildi — Takımın **<maç>** maçına 10 dakika içinde tam kadro gelmediği için maç hükmen kaybedildi. | ❌ Match lost by forfeit — Your team didn't arrive at **<match>** with a full roster within 10 minutes, so the match was lost by forfeit. | DM, absent team |
| ❌ Eleme lobisinden çıkarıldınız — Takımın **<lobi>** lobisine 10 dakika içinde tam kadro gelmediği için lobiden çıkarıldı. Başlamamış başka bir eleme lobisine kaydolabilirsiniz: <link> | ❌ You were removed from the qualifier lobby — Your team didn't arrive at **<lobby>** with a full roster within 10 minutes, so it was removed from the lobby. You can sign up for another qualifier lobby that hasn't started: <link> | DM, removed team |
| ℹ️ **<takım>** tam kadro gelmediği için lobiden çıkarıldı. | ℹ️ **<team>** was removed from the lobby for not arriving with a full roster. | Lobby channel |

### A.4 Site notifications (the bell)

| Turkish title | English title | Turkish text | English text |
|---|---|---|---|
| Oyuncu raporu | Player report | "<mesaj>" — <oyuncu> (<takım>) odadan -rapor ile hakemlere ve hostlara yazdı. | "<message>" — <player> (<team>) wrote to the referees and hosts from the room with -rapor. |
| Acil durdurma | Emergency stop | <oyuncu> (<takım>) -durdur yazdı: maç manuel kontrole geçti, bir hakemin bakması gerekiyor. | <player> (<team>) typed -durdur: the match went to manual control and needs a referee to look at it. |
| Otomatik hakem durdu | Auto referee stopped | Maç manuel kontrole geçti, bir hakemin devralması gerekiyor. Neden: <neden> | The match went to manual control; a referee needs to take over. Reason: <reason> |
| Maç hükmen bitti | Match ended by forfeit | <takım> maç saatinden sonraki 10 dakika içinde tam kadro gelmedi, maç hükmen sonuçlandı. | <team> didn't arrive with a full roster within 10 minutes of the match time; the match was decided by forfeit. |
| Eleme lobisinden çıkarıldınız | You were removed from the qualifier lobby | Takımın <lobi> için 10 dakika içinde tam kadro gelmediğinden lobiden çıkarıldı. Başlamamış başka bir lobiye kaydolabilirsin. | Your team didn't arrive at <lobby> with a full roster within 10 minutes, so it was removed from the lobby. You can sign up for another lobby that hasn't started. |

### A.5 Referee and bots pages

| Turkish | English |
|---|---|
| Otomatik hakem | Auto referee (the switch, and the log tab) |
| Bot maçı kendisi yönetiyor. Kapatınca manuel kontrole geçer; açınca kaldığı yerden devam eder. | The bot is running the match itself. Turning it off switches to manual control; turning it on continues where it left off. (switch tooltip, on) |
| Manuel kontrol. Açınca otomatik hakem kaldığı yerden devam eder. | Manual control. Turning it on makes the auto referee continue where it left off. (switch tooltip, off) |
| Otomatik hakem odayı açıp oyuncuları davet edecek | The auto referee will open the room and invite the players |
| Otomatik oda açma kapalı (Bancho botları sayfası): odayı bir hakem açınca otomatik hakem maçı yönetir. | Automatic room opening is off (Bancho bots page): once a referee opens the room, the auto referee runs the match. |
| Maç saati bekleniyor, herkes hazırsa erken başlar | Waiting for the match time; starts early if everyone is ready |
| Oyuncular bekleniyor; gelmeyen takım elenir | Waiting for the players; a team that doesn't come is out |
| Warmup: <takım> hostta | Warmup: <team> has host |
| !roll bekleniyor | Waiting for !roll |
| <takım> pick/ban sırası, slotu yazmaları bekleniyor | <team>'s pick/ban: waiting for them to type the slot |
| <slot> açık, herkesin hazır olması bekleniyor | <slot> is on; waiting for everyone to be ready |
| <slot> oynanıyor | <slot> is being played |
| <slot> skorları osu!'dan okunuyor | Reading <slot>'s scores from osu! |
| Sıradaki adım hazırlanıyor | Preparing the next step |
| Maç bitti | The match is over |
| Manuel kontrol — Bot sadece -rapor, -durdur ve -davet'e cevap veriyor. Açınca devam eder: <adım> | Manual control — The bot only answers -rapor, -durdur and -davet. When switched on, it continues with: <step> |
| Otomatik hakem maçı yönetiyor. Komutlar, havuz ve skor araçları manuel kontrolde açılır; sohbet açık. | The auto referee is running the match. Commands, the pool and the score tools open in manual control; the chat stays open. |
| Manuel kontrole geç | Switch to manual control |
| Odada henüz map yok. Otomatik hakem sırası gelince açacak. | No map in the room yet. The auto referee will open one when it's time. |
| <bot> üzerinden yaz: mesaj (!mp manuel kontrolde) | Write through <bot>: messages (!mp in manual control) |
| Otomatik hakem maçı yönetiyor: !mp komutları manuel kontrolde gönderilir. | The auto referee is running the match: !mp commands are sent in manual control. (error) |
| Otomatik hakemin kaydı: salt okunur | The auto referee's log: read-only |
| Otomatik hakem bu maçta henüz bir şey yapmadı. Oda açınca, oyuncuları beklerken ve her adımda buraya yazar. | The auto referee hasn't done anything in this match yet. It writes here when it opens the room, while it waits for the players, and at every step. |
| bilgi / uyarı / hata | info / warning / error (log levels) |
| Odaları otomatik aç | Open rooms automatically (bots page switch) |
| Açık: otomatik hakemdeki her maçın odası maç saatinden 10 dakika önce açılır, oyuncular davet edilir; 20 dakika önce Discord'dan hatırlatılır. | On: the room of every match in auto mode opens 10 minutes before its time and the players are invited; they're reminded on Discord 20 minutes before. |
| Kapalı: odaları hakemler açar. Açılan odalarda otomatik hakem maçı yine yönetir. | Off: referees open the rooms. In rooms they open, the auto referee still runs the match. |
| Otomatik hakemdeki her maçın odası, maç saatinden 10 dakika önce botlarla açılacak ve oyunculara davet gidecek. Açılsın mı? | The room of every match in auto mode will be opened by the bots 10 minutes before its time, and the players will be invited. Turn it on? (confirmation) |

### A.6 Auto referee log (referee page)

| Level | Turkish | English |
|---|---|---|
| bilgi | Oda açıldı: #mp_N (<bot>), oyuncular davet edildi. | Room opened: #mp_N (<bot>), players invited. |
| hata | Oda açılamadı, 1 dakika sonra tekrar denenecek: <hata> | Couldn't open the room; trying again in 1 minute: <error> |
| bilgi | Maç saati geldi, eksikler tekrar davet edildi: <oyuncular> | Match time; the missing players were invited again: <players> |
| bilgi | <oyuncu> -davet ile tekrar davet edildi. | <player> was invited again with -davet. |
| uyarı | <takım> gelmedi: hükmen <takım> kazandı. | <team> didn't show: <team> wins by forfeit. |
| uyarı | Tam kadro gelmeyen takımlar lobiden çıkarıldı: <takımlar> | Teams without a full roster were removed from the lobby: <teams> |
| bilgi | Herkes odada, maç başlıyor. | Everyone's in the room; the match is starting. |
| bilgi | Warmup: host <oyuncu> (<takım>). | Warmup: host <player> (<team>). |
| bilgi | <takım> warmupı geçti. / <takım> -warmup-gec yazdı. | <team> skipped their warmup. / <team> typed -warmup-gec. |
| bilgi | <takım> warmup açmadı, süre doldu. | <team> didn't open a warmup; time ran out. |
| uyarı | Warmup 4,5 dakikayı geçtiği için iptal edildi. | The warmup was aborted for going past 4.5 minutes. |
| uyarı | Kurallara uymayan warmup iptal edildi. | A warmup breaking the rules was aborted. |
| bilgi | Roll bekleniyor. | Waiting for the roll. |
| bilgi | <oyuncu> (<takım>) N attı. | <player> (<team>) rolled N. |
| bilgi | Roll: <takım> kazandı (…). | Roll: <team> won (…). |
| bilgi | <takım> takımının pick/ban seçimi bekleniyor. | Waiting for <team>'s pick/ban. |
| bilgi | <takım> <slot> seçti / banladı (<oyuncu> onayladı). | <team> picked / banned <slot> (confirmed by <player>). |
| bilgi | <slot> açıldı. | <slot> opened. |
| bilgi | NF uyarısı: <oyuncular> | NF warning: <players> |
| uyarı | NF açmadan başladı: <oyuncular> | Started without NF: <players> |
| uyarı | Map başlamadı, tekrar deneniyor. | The map didn't start; trying again. |
| bilgi | <oyuncu> -abort kullandı (N hakkı kaldı). | <player> used -abort (N left). |
| bilgi | <oyuncu> düştüğü için map iptal edildi (N hakkı kaldı). | The map was aborted because <player> disconnected (N left). |
| bilgi | <oyuncu> düştü ama takımının abort hakkı kalmadı. | <player> disconnected, but their team has no aborts left. |
| bilgi | <oyuncu> düştü, maç onsuz devam ediyor. | <player> disconnected; the match continues without them. |
| bilgi | Map bitti, osu!'dan skorlar bekleniyor. | The map finished; waiting for the scores from osu!. |
| bilgi | Skorlar osu!'dan doğrulandı. | Scores confirmed from osu!. |
| uyarı | osu! skorları getiremedi, sohbetteki skorlarla devam ediliyor. | osu! couldn't provide the scores; continuing with the chat's scores. |
| bilgi | Skor eşit, tiebreaker açılıyor. | The score is tied; opening the tiebreaker. |
| bilgi | <takım> kazandı (a - b). / Bütün mapler oynandı. | <team> won (a - b). / All maps have been played. |
| uyarı | Rapor (<oyuncu>): <mesaj> | Report (<player>): <message> |
| uyarı | <oyuncu> -durdur yazdı: manuel kontrole geçildi. | <player> typed -durdur: switched to manual control. |
| uyarı | <oyuncu> -durdur yazdı (maç zaten manuel kontrolde). | <player> typed -durdur (the match was already in manual control). |
| bilgi | <kişi> otomatik hakemi açtı. | <person> turned the auto referee on. |
| bilgi | <kişi> manuel kontrole geçti, otomatik hakem durdu. | <person> switched to manual control; the auto referee stopped. |
| bilgi | Kaldığı yerden devam ediyor: <adım>. | Continuing where it left off: <step>. |
| hata | Manuel kontrole geçildi: <neden> | Switched to manual control: <reason> |
| hata | İşlem başarısız: <hata> | Action failed: <error> |
