# RUN154 — Social Sentiment Contrarian: CryptoCompare Social Stats as Top/Bottom Signal

## Hypothesis

**Mechanism**: When a coin's social media mention volume spikes to extreme levels (Twitter/X, Reddit, Telegram combined), it often coincides with local price tops — the "buy the rumor, sell the news" dynamic. Conversely, social silence during price drops can signal local bottoms. COINCLAW has no sentiment data. Adding CryptoCompare's free social stats API provides a contrarian signal that predicts mean-reversion after social peaks.

**API to use**:
- CryptoCompare: `GET https://min-api.cryptocompare.com/data/social/stats?coinId={COIN_ID}`
  - Response: `{"Data": {"twitter": {"followers": X, "posts": Y}, "reddit": {"subscribers": Z}, "CoinId": N}}`
  - Fields: `twitter_posts`, `reddit_comments`, `social_score`
  - Update: hourly
  - Free tier: ~50-100 calls/day
  - Latency: ~500ms

**Note**: CryptoCompare requires an API key for reliable access. Free tier has rate limits. If CryptoCompare is too limited, use CoinGecko's trending coins endpoint as a proxy: `GET https://api.coingecko.com/api/v3/search/trending` — trending coins get a social boost.

**Sentiment spike calculation**: Compare current social volume to 7-day rolling average. If current > avg × SOCIAL_SPIKE_MULT, it's a spike. Direction: price up during spike → SHORT (social peak = local top); price down during spike → LONG (social silence = local bottom).

**Why this is not a duplicate**: No prior RUN uses social media or sentiment data. All signals are price-action/derivatives-based. Social sentiment is a fundamentally different data class that captures retail crowd behavior.

## Proposed Config Changes (config.rs)

```rust
// ── RUN154: Social Sentiment Contrarian ───────────────────────────────
// Social spike = current social_volume > rolling_avg * SOCIAL_SPIKE_MULT
// Entry: price UP during social spike → SHORT (social peak = local top)
// Entry: price DOWN during social silence → LONG (social bottom = local bottom)
// Exit: social volume normalization OR MAX_HOLD bars

pub const SOCIAL_ENABLED: bool = true;
pub const SOCIAL_SPIKE_MULT: f64 = 2.0;     // social vol must be 2x the 7-day rolling average
pub const SOCIAL_WINDOW_DAYS: usize = 7;      // rolling window for social volume avg
pub const SOCIAL_PRICE_CONFIRM: f64 = 0.005; // 0.5% price move required to confirm direction
pub const SOCIAL_SL: f64 = 0.005;           // 0.5% stop (social moves can be volatile)
pub const SOCIAL_TP: f64 = 0.004;           // 0.4% take profit
pub const SOCIAL_MAX_HOLD: u32 = 24;          // ~6 hours at 15m bars (sentiment fades slowly)
pub const SOCIAL_FETCH_MINUTES: u64 = 60;     // social data updates hourly
```

Add to `coinclaw/src/fetcher.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SocialStats {
    #[serde(alias = "twitter")]
    pub twitter: Option<TwitterStats>,
    #[serde(alias = "reddit")]
    pub reddit: Option<RedditStats>,
    #[serde(alias = "CoinId")]
    pub coin_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TwitterStats {
    #[serde(alias = "posts")]
    pub posts: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RedditStats {
    #[serde(alias = "comments")]
    pub comments: Option<u64>,
}

/// Compute social volume score = twitter_posts + reddit_comments
fn compute_social_score(stats: &SocialStats) -> u64 {
    let twitter = stats.twitter.as_ref().and_then(|t| t.posts).unwrap_or(0);
    let reddit = stats.reddit.as_ref().and_then(|r| r.comments).unwrap_or(0);
    twitter.saturating_add(reddit)
}

/// Fetch social stats for a coin from CryptoCompare.
/// Note: requires API key for production use. Free tier limited to ~50 calls/day.
/// Fallback: CoinGecko trending endpoint for social proxy.
pub async fn fetch_social_stats(coin_id: &str) -> Option<u64> {
    let url = format!(
        "https://min-api.cryptocompare.com/data/social/stats?coinId={}",
        coin_id
    );
    let resp = reqwest::get(&url).await.ok()?;
    let stats: SocialStats = resp.json().await.ok()?;
    Some(compute_social_score(&stats))
}

/// Fallback: CoinGecko trending as social proxy (no API key needed, rate limited)
pub async fn fetch_trending_score(coin_id: &str) -> Option<bool> {
    // Returns true if coin appears in trending list
    // Used as a binary signal rather than continuous score
    let url = "https://api.coingecko.com/api/v3/search/trending";
    let resp = reqwest::get(url).await.ok()?;
    let trending: serde_json::Value = resp.json().await.ok()?;
    // Check if coin_id appears in trending list (simplified)
    let trending_str = trending.to_string();
    Some(trending_str.contains(coin_id))
}
```

