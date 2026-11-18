# Rank limit and seeding


---

## The rank limit

Teams are two players. The limit doesn't cap each player's rank on its own. Each player's osu!standard **global rank** is turned into a **player value**, and the two players' values together, the **team value**, must not be above the limit:

```
player value = 26911 − 1890 · ln(rank + K) − 1890 · K / (rank + K)
team value   = player value (A) + player value (B)

K     = 175
limit = 19000        (a team may register while team value ≤ 19000)
```

I would like to mention that I used this exact formula with different values 5 years ago in my last 2v2 tournament. It turned out great, especially considering the bigger skill gap back then. With a smaller gap together this time. I feel like it's going to stay competetive.

### Where the formula comes from

The shape of the formula comes from a curve fitted to the global ranks and pp of Turkish players ([the fitted player curve](https://docs.google.com/spreadsheets/d/1yh7BeUyXR9o5S24P6TZvjDfKyDQSm4DFlNbT3f8wJis/edit?gid=1508715718#gid=1508715718)). For a given rank, the player value is close to the pp a Turkish player at that rank has. So the team value works like the pair's combined pp, but it is computed from the rank.
The two constants were then set by discussion, not by the fit alone:

- **K** shifts every rank before the curve is applied. It decides how much the top ranks are worth compared to the rest.
- **The limit** decides how strong a pair may be.


The calculation is in `backend/src/team_rank.rs`. It reproduces the Desmos sheets:

- [2026 Oyuncu kayıt formül](https://www.desmos.com/calculator/zaamotjkbr): the formula.
- [Oyuncu rank hesap ters](https://www.desmos.com/calculator/4uhkwb5pzs): its inverse, the best rank a partner may have.

### What it means in practice

| Player's rank | Player value | Best rank their teammate may have |
|---|---|---|
| #1 | 15 260 | #210 634 |
| #100 | 15 093 | #192 797 |
| #1 000 | 13 269 | #73 249 |
| #5 000 | 10 685 | #18 399 |
| #9 667 | 9 500 | #9 667 (two equal players at the limit) |
| #10 000 | 9 438 | #9 345 |
| #50 000 | 6 448 | #1 635 |
| #100 000 | 5 145 | #630 |
| #210 634 and beyond | ≤ 3 740 | anyone, #1 included |

- **No rank is excluded.** Every player has possible partners: the #1 player can team up with anyone ranked about #210 000 or beyond. From about #210 000 on, a player can team up with anyone. This is basically a slightly improved version of "Total PP/Rank Limit" to allow more 
- The curve flattens towards the top, so the difference between #1 and #100 is small (167), and the difference between #1 000 and #5 000 is large (2 584). This matches how pp is spread across ranks.

### How it is applied

- The limit is checked when a player sends an invite, and checked again when the other player accepts. The site uses the players' current ranks from osu! (refreshed if older than 10 minutes). The team page shows each player the best rank their teammate may have.
- A player without a global rank can't register.
- Ranks are refreshed daily until registration closes; closing registration freezes them. A team that goes over the limit because a player's rank improved after registering is flagged to the players and the hosts. It isn't removed automatically: the committee decides.

---

## Seeding

The bracket is seeded from the qualifier with the **Z-percentile** method described on the osu! wiki: [Common seeding methods](https://osu.ppy.sh/wiki/en/Tournaments/Common_seeding_methods).

In short:
1. On every qualifier map, a team's score (its two players' scores added up) is compared to every team that played that map. The comparison gives a Z-score.
2. The Z-score is turned into a percentile between 0 and 1.
3. A team's seeding points are the sum of its percentiles over the pool, and the highest total is seed 1.

If a team plays a map more than once, its last attempt counts. A map a team didn't play counts as −1.
