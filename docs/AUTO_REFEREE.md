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
- It never decides a case it isn't sure about: if neither side is complete, if the stage has no best of, if the pick order runs out, if both teams let the same pick run out, if the score is impossible, it stops and calls the staff (see [When the bot stops by itself](#when-the-bot-stops-by-itself)).

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
| 7 | Bracket: everyone joins. | Team 1 gets the host for a warmup; a pool map (or another difficulty of one) or a map over 4 minutes is refused; `-warmup-gec` skips. |
| 7a | The host opens a warmup and all four ready up without starting it. | The bot says "Herkes hazır, warmup başlıyor." and starts it with a 10-second countdown. |
| 7b | `!mp unlock` the room from the referee page in manual mode, switch back to auto. | Within 30 s the bot locks it again. A player who changes slot or colour by themselves is moved back. |
| 8 | Both teams `!roll`. | Only each team's first roll counts; the higher one is asked to ban or pick first per the stage's order. |
| 9 | The team types a slot (`hd1`), then again. | Asked to confirm, then the pick is recorded and the map opens with its mods. |
| 10 | Try a banned map, an already played map, TB, or the same mod twice in a row (not NM). | Each is refused with a reason. |
| 10a | On a ban, wait 3 minutes without naming a slot. | Reminders at every minute, 30 s and 10 s; then "ban süresi doldu", the ban is lost (a timer chip in the draft strip) and the next turn is asked. |
| 10b | On a pick, wait 2 minutes; then let the other team wait 2 minutes too. | The pick passes to the other team; when it runs out again the match goes to manual. Taking the missed turn back (undo on its chip, in manual mode) gives the team its turn again. |
| 11 | On an FM map, one player without NoFail readies. | The bot names them and asks for NF before starting. |
| 11a | On an FM map, a team where nobody picks HD, HR, EZ or FL readies. | The bot names the team and asks for a mod. Still none when the time and the 30-second grace run out: the match stops, goes to manual and the referees are pinged. |
| 12 | `-abort` within 30 s; then again after 30 s; then a third time. | First aborts ("1 hakkınız kaldı" *(1 left)*); the late one is refused; once 2 are used, refused. |
| 13 | A player drops in the first 30 s. | The map is aborted on their team's account. |
| 14 | Finish a map. | 3 s later the score is confirmed from osu! and posted; the next turn is asked. |
| 15 | `-rapor <text>`. | Every host and referee gets a notification with the text and a ping in the Discord referee channel. |
| 16 | `-durdur`. | The match goes to manual; staff are notified and pinged. Switching back to auto resumes at the same step. |
| 17 | Switch the match to manual on the referee page, act by hand, switch back. | The room hears both switches. The bot does nothing while manual, then resumes from the current state. |
| 17a | In manual mode, start a pool map yourself, then switch to auto while it plays. | The bot follows the map (no countdown, no other map opened) and goes on from the next step once it finishes. |
| 17b | Stop the bot during a team's pick, put a pool map on by clicking it, switch back. | The map counts as that team's pick, and it's played. |
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
- Players who join are moved to their slot and team colour automatically. Once all four are in, the bot checks everyone's place again and fixes anyone still out of place (players joining at the same moment can get in each other's way). Strangers are kicked.
- **The room stays locked** while the bot runs the match, so players can't change their slot or colour themselves. If it finds the room unlocked (someone's `!mp unlock`, a room added by link), it locks it again, asking every 30 seconds until Bancho confirms. A player who moves anyway is put back.

### 2. Waiting for the players (both kinds)