Add to `CoinState` in `state.rs`:

```rust
pub social_score: u64,
pub social_rolling_avg: f64,
pub social_rolling_history: Vec<u64>,  // 7-day rolling window
pub social_spike_bars: u32,          // bars since social spike detected
pub social_spike_direction: Option<Direction>,
```

Add entry logic in `engine.rs`:

```rust
/// Fires when social spike detected and price confirms direction
fn check_social_entry(state: &mut SharedState, ci: usize) -> Option<(Direction, &'static str)> {
    let cs = &state.coins[ci];
    if cs.pos.is_some() { return None; }
    if cs.cooldown > 0 { return None; }
    if !config::SOCIAL_ENABLED { return None; }

    if cs.social_spike_bars == 0 { return None; }  // no active spike

    let ind = cs.ind_15m.as_ref()?;
    if !ind.valid { return None; }

    let price_move_pct = match cs.social_spike_direction {
        Some(Direction::Long) => {
            // Price dropped during social spike → social silence = local bottom → LONG
            let drop = (cs.social_entry_price - ind.p) / cs.social_entry_price;
            if drop >= config::SOCIAL_PRICE_CONFIRM { Direction::Long } else { return None; }
        }
        Some(Direction::Short) => {
            // Price rose during social spike → social peak = local top → SHORT
            let rise = (ind.p - cs.social_entry_price) / cs.social_entry_price;
            if rise >= config::SOCIAL_PRICE_CONFIRM { Direction::Short } else { return None; }
        }
        None => return None,
    };

    Some((price_move_pct, "social_sentiment"))
}
```

Note: Social data updates hourly — integrate into fetch cycle with `SOCIAL_FETCH_MINUTES` interval. Track rolling 7-day average. Set `social_spike_bars = 1` when `current > avg * SPIKE_MULT`.

---

## Expected Outcome

**Validation**: Backtest on 18 coins, 1-year daily data (social data is daily/hourly granularity).
- **Baseline**: COINCLAW v16 (no social signal)
- **Comparison**: social sentiment trades tracked separately

**Metrics to measure**:
- Social sentiment WR (hypothesis: >55%)
- PF on social trades
- Social spike frequency vs price outcome correlation
- Lag: how long after social spike does reversal occur?

**Hypothesis**: Social volume spikes (2x+ 7-day average) accompanied by price movement in the expected direction predict mean-reversion with WR >55% within 6-24 bars.

---

## Validation Method

1. **Historical backtest** (run154_1_social_backtest.py):
   - 18 coins, 1-year daily or hourly data
   - Use CryptoCompare historical social data or web scraping
   - Identify all social volume spikes with price confirmation
   - Record: social score, price direction, entry price, stop, TP, exit, P&L
   - Output: per-coin WR, PF, avg hold time

2. **Walk-forward** (run154_2_social_wf.py):
   - 3-window walk-forward (train 4mo, test 2mo)
   - Sweep SOCIAL_SPIKE_MULT: 1.5 / 2.0 / 3.0
   - Sweep SOCIAL_PRICE_CONFIRM: 0.003 / 0.005 / 0.008

3. **Combined comparison** (run154_3_combined.py):
   - Side-by-side: COINCLAW v16 vs COINCLAW v16 + social_sentiment
   - Portfolio stats, correlation with regime trades

---

## Out-of-Sample Testing

- SPIKE_MULT sweep: 1.5 / 2.0 / 3.0 / 4.0
- PRICE_CONFIRM sweep: 0.003 / 0.005 / 0.008 / 0.010
- OOS: final 4 months held out from all parameter selection