- **Everyone is in at match time:** the match starts.
- **Everyone is in and ready before match time:** the match starts early.
- **Someone is missing at match time:** the bot re-invites them and says "Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takım kaybetmiş sayılacak." *(It's match time. Waiting 10 minutes for the missing players; a team that doesn't come will be counted as having lost.)* In a qualifier lobby: "…, gelmeyen takımlar lobiden çıkarılacak." *(…; teams that don't come will be removed from the lobby.)*
- **10 minutes after match time:**
  - **Bracket:** a team missing anyone forfeits, and the other team wins (W / FF). Both teams' players get a site notification, and the absent team also gets a Discord DM.
  - **Qualifier:** every incomplete team is kicked and removed from the lobby. Its players get a notification and a DM, and can sign up for another lobby that hasn't started. The lobby goes on with the complete teams.
  - **Nobody is complete:** the bot stops and hands the match to the staff (see [When the bot stops by itself](#when-the-bot-stops-by-itself)).

If a referee opened the room late, the 10 minutes count from when the room was opened, not from the match time.

### 3a. Qualifier lobby

1. The bot opens the pool's maps **one by one, in pool order** (the tiebreaker is skipped).
2. For each map it sets the map and mods and says "Sıradaki map: NM1. Herkes hazır olduğunda ya da 3 dakika sonra map başlayacak." *(Next map: NM1. The map starts when everyone is ready or in 3 minutes.)* The bot counts the **3 minutes** down itself (see [Countdowns](#countdowns)).
3. The map starts with a 10-second countdown when **everyone is ready**, or when the **time is up**.
4. After the **first** map, a player who drops doesn't stop anything: the lobby goes on and their score just isn't counted.
5. When every map has been played, the lobby is concluded.

### 3b. Bracket match

**Warmups**

1. Team 1 (blue) gets the host first: its first player is made the room's host. The bot says "<takım> takımına warmup açmaları için host yetkisi verildi…" *(<team> has been given host to open a warmup…)* and gives it **3 minutes** to open a warmup, counted down in the room.
2. The team picks a map and starts it themselves. Once **all four players are ready** on it, the bot starts it for them: "Herkes hazır, warmup başlıyor." *(Everyone is ready, the warmup is starting.)*, then a 10-second countdown.
   - A map **from the pool**, or **another difficulty of a pool map's song**, is refused (not started when everyone readies), and aborted if started.
   - A map **longer than 4 minutes** (by osu!'s record) is refused, and aborted if started.
3. When the warmup finishes, or the 3 minutes run out without one, the host passes to team 2.
4. A team can type `-warmup-gec` *(skip warmup)* to skip its warmup, even before its turn.

**Roll**

1. "Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var." *(Warmups are over. Use !roll to decide the pick and ban order; you have 2 minutes.)*
2. Only each team's **first** `!roll` counts, and it must be a plain `!roll` (1–100). Anything over 100 is ignored.
3. Higher roll wins. A tie means both teams roll again. A team that doesn't roll in 2 minutes loses the roll. If neither team rolls, the bot picks one at random.
4. The roll winner is **team "1" of the stage's draft order**.

**Bans and picks**

1. The bot asks the team whose turn it is: "<takım> takımı pick sırası, seçtiğiniz mapin slotunu söyleyin. 2 dakika süreniz var, süre dolarsa pick sırası rakibe geçer." *(<team>'s turn to pick: say the slot of the map you choose. You have 2 minutes; if time runs out, the pick passes to your opponent.)*
2. Either player of that team types the slot, e.g. `hd1` (case and spaces don't matter).
3. The bot asks them to confirm: "Pick için HD1 seçtiğinizi doğrulamak için tekrar yazın." *(Type it again to confirm HD1 as your pick.)* Typing the same slot again confirms it.
4. A ban is recorded. A pick is recorded and the map opens with its mods.
5. **Time limits.** A team has **3 minutes for a ban** and **2 minutes for a pick**, counted down like every other wait. Naming a slot once without confirming doesn't stop the clock.
   - A **ban** that runs out is **lost**: no map is banned for it, and the draft goes on to the next step.
   - A **pick** that runs out **passes to the other team**, which picks in its place. The stage's order then goes on as before, and the team that missed keeps its later turns.
   - If the other team also lets the passed pick run out, the bot stops and a referee takes over.
   - A missed ban or pick shows as a timer chip in the draft strip, on the referee page and the match page. A referee can take it back (manual mode) to give the team its turn again.
6. The bot **refuses**:
   - the tiebreaker
   - a map already banned
   - a map already picked or played
   - a pick in the **same category as the team's previous pick** (a NoMod category is the exception)

   The slots are named by their category's **short name** (set in the map pool editor: NM, HD, FM…) and their place: `hd1`, `nm3`, `tb`. The category's **kind** (NoMod, modded, Freemod, Tiebreaker) decides the rules, not its name.

**Each map**

- The map and mods go on, and the bot counts **2 minutes** down.
- It starts with a 10-second countdown when **all four players are in and ready**, or when the time is up.
- **FM and TB maps:** before starting, the bot reads everyone's mods. A player without **NoFail** gets "<oyuncu>: NoFail (NF) modunu seçmediniz, lütfen NF açın." *(<player>: you haven't selected NoFail (NF), please turn NF on.)* If the time is already up, they get 30 more seconds, counted down the same way. After that the map starts anyway. A player who fails the map without NoFail still has their score counted, like any other score.
- **FM maps** (not the tiebreaker): each team needs **at least one player on HD, HR, EZ or FL**. A team without one gets "<takım>: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. Seçmezseniz maç durdurulup hakem çağrılacak." *(<team>: on an FM map at least one player of the team must pick HD, HR, EZ or FL. If you don't, the match will be stopped and a referee called.)*, with the same 30-second grace when the time is already up. If the team still has none when the time runs out, the map **doesn't start**: the bot says "…Seçilmediği için maç durduruldu, hakemler çağrıldı." *(…Since none was picked, the match was stopped and the referees were called.)*, the match goes to manual and every host and referee is notified and pinged. Switched back to auto, the bot opens the map again with its full time.
- Which maps are FM or TB comes from the category's kind, or from the map itself when it was added as **FM** or **TB** in the map pool editor.
- After each map the bot posts the score, then asks for the next ban or pick.

**Tiebreaker and the end**

- When the score reaches the tiebreaker (e.g. 4–4 in a Bo9), the bot says "Tiebreaker!" and opens the TB map.
- When a team reaches the winning number of maps, the bot announces "<takım> maçı kazandı!" *(<team> won the match!)* with the score.

### 4. After every map (both kinds)

- **3 seconds** after the map ends, the bot reads osu!'s record of the room (the mp link). osu!'s scores replace what the chat said.
- If osu! doesn't have the map yet, the bot tries again every 3 seconds, up to 5 times. After that the chat's scores stand, and the log says so.
- Every player's score counts toward their team's total, including a player who failed the map. A failed score is saved as the player's score and marked as failed.

### 5. The end (both kinds)

1. "Maç bitti, oda 30 saniye sonra kapanacak." *(The match is over; the room closes in 30 seconds.)* Ten seconds before: "Oda 10 saniye sonra kapanacak." *(The room closes in 10 seconds.)*, and "Oda kapanıyor." *(The room is closing.)* as it closes.
2. The match is concluded on the site. A bracket match uses the bot's counted score, the teams move on in the bracket, and Discord gets the results.
3. 30 seconds later (plus the 10-second grace, see [Countdowns](#countdowns)) the room is closed.

---

## Player commands

Typed in the **match room**, unless the table says otherwise.

| Command | Meaning | What it does | Limits | Works in manual mode |
|---|---|---|---|---|
| `-davet` | *invite* | **In a DM to the room's bot:** sends a new room invite. | Once every 20 s | Yes |
| `-abort` | *abort* | Aborts the map being played. The bot says "Abort kullanıldı. N hakkınız kaldı." *(Abort used. You have N left.)* | **2 per team** per match, and only in the **first 30 seconds** of a map. Later: "Maç başladıktan 30 saniye sonra abort kullanılamaz" *(Abort can't be used 30 seconds after the map has started)* | No |
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
| Ban | 3 min; lost when it runs out |
| Pick | 2 min; passes to the other team when it runs out |
| Map ready time | 3 min (qualifier), 2 min (bracket) |
| NoFail / FM mod grace | 30 s extra, once |
| Room lock | asked again every 30 s while Bancho says it's unlocked |
| Start countdown | 10 s |
| `-abort` | 2 per team, within 30 s of the map starting |
| Score read from osu! | 3 s after the map, up to 5 tries |
| Room closes | 30 s after the match is over |
| Grace after every countdown | 10 s (the bot acts 10 s after the time it announced) |

All of these waits are counted by the bot itself; it doesn't use Bancho's `!mp timer`. Only the 10-second start countdown is Bancho's (`!mp start 10`).

### Countdowns

Every wait the room is told about (the no-show wait, a team's warmup turn, the roll, a team's ban or pick, a map's ready time and the NoFail grace, the room closing) is counted down by the bot in the chat:

- When the wait starts, the bot says how long it is.
- Then a reminder at **every whole minute** left, at **30 seconds** and at **10 seconds**.
- When it's over, a line saying so ("Süre doldu…", the forfeit, the roll's result, "Oda kapanıyor.").

What the bot says is always true from the moment the players see it. When a reminder goes out late because the bot's message queue was busy, the end moves back by as long as it was late: "30 saniye kaldı" sent 6 seconds late still leaves 30 seconds.

**Grace:** the bot acts **10 seconds after** the end it announced, so a slow message or a slow client doesn't cost anyone the step. After "10 saniye kaldı", there are really 20 seconds.

---

## When the bot stops by itself

When the bot can't continue safely, it **switches the match to manual**. It says "Otomatik hakem durdu, maçı bir hakem devralacak." *(The auto referee stopped; a referee will take over the match.)* in the room and notifies every host and referee: a site notification, plus a ping in the Discord referee channel with the reason.

| Reason, as the bot states it | In English | What to do |
|---|---|---|
| "10 dakika beklendi ama hiçbir takım tam kadro gelmedi." | *Waited 10 minutes, but no team arrived with a full roster.* | Decide with the hosts: forfeit, reschedule, or wait longer. |
| "Stage'in best of değeri yok, maçın ne zaman bittiği bilinmiyor." | *The stage has no best of, so it's unknown when the match ends.* | Set the stage's best of on `/admin/stages`, then switch back to auto. |
| "Stage'in pick sırası bitti ama maç bitmedi." | *The stage's pick order ran out, but the match isn't over.* | The draft order has no closing `*`. Fix the order, or finish the match by hand. |
| "Pick süresi iki takımda da doldu: ilk takımın pick'i rakibe geçti, o da seçmedi." | *Both teams let the pick run out: the first team's pick passed to its opponent, which didn't pick either.* | Ping the teams; mark the pick by hand in manual mode, or take a missed pick back (undo on its timer chip) to give a team the turn again, then switch back to auto. |
| "Sıradaki ban/pick'in takımı belirlenemedi." | *Couldn't determine which team has the next ban/pick.* | Choose the team under "Sıradaki pick" *(Next pick)* in manual mode, then switch back to auto. |
| "Skor (a - b) BoN'u aşıyor." | *The score (a - b) exceeds BoN.* | The score is wrong. Fix it ("Sayma" *(don't count)*, a referee's award, or "Maç linkinden eşitle" *(sync from the match link)*). |
| "Skor tiebreaker'a geldi ama havuzda tiebreaker yok." | *The score reached the tiebreaker, but the pool has no tiebreaker.* | Add a TB map, or open a map by hand. |
| "Tiebreaker oynandı ama kazanan belli değil." | *The tiebreaker was played, but there's no winner.* | Check the TB's result and fix the score. |
| "Maçın iki takımı belli değil." | *The match's two teams aren't known.* | Set both teams on the bracket. |
| "FM mapinde mod seçilmedi (<takım>): takımdan en az bir oyuncu HD, HR, EZ ya da FL seçmeli." | *No mod picked on the FM map (<team>): at least one player of the team must pick HD, HR, EZ or FL.* | Talk to the team; once someone has a mod, switch back to auto (the map opens again) or start it by hand. |

A player's `-durdur` also puts the match in manual (see above).

---

## Control surfaces (staff)

### Referee page (`/admin/referee/<match>`)

**Room bar.** The "Otomatik hakem" *(Auto referee)* switch flips between auto and manual, and the room hears it either way: "Maç manuel moda alındı…" *(The match was put in manual mode…)* or "Otomatik hakem tekrar devrede…" *(The auto referee is back on…)*. Switching it back on continues from where the match stands, not only from where the bot stopped: the bot restarts that step's timer and catches up with anything a referee did meanwhile.
- A pool map **being played** is followed: no countdown, no other map; the bot goes on once it finishes.
- A match the referee **took past its start** (warmups, the roll, a ban, a pick or a map done by hand) goes on from its next step; the bot doesn't start warmups over.
- A pick **marked by hand** that hasn't been played yet is opened before the next turn.
- A pool map the referee **put on** (clicked) during a team's pick, without marking it, counts as that team's pick, unless the team couldn't pick it.

The same catching up happens when the room's bot comes back after a disconnect, and after a restart.

**Status line** (under the room bar). One line shows:
- what the bot is waiting for, with a countdown: match time, the no-show cutoff, the warmup, the roll, the ban or pick, the map timer
- whose ban or pick it is
- each team's remaining `abort` uses (crossed out when spent)
- before the room exists: when the bot will open it, or that automatic room opening is off
- in manual mode: "Manuel kontrol" *(Manual control)*, and what the bot will continue with

**Commands while in auto.** Controls that referee the match stay visible but are **disabled**. A line at the top says "Otomatik hakem maçı yönetiyor" *(The auto referee is running the match)*, with a "Manuel kontrole geç" *(Switch to manual control)* button.

| Locked in auto mode | Still open in auto mode |
|---|---|
| Room commands: "Başlat" *(Start)*, "İptal et" *(Abort)*, "Zamanlayıcı" *(Timer)*, "Durdur" *(Stop timer)*, "Odayı yenile" *(Refresh room)*, "Hepsini davet et" *(Invite all)*, "Yerleri düzelt" *(Fix slots)*, "Kilitle" *(Lock)*, "Kurulumu tekrarla" *(Redo setup)* | The "Otomatik hakem" *(Auto referee)* switch |
| Clicking pool maps, ban/pick buttons and taking them back (missed ones too) | Chat (plain messages; `!mp` lines wait for manual mode) |
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

**The bot is waiting for a pick but nobody answers.** Picks have a 2-minute timer, bans a 3-minute one. When a pick runs out it passes to the other team; when a ban runs out it's lost. If you'd rather wait for the team (a disconnect, say), go to manual and mark the pick yourself, or switch back to auto later: the timer starts over.

**A team's ban or pick ran out but they deserve it back.** Switch to manual and click the undo button on the timer chip in the draft strip. The team has the turn again, and switching back to auto asks them with a fresh timer.

**I started the match by hand and only then switched to auto.** Fine: the bot reads where the match stands. A map being played is followed to its end, a match past its warmups or roll goes on from its next ban or pick, and a lobby goes on with its next unplayed map.

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
| Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takım kaybetmiş sayılacak. | It's match time. Waiting 10 minutes for the missing players; a team that doesn't come will be counted as having lost. | Match time, someone missing (bracket) |
| Maç saati geldi. Eksik oyuncular için 10 dakika bekleniyor, gelmeyen takımlar lobiden çıkarılacak. | It's match time. Waiting 10 minutes for the missing players; teams that don't come will be removed from the lobby. | Match time, someone missing (qualifier) |
| <takım> takımı 10 dakika içinde tam kadro gelmedi. Maç hükmen <takım> takımına verildi. | <team> didn't arrive with a full roster within 10 minutes. The match is awarded to <team> by forfeit. | Bracket no-show |
| <takımlar> tam kadro gelmediği için lobiden çıkarıldı. | <teams> were removed from the lobby for not arriving with a full roster. | Qualifier no-show |
| Herkes burada, maç başlıyor! Mapler sırayla açılacak, iyi şanslar. | Everyone's here, the match is starting! The maps will open in order, good luck. | Qualifier starts |

**Warmups**

| Turkish | English | When |
|---|---|---|
| <takım> takımına warmup açmaları için host yetkisi verildi, 3 dakika süreleri var. Warmup mapleri 4 dakikayı geçmemelidir. Eğer warmup oynamak istemiyorlarsa -warmup-gec yazabilirler. | <team> has been given host to open a warmup; they have 3 minutes. Warmup maps must not exceed 4 minutes. If they don't want to play a warmup, they can type -warmup-gec. | A team's warmup turn |
| <takım> warmupı geçti. | <team> skipped their warmup. | `-warmup-gec` on their turn |
| <takım> sırası gelince warmupı geçecek. | <team> will skip their warmup when their turn comes. | `-warmup-gec` before their turn |
| <takım> takımının warmup süresi doldu. | <team>'s warmup time ran out. | 3 minutes without a warmup |
| Havuzdaki mapler (ve diğer zorlukları) warmup olarak oynanamaz, başka bir map seçin. | Pool maps (and their other difficulties) can't be played as a warmup; choose another map. | A pool song is chosen, readied on or started |
| Bu map M:SS uzunluğunda, warmup 4 dakikayı geçmemeli. Başka bir map seçin. | This map is M:SS long; a warmup must not exceed 4 minutes. Choose another map. | A long map is chosen, readied on or started (and aborted) |
| Herkes hazır, warmup başlıyor. | Everyone is ready, the warmup is starting. | All four ready on an allowed warmup |
| Warmup 4 dakikayı geçti, sonlandırıldı. | The warmup went past 4 minutes and was ended. | A warmup runs too long |

**Roll**

| Turkish | English | When |
|---|---|---|
| Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var. | Warmups are over. Use !roll to decide the pick and ban order; you have 2 minutes. | Roll starts |
| Pick ve ban sırasını belirlemek için !roll komutunu kullanın, 2 dakika süreniz var. | Use !roll to decide the pick and ban order; you have 2 minutes. | Roll resumes after manual mode |
| <oyuncu>: sadece !roll yazın (1-100), bu roll sayılmadı. | <player>: type just !roll (1-100); this roll didn't count. | A roll over 100 |
| Berabere (a - b)! İki takım da tekrar !roll yazsın, 2 dakika süreniz var. | A tie (a - b)! Both teams roll again; you have 2 minutes. | Equal rolls |
| <takım> rollü kazandı (a - b). | <team> won the roll (a - b). | Both rolled |
| <takım> rollü kazandı (rakibi roll atmadı). | <team> won the roll (their opponent didn't roll). | One team didn't roll |
| <takım> rollü kazandı (iki takım da roll atmadı, bot rastgele seçti). | <team> won the roll (neither team rolled; the bot chose at random). | Neither team rolled |

**Bans and picks**

| Turkish | English | When |
|---|---|---|
| <takım> takımı pick sırası, seçtiğiniz mapin slotunu söyleyin. 2 dakika süreniz var, süre dolarsa pick sırası rakibe geçer. | <team>'s turn to pick: say the slot of the map you choose. You have 2 minutes; if time runs out, the pick passes to your opponent. | Asking for a pick |
| <takım> takımı pick sırası, seçtiğiniz mapin slotunu söyleyin. 2 dakika süreniz var. | <team>'s turn to pick: say the slot of the map you choose. You have 2 minutes. | Asking for a pick passed from the other team |
| <takım> takımı ban sırası, seçtiğiniz mapin slotunu söyleyin. 3 dakika süreniz var, süre dolarsa ban hakkınız yanar. | <team>'s turn to ban: say the slot of the map you choose. You have 3 minutes; if time runs out, you lose the ban. | Asking for a ban |
| Pick için <slot> seçtiğinizi doğrulamak için tekrar yazın. | Type it again to confirm <slot> as your pick. | First answer |
| Ban için <slot> seçtiğinizi doğrulamak için tekrar yazın. | Type it again to confirm <slot> as your ban. | First answer |
| <takım> takımı <slot> mapini seçti. | <team> picked <slot>. | Pick confirmed |
| <takım> takımı <slot> mapini banladı. | <team> banned <slot>. | Ban confirmed |
| <slot> tiebreaker, picklenemez. / banlanamaz. | <slot> is the tiebreaker; it can't be picked. / banned. | TB named |
| <slot> banlandı, başka bir map seçin. | <slot> is banned; choose another map. | A banned map named |
| <slot> zaten oynandı, başka bir map seçin. | <slot> has already been played; choose another map. | A picked or played map named |
| Aynı moddan (<mod>) arka arkaya pick yapamazsınız (NM hariç), başka bir mod seçin. | You can't pick the same mod (<mod>) twice in a row (except NM); choose another mod. | Same category twice |
| <takım> takımının ban süresi doldu, ban hakkı yandı. | <team>'s time to ban ran out; the ban is lost. | Ban time over |
| <takım> takımının pick süresi doldu, pick sırası <takım> takımına geçti. | <team>'s time to pick ran out; the pick passes to <team>. | Pick time over |
| <takım> takımının pick süresi doldu. | <team>'s time to pick ran out. | A passed pick's time over (then the bot stops) |
| <takım> takımının pick'i hakemin açtığı <slot> olarak sayıldı. | <team>'s pick was counted as <slot>, which the referee opened. | Back to auto during a pick, with a pool map the referee put on |

**Maps**

| Turkish | English | When |
|---|---|---|
| Sıradaki map: <slot>. Herkes hazır olduğunda ya da N dakika sonra map başlayacak. | Next map: <slot>. The map starts when everyone is ready or in N minutes. | A map opens |
| <oyuncular>: NoFail (NF) modunu seçmediniz, lütfen NF açın. | <players>: you haven't selected NoFail (NF), please turn NF on. | FM/TB, NF missing |
| Süre doldu, map NoFail'siz oyuncularla başlıyor. | Time's up; the map starts with players who don't have NoFail. | NF still missing after the grace |
| <takımlar>: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. Seçmezseniz maç durdurulup hakem çağrılacak. | <teams>: on an FM map at least one player of the team must pick HD, HR, EZ or FL. If you don't, the match will be stopped and a referee called. | FM, a team without a mod |
| <takımlar>: FM mapinde takımdan en az bir oyuncu HD, HR, EZ veya FL seçmeli. Seçilmediği için maç durduruldu, hakemler çağrıldı. | <teams>: on an FM map at least one player of the team must pick HD, HR, EZ or FL. Since none was picked, the match was stopped and the referees were called. | FM mod still missing when time runs out (then the bot stops) |
| Abort kullanıldı. N hakkınız kaldı. | Abort used. You have N left. | `-abort` accepted |
| <oyuncu> oyundan düştü. Abort kullanıldı. N hakkınız kaldı. | <player> disconnected. Abort used. You have N left. | Early disconnect |
| Maç başladıktan 30 saniye sonra abort kullanılamaz. | Abort can't be used 30 seconds after the map has started. | Late `-abort` |
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
| Oda 10 saniye sonra kapanacak. | The room closes in 10 seconds. | 10 s before the room closes (also after a referee concludes) |
| Oda kapanıyor. | The room is closing. | As the room closes |

**Countdowns** (see [Countdowns](#countdowns); `N` is the time left: "2 dakika", "1 dakika", "30 saniye", "10 saniye")

| Turkish | English | When |
|---|---|---|
| Eksik oyuncular için N kaldı. | N left for the missing players. | No-show wait |
| <takım>: warmup açmak için N kaldı. | <team>: N left to open a warmup. | Warmup turn |
| !roll için N kaldı. | N left to !roll. | Roll |
| !roll süresi doldu. | Time's up for !roll. | Roll over (then the result) |
| <takım>: ban için N kaldı. / pick için N kaldı. | <team>: N left to ban. / to pick. | Ban or pick turn |
| Map N sonra başlayacak. | The map starts in N. | Map ready time, NoFail grace |
| Süre doldu, map başlıyor. | Time's up; the map is starting. | Map ready time over |

**Player commands and stops**

| Turkish | English | When |
|---|---|---|
| <oyuncu>: raporun hostlara ve hakemlere iletildi. | <player>: your report was passed on to the hosts and referees. | `-rapor` accepted |
| <oyuncu>: kullanım: -rapor <mesajınız> | <player>: usage: -rapor <your message> | `-rapor` with no text |
| <oyuncu>: bir dakikada bir rapor gönderebilirsiniz. | <player>: you can send one report per minute. | `-rapor` too often |
| Durdur komutu kullanıldı. Bu komut sadece acil durumlarda kullanılmalıdır. Hostlar bilgilendirildi, eğer bu komut kötü niyetle kullanıldıysa -rapor komutuyla bildiriniz. | The stop command was used. This command should only be used in emergencies. The hosts have been informed; if it was used in bad faith, report it with the -rapor command. | `-durdur` |
| Otomatik hakem durdu, maçı bir hakem devralacak. | The auto referee stopped; a referee will take over the match. | The bot hands over |
| Maç manuel moda alındı: bundan sonra maçı hakem yönetiyor, otomatik hakem durdu. | The match was put in manual mode: a referee runs the match from now on; the auto referee stopped. | A referee switches the match to manual |
| Otomatik hakem tekrar devrede: maç otomatik modda, bot kaldığı yerden devam ediyor. | The auto referee is back on: the match is in auto mode, and the bot continues from where it stands. | A referee switches the match back to auto |

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
| Ban: <takım>, süre doldu, ban hakkı yandı / Pick: <takım>, süre doldu, pick rakibe geçti | Ban: <team>, time ran out, the ban is lost / Pick: <team>, time ran out, the pick passed to the opponent (a missed turn's chip in the draft strip) |
| Geri al: ban / pick yeniden <takım> sırası olsun | Take back: make the ban / pick <team>'s turn again (the undo button on that chip) |
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
| bilgi | Herkes hazır, warmup başlatıldı. | Everyone was ready; the warmup was started. |
| uyarı | Odanın kilidi açılmış, tekrar kilitleniyor. | The room had been unlocked; locking it again. |
| bilgi | Roll bekleniyor. | Waiting for the roll. |
| bilgi | <oyuncu> (<takım>) N attı. | <player> (<team>) rolled N. |
| bilgi | Roll: <takım> kazandı (…). | Roll: <team> won (…). |
| bilgi | <takım> takımının pick/ban seçimi bekleniyor. | Waiting for <team>'s pick/ban. |
| bilgi | <takım> <slot> seçti / banladı (<oyuncu> onayladı). | <team> picked / banned <slot> (confirmed by <player>). |
| bilgi | <takım> ban süresini kaçırdı, ban hakkı yandı. | <team> missed its ban time; the ban is lost. |
| bilgi | <takım> pick süresini kaçırdı, pick <takım> takımına geçti. | <team> missed its pick time; the pick passed to <team>. |
| bilgi | Ban/pick sırası değişti, yeniden soruluyor. | The ban/pick turn changed; asking again. |
| bilgi | <slot> açıldı. | <slot> opened. |
| bilgi | NF uyarısı: <oyuncular> | NF warning: <players> |
| uyarı | NF açmadan başladı: <oyuncular> | Started without NF: <players> |
| bilgi | FM mod uyarısı: <takımlar> | FM mod warning: <teams> |
| uyarı | FM modu seçilmedi, maç durduruldu: <takımlar> | No FM mod picked, the match was stopped: <teams> |
| uyarı | <slot> bot dışında başlatıldı, bot onu takip ediyor. | <slot> was started by someone else; the bot follows it. |
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
| bilgi | Devam ediyor: <slot> oynanıyor, bitince sıradaki adıma geçilecek. | Continuing: <slot> is being played; the next step comes once it finishes. |
| bilgi | Devam ediyor: maç bot olmadan başlamış (ban, pick, roll ya da oynanan map var), sıradaki adımdan devam ediliyor. | Continuing: the match started without the bot (there's a ban, a pick, a roll or a played map); going on from the next step. |
| bilgi | Devam ediyor: bir warmup oynanıyor, bitince sıradaki warmup. | Continuing: a warmup is being played; the next warmup comes once it finishes. |
| bilgi | Hakem <slot> mapini açmıştı: <takım> takımının pick'i olarak işaretlendi. | The referee had opened <slot>: marked as <team>'s pick. |
| bilgi | <slot> seçildi ama henüz oynanmadı, açılıyor. | <slot> was picked but hasn't been played yet; opening it. |
| hata | Manuel kontrole geçildi: <neden> | Switched to manual control: <reason> |
| hata | İşlem başarısız: <hata> | Action failed: <error> |
